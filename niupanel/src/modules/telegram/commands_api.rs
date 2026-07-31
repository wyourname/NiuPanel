use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Json,
    extract::{Extension, Path, State},
};
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::{ApiResponse, PaginatedData};
use niupanel_core::audit::service::AuditService;
use niupanel_entity::tg_commands;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct QueryParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateCmdReq {
    pub name: String,
    pub script: String,
}

#[derive(Serialize, ToSchema)]
pub struct CommandDto {
    pub id: i32,
    pub name: String,
    pub script: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<tg_commands::Model> for CommandDto {
    fn from(m: tg_commands::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            script: m.script,
            created_at: m.created_at.and_utc().timestamp(),
            updated_at: m.updated_at.and_utc().timestamp(),
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/bot/telegram/commands",
    responses(
        (status = 200, description = "List Telegram custom commands")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn list_commands(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<QueryParams>,
) -> Result<ApiResponse<PaginatedData<CommandDto>>> {
    let page = std::cmp::max(params.page.unwrap_or(1), 1);
    let limit = std::cmp::max(params.limit.unwrap_or(20), 1);

    let paginator = tg_commands::Entity::find()
        .order_by_desc(tg_commands::Column::UpdatedAt)
        .paginate(&state.db, limit);

    let total = paginator.num_items().await?;
    let models = paginator.fetch_page(page - 1).await?;
    let dtos = models.into_iter().map(CommandDto::from).collect();

    Ok(ApiResponse::success(PaginatedData { items: dtos, total }))
}

#[utoipa::path(
    post,
    path = "/api/v1/bot/telegram/commands",
    request_body = CreateCmdReq,
    responses(
        (status = 200, description = "Create Telegram custom command")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn create_command(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(req): Json<CreateCmdReq>,
) -> Result<ApiResponse<CommandDto>> {
    let now = chrono::Utc::now().naive_utc();
    let model = tg_commands::ActiveModel {
        name: Set(req.name.clone()),
        script: Set(req.script),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(|e| AppError::Generic(e.to_string()))?;

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "创建TG自定义指令",
        "telegram",
        Some(model.id.to_string()),
        Some(req.name),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(model.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/bot/telegram/commands/{id}",
    params(
        ("id" = i32, Path, description = "Command ID")
    ),
    request_body = CreateCmdReq,
    responses(
        (status = 200, description = "Update Telegram custom command")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn update_command(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(req): Json<CreateCmdReq>,
) -> Result<ApiResponse<CommandDto>> {
    let mut am: tg_commands::ActiveModel = tg_commands::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::Generic("指令不存在".into()))?
        .into();

    am.name = Set(req.name.clone());
    am.script = Set(req.script);
    am.updated_at = Set(chrono::Utc::now().naive_utc());

    let model = am
        .update(&state.db)
        .await
        .map_err(|e| AppError::Generic(e.to_string()))?;

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "更新TG自定义指令",
        "telegram",
        Some(id.to_string()),
        Some(req.name),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(model.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/bot/telegram/commands/{id}",
    params(
        ("id" = i32, Path, description = "Command ID")
    ),
    responses(
        (status = 200, description = "Delete Telegram custom command")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn delete_command(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
) -> Result<ApiResponse<()>> {
    let result = tg_commands::Entity::delete_by_id(id)
        .exec(&state.db)
        .await?;
    if result.rows_affected > 0 {
        AuditService::log(
            &state.db,
            Some(user.id),
            "User",
            "删除TG自定义指令",
            "telegram",
            Some(id.to_string()),
            None,
            Some(ip),
        )
        .await;
    }
    Ok(ApiResponse::success(()))
}
