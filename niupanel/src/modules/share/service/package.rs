use super::common::{generate_package, get_station_config};
use crate::modules::auth::service::AuthenticatedUser;
use crate::modules::share::models::{CreateShareRequest, NiuPackage, TaskExportSpec};
use niupanel_common::error::{AppError, Result};
use niupanel_common::info;
use niupanel_entity::station_files;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Set};
use tokio::io::AsyncWriteExt;

async fn compress_package(package: &NiuPackage) -> Result<Vec<u8>> {
    let mut msgpack_bytes = Vec::new();
    rmp_serde::encode::write(&mut msgpack_bytes, package)
        .map_err(|e| AppError::Serialization(format!("MsgPack 序列化失败: {}", e)))?;

    let mut encoder = async_compression::tokio::write::GzipEncoder::new(Vec::new());
    encoder
        .write_all(&msgpack_bytes)
        .await
        .map_err(AppError::Io)?;
    encoder.shutdown().await.map_err(AppError::Io)?;
    Ok(encoder.into_inner())
}

async fn upload_content_to_station(
    client: &reqwest::Client,
    station_url: &str,
    admin_token: &str,
    token: &str,
    content: Vec<u8>,
    req: &CreateShareRequest,
    is_update: bool,
) -> Result<()> {
    let url = if is_update {
        format!("{}/admin/files", station_url)
    } else {
        format!("{}/niupanel/upload", station_url)
    };

    let mut query = vec![("token", token.to_string())];
    if !is_update {
        if let Some(h) = req.expires_in_hours {
            query.push(("expire", h.to_string()));
        }
        if let Some(b) = req.burn_after_reading {
            query.push(("burn", b.to_string()));
        }
        if let Some(l) = req.max_uses {
            query.push(("limit", l.to_string()));
        }
        if let Some(n) = &req.note {
            query.push(("note", n.to_string()));
        }
    }
    if let Some(ref p) = req.password {
        query.push(("password", p.clone()));
    }

    let builder = if is_update {
        client.post(&url).header("X-Admin-Auth", admin_token)
    } else {
        client.put(&url)
    };

    let res = builder
        .query(&query)
        .body(content)
        .send()
        .await
        .map_err(|e| AppError::ExternalApi(format!("连接中转站失败: {}", e)))?;

    if !res.status().is_success() {
        let msg = res.text().await.unwrap_or_else(|_| "未知错误".to_string());
        return Err(AppError::ExternalApi(format!("中转站上传失败: {}", msg)));
    }
    Ok(())
}

pub async fn create_and_upload_share(
    db: &DatabaseConnection,
    client: &reqwest::Client,
    user: &AuthenticatedUser,
    req: CreateShareRequest,
) -> Result<String> {
    let (station_url, admin_token) = get_station_config(db).await;
    if station_url.is_empty() {
        return Err(AppError::Station("中转站地址未设置".to_string()));
    }

    info!("开始生成分享包并上传至中转站...");
    let package = generate_package(db, &req.tasks).await?;
    let compressed_bytes = compress_package(&package).await?;
    let token = nanoid::nanoid!(8);

    upload_content_to_station(
        client,
        &station_url,
        &admin_token,
        &token,
        compressed_bytes,
        &req,
        false,
    )
    .await?;

    let station_file = station_files::ActiveModel {
        token: Set(token.clone()),
        user_id: Set(user.id),
        file_name: Set(format!("{}.npack", token)),
        note: Set(req.note.clone()),
        password: Set(req.password.clone()),
        uploaded_at: Set(chrono::Utc::now().into()),
        package_config: Set(serde_json::to_string(&req.tasks).ok()),
    };
    station_file.insert(db).await?;

    let full_url = format!("{}/share/{}", station_url, token);
    info!("分享创建成功: {}", full_url);
    Ok(full_url)
}

pub async fn update_station_content(
    db: &DatabaseConnection,
    client: &reqwest::Client,
    token: &str,
) -> Result<()> {
    let (station_url, admin_token) = get_station_config(db).await;
    let station_file = station_files::Entity::find_by_id(token)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("本地中转文件记录不存在".to_string()))?;

    info!("开始自动更新分享内容 (Token: {})...", token);
    let config_str = station_file
        .package_config
        .as_ref()
        .ok_or_else(|| AppError::Generic("无包配置信息".to_string()))?;
    let tasks_spec: Vec<TaskExportSpec> = serde_json::from_str(config_str)
        .map_err(|_| AppError::Serialization("解析失败".to_string()))?;

    let package = generate_package(db, &tasks_spec).await?;
    let compressed_bytes = compress_package(&package).await?;

    let dummy_req = CreateShareRequest {
        tasks: vec![],
        password: station_file.password.clone(),
        ..Default::default()
    };
    upload_content_to_station(
        client,
        &station_url,
        &admin_token,
        token,
        compressed_bytes,
        &dummy_req,
        true,
    )
    .await?;

    let mut active = station_file.into_active_model();
    active.uploaded_at = Set(chrono::Utc::now().into());
    active.update(db).await?;
    info!("分享内容更新完成 (Token: {})", token);
    Ok(())
}
