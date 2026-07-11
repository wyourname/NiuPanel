use super::common::get_station_config;
use crate::modules::share::models::{
    StationFile, StationFilesResponse, StationStats, UpdateStationFileRequest,
};
use niupanel_common::error::{AppError, Result};
use niupanel_common::{info, warn};
use niupanel_entity::station_files;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Set};
use std::collections::HashMap;

pub async fn list_station_files(
    db: &DatabaseConnection,
    client: &reqwest::Client,
) -> Result<Vec<StationFile>> {
    let (url, admin_token) = get_station_config(db).await;
    if url.is_empty() {
        return Ok(Vec::new());
    }

    let res = client
        .get(format!("{}/admin/files", url))
        .header("X-Admin-Auth", admin_token)
        .send()
        .await
        .map_err(|e| AppError::ExternalApi(format!("中转站连接失败: {}", e)))?;

    let mut remote_files: StationFilesResponse = res
        .json()
        .await
        .map_err(|e| AppError::ExternalApi(e.to_string()))?;
    let local_files = station_files::Entity::find().all(db).await?;
    let local_map: HashMap<_, _> = local_files
        .into_iter()
        .map(|f| (f.token.clone(), f))
        .collect();

    for f in &mut remote_files.files {
        if let Some(l) = local_map.get(&f.token) {
            f.note = l.note.clone();
            f.uploaded_at = Some(l.uploaded_at.timestamp());
        }
    }
    remote_files
        .files
        .sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));
    Ok(remote_files.files)
}

pub async fn get_station_stats(
    db: &DatabaseConnection,
    client: &reqwest::Client,
) -> Result<StationStats> {
    let (url, token) = get_station_config(db).await;
    if url.is_empty() {
        return Ok(StationStats {
            is_configured: false,
            ..Default::default()
        });
    }

    let res = client
        .get(format!("{}/admin/stats", url))
        .header("X-Admin-Auth", token)
        .send()
        .await
        .map_err(|e| AppError::ExternalApi(format!("获取中转站状态失败: {}", e)))?;

    let mut stats: StationStats = res
        .json()
        .await
        .map_err(|e| AppError::ExternalApi(e.to_string()))?;
    stats.is_configured = true;
    Ok(stats)
}

pub async fn update_station_file(
    db: &DatabaseConnection,
    client: &reqwest::Client,
    token: &str,
    payload: UpdateStationFileRequest,
) -> Result<()> {
    let (url, admin_token) = get_station_config(db).await;
    if !url.is_empty() {
        client
            .patch(format!("{}/admin/files/{}", url, token))
            .header("X-Admin-Auth", admin_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("更新中转文件失败: {}", e)))?;
    }
    if let Some(note) = payload.note {
        if let Some(existing) = station_files::Entity::find_by_id(token).one(db).await? {
            let mut active = existing.into_active_model();
            active.note = Set(Some(note));
            active.update(db).await?;
        }
    }
    Ok(())
}

pub async fn delete_station_file(
    db: &DatabaseConnection,
    client: &reqwest::Client,
    token: &str,
) -> Result<()> {
    let (url, admin_token) = get_station_config(db).await;
    info!("删除中转站文件 (Token: {})...", token);
    if !url.is_empty() {
        let res = client
            .delete(format!("{}/admin/files/{}", url, token))
            .header("X-Admin-Auth", admin_token)
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("删除中转文件请求失败: {}", e)))?;
        if !res.status().is_success() && res.status() != reqwest::StatusCode::NOT_FOUND {
            warn!(
                "中转站远程删除失败 (Token: {}), 状态码: {}",
                token,
                res.status()
            );
        }
    }
    station_files::Entity::delete_by_id(token).exec(db).await?;
    info!("中转站文件删除成功 (Token: {})", token);
    Ok(())
}
