use crate::modules::share::models::{
    CreateMarketSourceRequest, MarketScriptAggregated, MarketScriptItem, MarketSourceInfo,
};
use futures::StreamExt;
use niupanel_common::error::{AppError, Result};
use niupanel_common::{info, warn};
use niupanel_entity::market_sources;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};

pub async fn list_market_sources(db: &DatabaseConnection) -> Result<Vec<market_sources::Model>> {
    market_sources::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn add_market_source(
    db: &DatabaseConnection,
    client: &reqwest::Client,
    req: CreateMarketSourceRequest,
) -> Result<market_sources::Model> {
    let _info = fetch_market_info(client, &req.url).await?;

    let source = market_sources::ActiveModel {
        name: Set(req.name),
        url: Set(req.url),
        description: Set(req.description),
        enabled: Set(true),
        created_at: Set(chrono::Utc::now().into()),
        last_synced_at: Set(Some(chrono::Utc::now().into())),
        ..Default::default()
    };
    source.insert(db).await.map_err(AppError::Database)
}

pub async fn delete_market_source(db: &DatabaseConnection, id: i32) -> Result<u64> {
    let res = market_sources::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(res.rows_affected)
}

pub async fn sync_market_source(
    db: &DatabaseConnection,
    client: &reqwest::Client,
    id: i32,
) -> Result<()> {
    let source = market_sources::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("订阅源不存在".to_string()))?;

    let info = fetch_market_info(client, &source.url).await?;

    let mut active = source.into_active_model();
    active.name = Set(info.name);
    active.description = Set(info.description);
    active.icon = Set(info.icon);
    active.last_synced_at = Set(Some(chrono::Utc::now().into()));
    active.update(db).await.map_err(AppError::Database)?;

    Ok(())
}

async fn fetch_market_info(client: &reqwest::Client, url: &str) -> Result<MarketSourceInfo> {
    let mut target_url = url.to_string();

    let is_github =
        url.starts_with("https://github.com/") && !url.contains("raw.githubusercontent.com");
    if is_github && !url.ends_with(".json") {
        let base = url.trim_end_matches(".git").trim_end_matches("/");
        target_url = base.replace("github.com", "raw.githubusercontent.com") + "/main/market.json";
    }

    let res_result = client
        .get(&target_url)
        .header("User-Agent", "NiuPanel-Server")
        .send()
        .await;

    if let Ok(res) = res_result {
        if res.status().is_success() {
            if let Ok(info) = res.json::<MarketSourceInfo>().await {
                return Ok(info);
            }
        } else if res.status() == reqwest::StatusCode::NOT_FOUND
            && target_url.contains("/main/market.json")
        {
            let alt_url = target_url.replace("/main/market.json", "/master/market.json");
            if let Ok(alt_res) = client
                .get(&alt_url)
                .header("User-Agent", "NiuPanel-Server")
                .send()
                .await
            {
                if alt_res.status().is_success() {
                    if let Ok(info) = alt_res.json::<MarketSourceInfo>().await {
                        return Ok(info);
                    }
                }
            }
        }
    }

    if is_github {
        info!(
            "未在 GitHub 仓库中找到 market.json，启动自动识别模式: {}",
            url
        );
        return discover_github_scripts(client, url).await;
    }

    Err(AppError::ExternalApi(
        "获取订阅源失败: 无法定位到有效的市场索引文件".to_string(),
    ))
}

async fn discover_github_scripts(
    client: &reqwest::Client,
    repo_url: &str,
) -> Result<MarketSourceInfo> {
    let parts: Vec<&str> = repo_url.trim_end_matches("/").split('/').collect();
    if parts.len() < 5 {
        return Err(AppError::Generic("无效的 GitHub 仓库地址".to_string()));
    }
    let owner = parts[3];
    let repo = parts[4].trim_end_matches(".git");

    let api_url = format!("https://api.github.com/repos/{}/{}/contents", owner, repo);
    let res = client
        .get(&api_url)
        .header("User-Agent", "NiuPanel-Server")
        .send()
        .await
        .map_err(|e| AppError::ExternalApi(format!("GitHub API 请求失败: {}", e)))?;

    if !res.status().is_success() {
        return Err(AppError::ExternalApi(format!(
            "无法访问仓库内容 ({}): {}",
            res.status(),
            api_url
        )));
    }

    let items = res
        .json::<Vec<serde_json::Value>>()
        .await
        .map_err(|e| AppError::Serialization(format!("解析 GitHub 响应失败: {}", e)))?;

    let mut scripts = Vec::new();
    let whitelist_ext = ["py", "js", "ts", "sh"];

    for item in items {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let download_url = item
            .get("download_url")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let type_str = item
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if type_str == "file" && !download_url.is_empty() {
            let ext = name.split('.').last().unwrap_or_default();
            if whitelist_ext.contains(&ext) {
                scripts.push(MarketScriptItem {
                    name: name.to_string(),
                    description: Some(format!("来自于 {}/{} 的自动识别脚本", owner, repo)),
                    version: "auto".to_string(),
                    author: Some(owner.to_string()),
                    url: download_url.to_string(),
                    icon: None,
                    tags: vec![ext.to_uppercase()],
                    updated_at: chrono::Utc::now().timestamp(),
                    is_encrypted: false,
                });
            }
        }
    }

    Ok(MarketSourceInfo {
        name: format!("Repo: {}/{}", owner, repo),
        description: Some("自动生成的市场源".to_string()),
        icon: Some("i-ep-opportunity".to_string()),
        scripts,
    })
}

pub async fn list_aggregated_market_scripts(
    db: &DatabaseConnection,
    client: &reqwest::Client,
) -> Result<Vec<MarketScriptAggregated>> {
    let sources = market_sources::Entity::find()
        .filter(market_sources::Column::Enabled.eq(true))
        .all(db)
        .await?;

    let mut results = Vec::new();
    let mut fetches = futures::stream::iter(sources)
        .map(|source| async move {
            match fetch_market_info(client, &source.url).await {
                Ok(info) => (source.id, source.name, Some(info.scripts)),
                Err(e) => {
                    warn!("同步源 '{}' (ID: {}) 失败: {}", source.name, source.id, e);
                    (source.id, source.name, None)
                }
            }
        })
        .buffer_unordered(10);

    while let Some((sid, sname, scripts_opt)) = fetches.next().await {
        if let Some(scripts) = scripts_opt {
            for script in scripts {
                results.push(MarketScriptAggregated {
                    source_id: sid,
                    source_name: sname.clone(),
                    script,
                });
            }
        }
    }

    results.sort_by(|a, b| b.script.updated_at.cmp(&a.script.updated_at));
    Ok(results)
}
