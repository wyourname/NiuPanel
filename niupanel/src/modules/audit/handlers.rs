use crate::common::state::AppState;
use axum::extract::{Query, State};
use niupanel_common::error::Result;
use niupanel_common::response::{ApiResponse, PaginatedData};
use niupanel_entity::audit_logs;
use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder, QuerySelect};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct AuditQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/audit",
    params(AuditQueryParams),
    responses(
        (status = 200, description = "List audit logs", body = ApiResponse<PaginatedData<niupanel_entity::audit_logs::Model>>)
    ),
    tag = "Audit",
    security(("session_cookie" = []))
)]
pub async fn list_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditQueryParams>,
) -> Result<ApiResponse<PaginatedData<audit_logs::Model>>> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let total = audit_logs::Entity::find().count(&state.db).await?;

    let items = audit_logs::Entity::find()
        .order_by_desc(audit_logs::Column::CreatedAt)
        .limit(page_size)
        .offset(offset)
        .all(&state.db)
        .await?;

    Ok(ApiResponse::success(PaginatedData { items, total }))
}
