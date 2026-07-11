use super::{
    ApiDocAuth, ApiDocFiles, ApiDocMcp, ApiDocPlugins, ApiDocSettings, ApiDocShareExport,
    ApiDocShareImport, ApiDocSystem, ApiDocTasks,
};
use crate::common::state::AppState;
use axum::Router;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use std::sync::OnceLock;
use utoipa::OpenApi;

static OPENAPI_JSON: OnceLock<String> = OnceLock::new();

macro_rules! spawn_openapi {
    ($name:expr, $ctor:expr) => {
        std::thread::Builder::new()
            .name($name.into())
            .stack_size(8 * 1024 * 1024)
            .spawn($ctor)
            .expect(concat!("Failed to spawn ", $name, " thread"))
    };
}

fn build_spec_json() -> String {
    let h1 = spawn_openapi!("openapi-auth", || ApiDocAuth::openapi());
    let h2 = spawn_openapi!("openapi-tasks", || ApiDocTasks::openapi());
    let h3 = spawn_openapi!("openapi-settings", || ApiDocSettings::openapi());
    let h4 = spawn_openapi!("openapi-files", || ApiDocFiles::openapi());
    let h5 = spawn_openapi!("openapi-share-export", || ApiDocShareExport::openapi());
    let h6 = spawn_openapi!("openapi-share-import", || ApiDocShareImport::openapi());
    let h7 = spawn_openapi!("openapi-plugins", || ApiDocPlugins::openapi());
    let h8 = spawn_openapi!("openapi-mcp", || ApiDocMcp::openapi());
    let h9 = spawn_openapi!("openapi-system", || ApiDocSystem::openapi());
    let mut spec = h1.join().expect("openapi-auth panicked");
    spec.merge(h2.join().expect("openapi-tasks panicked"));
    spec.merge(h3.join().expect("openapi-settings panicked"));
    spec.merge(h4.join().expect("openapi-files panicked"));
    spec.merge(h5.join().expect("openapi-share-export panicked"));
    spec.merge(h6.join().expect("openapi-share-import panicked"));
    spec.merge(h7.join().expect("openapi-plugins panicked"));
    spec.merge(h8.join().expect("openapi-mcp panicked"));
    spec.merge(h9.join().expect("openapi-system panicked"));
    serde_json::to_string(&spec).expect("Failed to serialize OpenAPI spec")
}

fn get_openapi_json() -> &'static str {
    OPENAPI_JSON.get_or_init(|| {
        std::thread::Builder::new()
            .name("openapi-build".into())
            .stack_size(4 * 1024 * 1024)
            .spawn(build_spec_json)
            .expect("Failed to spawn openapi-build thread")
            .join()
            .expect("openapi-build thread panicked")
    })
}

async fn openapi_json() -> impl IntoResponse {
    let json: &str = get_openapi_json();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json,
    )
}

pub fn create_router() -> Router<AppState> {
    Router::new().route(
        "/api/v1/documents/openapi.json",
        axum::routing::get(openapi_json),
    )
}

#[cfg(test)]
mod tests {
    use super::build_spec_json;

    #[test]
    fn public_openapi_exposes_plugins_without_legacy_agents_routes() {
        let spec: serde_json::Value =
            serde_json::from_str(&build_spec_json()).expect("valid OpenAPI JSON");
        let paths = spec
            .get("paths")
            .and_then(|value| value.as_object())
            .expect("OpenAPI paths object");

        for path in [
            "/api/v1/plugins",
            "/api/v1/plugins/preview",
            "/api/v1/plugins/install",
            "/api/v1/plugins/preview-upload",
            "/api/v1/plugins/upload",
            "/api/v1/plugins/{id}/versions",
            "/api/v1/plugins/{id}/preview-update",
            "/api/v1/plugins/{id}/update",
            "/api/v1/plugins/{id}/preview-upload-update",
            "/api/v1/plugins/{id}/upload-update",
            "/api/v1/plugins/{id}/rollback/{version_id}",
            "/api/v1/plugins/{id}/enable",
            "/api/v1/plugins/{id}/disable",
            "/api/v1/plugins/{id}",
            "/api/v1/plugins/apps",
            "/api/v1/plugins/health",
            "/api/v1/plugins/market",
            "/api/v1/plugins/market/sources",
            "/api/v1/plugins/market/updates",
            "/api/v1/plugins/market/preview",
            "/api/v1/plugins/market/install",
            "/api/v1/plugins/{id}/invoke",
            "/api/v1/mcp/info",
            "/api/v1/system/meta",
            "/api/v1/system/core/releases",
            "/api/v1/system/core/releases/{version}/activate",
            "/api/v1/system/web/releases",
            "/api/v1/system/web/releases/upload",
            "/api/v1/system/web/releases/{version}/activate",
            "/api/v1/system/web/rollback",
            "/api/v1/system/web/update/check",
            "/api/v1/system/web/update/install",
        ] {
            assert!(paths.contains_key(path), "missing OpenAPI path: {path}");
        }

        for path in paths.keys() {
            assert!(
                !path.starts_with("/api/v1/agents"),
                "public OpenAPI must not expose legacy agents path: {path}"
            );
            assert!(
                !path.starts_with("/api/v1/plugins/agents")
                    && !path.starts_with("/api/v1/plugins/compiler"),
                "public OpenAPI must not expose capability-specific plugin management paths: {path}"
            );
        }

        for path in paths.keys().filter(|path| path.starts_with("/api/v1/mcp/")) {
            assert!(
                path.as_str() == "/api/v1/mcp/info",
                "OpenAPI must only expose MCP server management information: {path}"
            );
        }
    }
}
