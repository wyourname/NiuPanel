pub use super::models::{
    CreateEnvRequest, EnvironmentInfo, InstallPackageRequest, SetMirrorSourceRequest,
};
use super::usecase::EnvironmentUseCase;
use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use niupanel_common::error::Result;
use niupanel_common::response::ApiResponse;

#[utoipa::path(
    get,
    path = "/api/v1/environments",
    responses(
        (status = 200, description = "List environments", body = ApiResponse<Vec<EnvironmentInfo>>)
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn list_environments(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<EnvironmentInfo>>> {
    let envs = EnvironmentUseCase::from_state(&state)
        .list_environments()
        .await?;
    Ok(ApiResponse::success(envs))
}

#[utoipa::path(
    get,
    path = "/api/v1/environments/versions",
    responses(
        (status = 200, description = "List available Python and Node.js versions")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn list_available_versions(State(state): State<AppState>) -> Result<ApiResponse<String>> {
    let versions = EnvironmentUseCase::from_state(&state)
        .list_available_versions()
        .await?;
    Ok(ApiResponse::success(versions))
}

#[utoipa::path(
    post,
    path = "/api/v1/environments/python",
    request_body = CreateEnvRequest,
    responses(
        (status = 200, description = "Create Python environment, returns job ID")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn create_python_env(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(payload): Json<CreateEnvRequest>,
) -> Result<ApiResponse<i32>> {
    let job_id = EnvironmentUseCase::from_state(&state)
        .create_python_env(&user, ip, payload)
        .await?;
    Ok(ApiResponse::success(job_id))
}

#[utoipa::path(
    delete,
    path = "/api/v1/environments/python/{name}",
    params(
        ("name" = String, Path, description = "Environment name (e.g. venv_3.11)")
    ),
    responses(
        (status = 200, description = "Delete Python environment")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn delete_python_env(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path(name): Path<String>,
) -> Result<ApiResponse<()>> {
    EnvironmentUseCase::from_state(&state)
        .delete_python_env(&user, ip, name)
        .await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/environments/python/{name}/packages",
    params(
        ("name" = String, Path, description = "Environment name")
    ),
    responses(
        (status = 200, description = "List Python packages in environment")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn list_python_packages(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<ApiResponse<String>> {
    let list = EnvironmentUseCase::from_state(&state)
        .list_python_packages(&name)
        .await?;
    Ok(ApiResponse::success(list))
}

#[utoipa::path(
    post,
    path = "/api/v1/environments/python/{name}/packages",
    params(
        ("name" = String, Path, description = "Environment name")
    ),
    request_body = InstallPackageRequest,
    responses(
        (status = 200, description = "Install Python packages, returns job ID")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn install_python_packages(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path(name): Path<String>,
    Json(payload): Json<InstallPackageRequest>,
) -> Result<ApiResponse<i32>> {
    let job_id = EnvironmentUseCase::from_state(&state)
        .install_python_packages(&user, ip, name, payload)
        .await?;
    Ok(ApiResponse::success(job_id))
}

#[utoipa::path(
    delete,
    path = "/api/v1/environments/python/{name}/packages/{package}",
    params(
        ("name" = String, Path, description = "Environment name"),
        ("package" = String, Path, description = "Package name")
    ),
    responses(
        (status = 200, description = "Uninstall Python package, returns job ID")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn uninstall_python_package(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path((name, package)): Path<(String, String)>,
) -> Result<ApiResponse<i32>> {
    let job_id = EnvironmentUseCase::from_state(&state)
        .uninstall_python_package(&user, ip, name, package)
        .await?;
    Ok(ApiResponse::success(job_id))
}

#[utoipa::path(
    get,
    path = "/api/v1/environments/node/{name}/packages",
    params(
        ("name" = String, Path, description = "Node.js version name")
    ),
    responses(
        (status = 200, description = "List Node.js version-shared packages")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn list_node_packages(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<ApiResponse<String>> {
    let list = EnvironmentUseCase::from_state(&state)
        .list_node_packages(&name)
        .await?;
    Ok(ApiResponse::success(list))
}

#[utoipa::path(
    post,
    path = "/api/v1/environments/node",
    request_body = CreateEnvRequest,
    responses(
        (status = 200, description = "Create Node.js environment, returns job ID")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn create_node_env(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(payload): Json<CreateEnvRequest>,
) -> Result<ApiResponse<i32>> {
    let job_id = EnvironmentUseCase::from_state(&state)
        .create_node_env(&user, ip, payload)
        .await?;
    Ok(ApiResponse::success(job_id))
}

#[utoipa::path(
    delete,
    path = "/api/v1/environments/node/{name}",
    params(
        ("name" = String, Path, description = "Node.js version name")
    ),
    responses(
        (status = 200, description = "Delete Node.js environment")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn delete_node_env(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path(name): Path<String>,
) -> Result<ApiResponse<()>> {
    EnvironmentUseCase::from_state(&state)
        .delete_node_env(&user, ip, name)
        .await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/environments/node/{name}/packages",
    params(
        ("name" = String, Path, description = "Node.js version name")
    ),
    request_body = InstallPackageRequest,
    responses(
        (status = 200, description = "Install Node.js version-shared packages, returns job ID")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn install_node_packages(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path(name): Path<String>,
    Json(payload): Json<InstallPackageRequest>,
) -> Result<ApiResponse<i32>> {
    let job_id = EnvironmentUseCase::from_state(&state)
        .install_node_packages(&user, ip, name, payload)
        .await?;
    Ok(ApiResponse::success(job_id))
}

#[utoipa::path(
    delete,
    path = "/api/v1/environments/node/{name}/packages/{package}",
    params(
        ("name" = String, Path, description = "Node.js version name"),
        ("package" = String, Path, description = "Package name")
    ),
    responses(
        (status = 200, description = "Uninstall Node.js version-shared package, returns job ID")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn uninstall_node_package(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path((name, package)): Path<(String, String)>,
) -> Result<ApiResponse<i32>> {
    let job_id = EnvironmentUseCase::from_state(&state)
        .uninstall_node_package(&user, ip, name, package)
        .await?;
    Ok(ApiResponse::success(job_id))
}

#[utoipa::path(
    post,
    path = "/api/v1/environments/node/{name}/set-default",
    params(
        ("name" = String, Path, description = "Node.js version name")
    ),
    responses(
        (status = 200, description = "Set Node.js default version")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn set_node_default(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path(name): Path<String>,
) -> Result<ApiResponse<()>> {
    EnvironmentUseCase::from_state(&state)
        .set_node_default(&user, ip, name)
        .await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/environments/shell/packages",
    responses(
        (status = 200, description = "List system shell packages")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn list_shell_packages(State(state): State<AppState>) -> Result<ApiResponse<String>> {
    let list = EnvironmentUseCase::from_state(&state)
        .list_shell_packages()
        .await?;
    Ok(ApiResponse::success(list))
}

#[utoipa::path(
    post,
    path = "/api/v1/environments/shell/packages",
    request_body = InstallPackageRequest,
    responses(
        (status = 200, description = "Install system shell packages, returns job ID")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn install_shell_packages(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(payload): Json<InstallPackageRequest>,
) -> Result<ApiResponse<i32>> {
    let job_id = EnvironmentUseCase::from_state(&state)
        .install_shell_packages(&user, ip, payload)
        .await?;
    Ok(ApiResponse::success(job_id))
}

#[utoipa::path(
    delete,
    path = "/api/v1/environments/shell/packages/{package}",
    params(
        ("package" = String, Path, description = "Package name")
    ),
    responses(
        (status = 200, description = "Uninstall system shell package")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn uninstall_shell_package(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path(package): Path<String>,
) -> Result<ApiResponse<()>> {
    EnvironmentUseCase::from_state(&state)
        .uninstall_shell_package(&user, ip, package)
        .await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/environments/mirror/{env_type}",
    params(
        ("env_type" = String, Path, description = "Environment type (python or node)")
    ),
    request_body = SetMirrorSourceRequest,
    responses(
        (status = 200, description = "Set mirror source for environment")
    ),
    tag = "Environments",
    security(("session_cookie" = []))
)]
pub async fn set_mirror_source(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path(env_type): Path<String>,
    Json(payload): Json<SetMirrorSourceRequest>,
) -> Result<ApiResponse<()>> {
    EnvironmentUseCase::from_state(&state)
        .set_mirror_source(&user, ip, env_type, payload.mirror_url)
        .await?;
    Ok(ApiResponse::success(()))
}
