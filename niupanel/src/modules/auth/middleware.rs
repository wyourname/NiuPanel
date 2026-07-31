use crate::common::extractors::RealIp;
use axum::{
    extract::{Extension, Request, State},
    http::{HeaderMap, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use niupanel_common::error::AppError;
use tower_sessions::Session;
// use niupanel_common::logger::{debug};
use super::service::AuthenticatedUser;
use crate::common::state::AppState;
use niupanel_common::auth::permissions::{Permission, UserRole, parse_permissions};
use niupanel_common::auth::sdk_token;
use niupanel_common::config::Config;
use niupanel_entity::{api_keys, users};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::collections::HashSet;
// use std::net::SocketAddr;
use axum::Router;
use sha2::{Digest, Sha256};
use std::str::FromStr;

const USER_ID_KEY: &str = "user_id";
const USER_ROLE_KEY: &str = "user_role";
const USER_PERMISSIONS_KEY: &str = "user_permissions";

/// Session 认证中间件 - 用于内部 API (Web UI)
pub async fn auth_middleware(
    State(state): State<AppState>,
    RealIp(ip): RealIp,
    mut req: Request,
    next: Next,
) -> Response {
    let internal_user_token = bearer_token(req.headers())
        .filter(|token| sdk_token::is_internal_user_token(token))
        .map(str::to_string);
    if let Some(token) = internal_user_token {
        return match authenticate_internal_user_token(&state, &token, ip, req, next).await {
            Ok(response) => response,
            Err(err) => err.into_response(),
        };
    }

    // 1. Check Session
    let session = match req.extensions().get::<Session>() {
        Some(s) => s,
        None => return AppError::Auth("Session layer missing".to_string()).into_response(),
    };

    let user_id: i32 = match session.get(USER_ID_KEY).await {
        Ok(Some(id)) => id,
        Ok(None) => return AppError::Auth("Authentication required".to_string()).into_response(),
        Err(e) => return AppError::Auth(format!("Session error: {}", e)).into_response(),
    };

    // 2. Try get role from session first to avoid DB query (Performance)
    let session_role = session.get::<String>(USER_ROLE_KEY).await.unwrap_or(None);
    let session_perms = session
        .get::<String>(USER_PERMISSIONS_KEY)
        .await
        .unwrap_or(None);

    let (role, permissions_str): (UserRole, Option<String>) = if user_id == 0 {
        (UserRole::System, None)
    } else {
        if let (Some(r_str), Some(p_str)) = (session_role, session_perms) {
            (
                UserRole::from_str(&r_str).unwrap_or(UserRole::User),
                Some(p_str),
            )
        } else {
            // Fallback to DB
            match users::Entity::find_by_id(user_id).one(&state.db).await {
                Ok(Some(user_model)) => {
                    let r = UserRole::from_str(&user_model.role).unwrap_or(UserRole::User);
                    let p = user_model.permissions.clone();

                    let _ = session.insert(USER_ROLE_KEY, r.as_ref()).await;
                    // Always insert a string (even if empty) to mark that we've cached the permissions
                    let p_str = p.clone().unwrap_or_default();
                    let _ = session.insert(USER_PERMISSIONS_KEY, p_str.clone()).await;

                    (r, Some(p_str))
                }
                Ok(None) => return AppError::Auth("User not found".to_string()).into_response(),
                Err(e) => return AppError::Database(e).into_response(),
            }
        }
    };

    let mut permissions = if let Some(p_str) = permissions_str {
        parse_permissions(&p_str)
    } else {
        HashSet::new()
    };

    // Everyone can manage their own profile
    permissions.insert(Permission::ProfileRead);
    permissions.insert(Permission::ProfileUpdate);

    let user = AuthenticatedUser {
        id: user_id,
        role,
        permissions,
        ip,
    };
    req.extensions_mut().insert(user);

    next.run(req).await
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let auth_header = headers.get(header::AUTHORIZATION)?;
    let auth_str = auth_header.to_str().ok()?;
    let token = auth_str.trim_start_matches("Bearer ").trim();
    (!token.is_empty()).then_some(token)
}

fn api_key_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

use crate::common::state::CachedApiKey;

/// Token 认证中间件 - 用于 OpenAPI (System Key) - 严格校验
pub async fn token_auth_middleware(
    State(state): State<AppState>,
    RealIp(ip): RealIp,
    mut req: Request,
    next: Next,
) -> Response {
    let token = match api_key_token(req.headers()) {
        Some(token) => token.to_string(),
        None => return AppError::Auth("Missing X-API-Key header".to_string()).into_response(),
    };

    if sdk_token::is_internal_sdk_token(&token) {
        return match authenticate_internal_sdk_token(&token, ip, req, next).await {
            Ok(response) => response,
            Err(err) => err.into_response(),
        };
    }

    // 1. Hash
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let token_hash = hex::encode(hasher.finalize());

    // 2. Try Cache First
    let mut cached = state.api_key_cache.get(&token_hash).await;

    // Check if the cached key expired while in cache
    if let Some(ref c) = cached {
        if let Some(expires) = c.expires_at {
            if chrono::Utc::now() > expires {
                state.api_key_cache.invalidate(&token_hash).await;
                cached = None; // Force fallback to DB / rejection
            }
        }
    }

    let (key_id, user_id, permissions) = if let Some(c) = cached {
        (c.key_id, c.user_id, c.permissions)
    } else {
        // 3. Fallback to DB
        let key_record = match api_keys::Entity::find()
            .filter(api_keys::Column::KeyHash.eq(&token_hash))
            .one(&state.db)
            .await
        {
            Ok(Some(k)) => k,
            Ok(None) => return AppError::Auth("Invalid API Key".to_string()).into_response(),
            Err(e) => return AppError::Database(e).into_response(),
        };

        if let Some(expires) = key_record.expires_at {
            if chrono::Utc::now() > expires {
                return AppError::Auth("API Key expired".to_string()).into_response();
            }
        }

        let permissions = if let Some(p_str) = key_record.permissions {
            parse_permissions(&p_str)
        } else {
            HashSet::new()
        };

        state
            .api_key_cache
            .insert(
                token_hash.clone(),
                CachedApiKey {
                    key_id: key_record.id,
                    user_id: key_record.user_id,
                    permissions: permissions.clone(),
                    expires_at: key_record.expires_at,
                },
            )
            .await;

        (key_record.id, key_record.user_id, permissions)
    };

    // Update Usage (Async)
    let db_clone = state.db.clone();
    let ip_clone = ip.clone();
    tokio::spawn(async move {
        let active_key = api_keys::ActiveModel {
            id: Set(key_id),
            last_used_at: Set(Some(chrono::Utc::now().into())),
            last_used_ip: Set(Some(ip_clone)),
            ..Default::default()
        };
        let _ = active_key.update(&db_clone).await;
    });

    let user = AuthenticatedUser {
        id: user_id,
        role: UserRole::ApiClient,
        permissions,
        ip,
    };

    req.extensions_mut().insert(user);
    next.run(req).await
}

async fn authenticate_internal_sdk_token(
    token: &str,
    ip: String,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let claims = sdk_token::verify_internal_sdk_token(&Config::global().session_key, token)
        .map_err(|_| AppError::Auth("Invalid internal SDK token".to_string()))?;

    let user = AuthenticatedUser {
        id: 0,
        role: UserRole::System,
        permissions: internal_sdk_permissions(),
        ip,
    };

    req.extensions_mut().insert(claims);
    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

async fn authenticate_internal_user_token(
    state: &AppState,
    token: &str,
    ip: String,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let claims = sdk_token::verify_internal_user_token(&Config::global().session_key, token)
        .map_err(|_| AppError::Auth("Invalid internal user token".to_string()))?;
    if claims.user_id <= 0 {
        return Err(AppError::Auth("Invalid internal user token".to_string()));
    }

    let user = load_authenticated_user(state, claims.user_id, ip).await?;
    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

async fn load_authenticated_user(
    state: &AppState,
    user_id: i32,
    ip: String,
) -> Result<AuthenticatedUser, AppError> {
    let user_model = users::Entity::find_by_id(user_id)
        .one(&state.db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    let role = UserRole::from_str(&user_model.role).unwrap_or(UserRole::User);
    let mut permissions = user_model
        .permissions
        .as_deref()
        .map(parse_permissions)
        .unwrap_or_default();
    permissions.insert(Permission::ProfileRead);
    permissions.insert(Permission::ProfileUpdate);

    Ok(AuthenticatedUser {
        id: user_id,
        role,
        permissions,
        ip,
    })
}

fn internal_sdk_permissions() -> HashSet<Permission> {
    [
        Permission::TaskAll,
        Permission::VarAll,
        Permission::FileAll,
        Permission::EnvAll,
        Permission::JobAll,
        Permission::OverviewRead,
        Permission::CompilerRead,
        Permission::CompilerRun,
        Permission::ShareAll,
        Permission::WebhookPush,
    ]
    .into_iter()
    .collect()
}

pub async fn require_permission(
    permission: Permission,
    Extension(user): Extension<AuthenticatedUser>,
    req: Request,
    next: Next,
) -> Response {
    if user.has_permission(permission) {
        next.run(req).await
    } else {
        AppError::Forbidden("Permission denied".into()).into_response()
    }
}

/// 权限控制扩展 Trait，让路由定义更优雅
pub trait PermissionExt {
    fn require(self, perm: Permission) -> Self;
}

impl PermissionExt for Router<AppState> {
    fn require(self, perm: Permission) -> Self {
        self.route_layer(axum::middleware::from_fn(
            move |user, req, next| async move { require_permission(perm, user, req, next).await },
        ))
    }
}
