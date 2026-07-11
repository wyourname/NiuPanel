use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use axum::{
    Json,
    extract::{Extension, Path, State},
};
use chrono::Utc;
use niupanel_common::error::Result;
use niupanel_common::response::ApiResponse;
use niupanel_core::audit::service::AuditService;
use niupanel_entity::api_keys;
use sea_orm::{ActiveModelTrait, EntityTrait, ModelTrait, QueryOrder, Set};
use serde::{Deserialize, Serialize};
// use std::net::SocketAddr;
use crate::modules::auth::service::AuthenticatedUser;
use niupanel_common::auth::permissions::parse_permissions;
use niupanel_common::error::AppError;
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ApiKeyDto {
    pub id: i32,
    pub name: String,
    pub prefix: String, // Only show first few chars of session ID
    pub permissions: Option<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub last_used_ip: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: Option<String>, // "read,write" etc.
    pub expires_in_days: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateApiKeyRequest {
    pub name: Option<String>,
    pub permissions: Option<String>, // "read,write" etc.
    pub expires_at: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    pub id: i32,
    pub name: String,
    pub token: String, // The session ID, only shown once
}

#[utoipa::path(
    get,
    path = "/api/v1/keys",
    responses(
        (status = 200, description = "List API keys")
    ),
    tag = "API Keys",
    security(("session_cookie" = []))
)]
pub async fn list_api_keys(State(state): State<AppState>) -> Result<ApiResponse<Vec<ApiKeyDto>>> {
    let keys = api_keys::Entity::find()
        .order_by_desc(api_keys::Column::CreatedAt)
        .all(&state.db)
        .await?;

    let dtos = keys
        .into_iter()
        .map(|k| ApiKeyDto {
            id: k.id,
            prefix: k.name.clone(), // Show name instead of prefix since we store hash now
            name: k.name,
            permissions: k.permissions,
            created_at: k.created_at.timestamp(),
            expires_at: k.expires_at.map(|t| t.timestamp()),
            last_used_at: k.last_used_at.map(|t| t.timestamp()),
            last_used_ip: k.last_used_ip,
        })
        .collect();

    Ok(ApiResponse::success(dtos))
}

#[utoipa::path(
    post,
    path = "/api/v1/keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 200, description = "Create API key")
    ),
    tag = "API Keys",
    security(("session_cookie" = []))
)]
pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<ApiResponse<CreateApiKeyResponse>> {
    // 0. Validate Permissions
    if let Some(p_str) = &payload.permissions {
        if !p_str.trim().is_empty() {
            let perms = parse_permissions(p_str);
            if perms.is_empty() {
                return Err(AppError::ValidationError("无效的权限格式或权限项".into()));
            }
        }
    }

    // 1. Generate Token
    let token = nanoid::nanoid!(32);

    // 2. Hash the token for storage
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let token_hash = hex::encode(hasher.finalize());

    // 3. Prepare Expiry
    let now = Utc::now();
    let expiry_date = if let Some(days) = payload.expires_in_days {
        now + chrono::Duration::days(days)
    } else {
        now + chrono::Duration::days(3650)
    };

    // 5. Insert Metadata into api_keys table
    let new_key = api_keys::ActiveModel {
        user_id: Set(user.id),     // Bind to current user
        key_hash: Set(token_hash), // Store the hash
        name: Set(payload.name.clone()),
        permissions: Set(payload.permissions),
        created_at: Set(now.into()),
        expires_at: Set(Some(expiry_date.into())),
        ..Default::default()
    };

    let res = new_key.insert(&state.db).await?;

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "创建API密钥",
        "system_key",
        Some(res.id.to_string()),
        Some(format!("创建了 API 密钥: {}", payload.name)),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(CreateApiKeyResponse {
        id: res.id,
        name: res.name,
        token, // Return original token ONLY ONCE
    }))
}

#[utoipa::path(
    put,
    path = "/api/v1/keys/{id}",
    params(
        ("id" = i32, Path, description = "API Key ID")
    ),
    request_body = UpdateApiKeyRequest,
    responses(
        (status = 200, description = "Update API key")
    ),
    tag = "API Keys",
    security(("session_cookie" = []))
)]
pub async fn update_api_key(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateApiKeyRequest>,
) -> Result<ApiResponse<()>> {
    // 1. Find key record
    let key_record = api_keys::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("API Key not found".to_string()))?;

    let mut active: api_keys::ActiveModel = key_record.clone().into();
    let mut permissions_changed = false;

    if let Some(name) = payload.name {
        active.name = Set(name);
    }

    if let Some(p_str) = payload.permissions {
        if !p_str.trim().is_empty() {
            let perms = parse_permissions(&p_str);
            if perms.is_empty() {
                return Err(AppError::ValidationError("无效的权限格式或权限项".into()));
            }
        }
        if active.permissions != Set(Some(p_str.clone())) {
            active.permissions = Set(Some(p_str));
            permissions_changed = true;
        }
    }

    if let Some(expires_at) = payload.expires_at {
        let dt = chrono::DateTime::<Utc>::from_timestamp(expires_at, 0).ok_or(
            AppError::ValidationError("Invalid expiration timestamp".into()),
        )?;
        active.expires_at = Set(Some(dt.into()));
    }

    let res = active.update(&state.db).await?;

    // 2. Invalidate Cache if permissions changed
    if permissions_changed {
        state.api_key_cache.invalidate(&key_record.key_hash).await;
    }

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "更新API密钥",
        "system_key",
        Some(id.to_string()),
        Some(format!("更新了 API 密钥: {}", res.name)),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/keys/{id}",
    params(
        ("id" = i32, Path, description = "API Key ID")
    ),
    responses(
        (status = 200, description = "Delete API key")
    ),
    tag = "API Keys",
    security(("session_cookie" = []))
)]
pub async fn delete_api_key(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path(id): Path<i32>,
) -> Result<ApiResponse<()>> {
    // 1. Find key record
    let key_record = api_keys::Entity::find_by_id(id).one(&state.db).await?;

    if let Some(record) = key_record {
        let name = record.name.clone();
        let hash = record.key_hash.clone();

        // 2. Delete from api_keys
        record.delete(&state.db).await?;

        // 3. Invalidate Cache
        state.api_key_cache.invalidate(&hash).await;

        AuditService::log(
            &state.db,
            Some(user.id),
            "User",
            "删除API密钥",
            "system_key",
            Some(id.to_string()),
            Some(format!("删除了 API 密钥: {}", name)),
            Some(ip),
        )
        .await;
    }

    Ok(ApiResponse::success(()))
}
