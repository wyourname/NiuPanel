use super::models::McpInfoResponse;
use niupanel_common::error::Result;
use niupanel_common::response::ApiResponse;

#[utoipa::path(
    get,
    path = "/api/v1/mcp/info",
    responses((status = 200, description = "Get NiuPanel MCP server connection information")),
    tag = "MCP",
    security(("session_cookie" = []))
)]
pub async fn get_info() -> Result<ApiResponse<McpInfoResponse>> {
    Ok(ApiResponse::success(McpInfoResponse::current()))
}
