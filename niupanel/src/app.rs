use crate::common::state::AppState;
use crate::modules;
use crate::modules::auth::middleware::{auth_middleware, token_auth_middleware};
use axum::http::{HeaderValue, Method, Request, header};
use axum::middleware::Next;
use axum::response::Response;
use axum::{Router, extract::DefaultBodyLimit, middleware};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

pub fn create_router(state: AppState) -> Router {
    let config = niupanel_common::config::Config::global();
    if let Err(error) = modules::system::web_releases::ensure_web_runtime() {
        tracing::warn!("Web release runtime is not ready: {error}");
    }
    let web_root = modules::system::web_releases::web_root();
    let cors = if config.cors_allowed_origins.is_empty() {
        CorsLayer::new().allow_origin(tower_http::cors::AllowOrigin::mirror_request())
    } else {
        let origins: Vec<_> = config
            .cors_allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new().allow_origin(tower_http::cors::AllowOrigin::list(origins))
    }
    .allow_methods([
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
        Method::OPTIONS,
        Method::HEAD,
    ])
    .allow_headers([
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::ORIGIN,
        header::COOKIE,
        header::HeaderName::from_static("x-api-key"),
        header::HeaderName::from_static("mcp-session-id"),
        header::HeaderName::from_static("mcp-protocol-version"),
        header::HeaderName::from_static("last-event-id"),
    ])
    .allow_credentials(true);

    let trace_layer = create_trace_layer();

    Router::new()
        .merge(modules::system::routes::create_public_router())
        .merge(modules::mcp::routes::create_protocol_router(state.clone()))
        .nest("/api/v1", create_internal_router(state.clone()))
        .merge(modules::documents::routes::create_router().route_layer(
            middleware::from_fn_with_state(state.clone(), auth_middleware),
        ))
        .nest("/open/api", create_openapi_router(state.clone()))
        .fallback_service(
            ServeDir::new(&web_root).not_found_service(ServeFile::new(web_root.join("index.html"))),
        )
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024))
        .layer(middleware::from_fn(web_cache_headers))
        .layer(cors)
        .layer(trace_layer)
        .with_state(state)
}

async fn web_cache_headers(request: Request<axum::body::Body>, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;
    if path == "/" || path.ends_with(".html") {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        );
    } else if path.starts_with("/assets/") {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    response
}

/// 内部 API - 供 Web UI 使用
fn create_internal_router(state: AppState) -> Router<AppState> {
    let modules = [
        ("/tasks", modules::tasks::routes::create_router()),
        ("/jobs", modules::jobs::routes::create_router()),
        ("/overview", modules::overview::routes::create_router()),
        ("/variables", modules::variable::routes::create_router()),
        (
            "/environments",
            modules::environment::routes::create_router(),
        ),
        ("/share", modules::share::routes::create_router()),
        ("/settings", modules::settings::routes::create_router()),
        ("/system", modules::system::routes::create_router()),
        ("/compiler", modules::compiler::routes::create_router()),
        ("/mcp", modules::mcp::routes::create_management_router()),
        ("/plugins", modules::plugins::routes::create_router()),
        ("/keys", modules::system_key::routes::create_router()),
        ("/audit", modules::audit::routes::create_router()),
        ("/webhook", modules::webhook::routes::create_router()),
        ("/git", modules::git_sync::routes::create_router()),
        ("/terminal", modules::terminal::routes::create_router()),
        ("/bot", modules::telegram::routes::create_router()),
    ];

    let business_routes = modules
        .into_iter()
        .fold(Router::<AppState>::new(), |r, (path, router)| {
            r.nest(path, router)
        })
        .merge(modules::plugins::routes::create_root_router())
        .nest(
            "/files",
            modules::file_manager::routes::create_router()
                .layer(DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Routes that MUST NOT have auth_middleware (they handle the authentication itself)
    let public_routes = Router::new()
        .nest(
            "/users",
            modules::user::routes::create_router(state.clone()),
        )
        .nest("/auth", modules::auth::routes::create_router(state));

    public_routes.merge(business_routes)
}

/// OpenAPI - 供系统密钥使用，强制 API Key 认证
fn create_openapi_router(state: AppState) -> Router<AppState> {
    let modules = [
        ("/tasks", modules::tasks::routes::create_router()),
        ("/jobs", modules::jobs::routes::create_router()),
        ("/variables", modules::variable::routes::create_router()),
        (
            "/environments",
            modules::environment::routes::create_router(),
        ),
        ("/compiler", modules::compiler::routes::create_router()),
        ("/overview", modules::overview::routes::create_router()),
        ("/share", modules::share::routes::create_router()),
        ("/webhook", modules::webhook::routes::create_router()),
    ];

    modules
        .into_iter()
        .fold(Router::<AppState>::new(), |r, (path, router)| {
            r.nest(path, router)
        })
        .nest(
            "/files",
            modules::file_manager::routes::create_router()
                .layer(DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .layer(middleware::from_fn_with_state(state, token_auth_middleware))
}

fn create_trace_layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
> {
    TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::DEBUG))
        .on_request(DefaultOnRequest::new().level(Level::DEBUG))
        .on_response(DefaultOnResponse::new().level(Level::DEBUG))
}
