use super::layout::RuntimeKind;
use crate::common::{extractors::RealIp, state::AppState};
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use niupanel_common::{
    error::{AppError, Result},
    response::ApiResponse,
};
use niupanel_core::{
    audit::service::AuditService,
    settings::{
        NPM_REGISTRY_MIRROR, PNPM_NODE_DIST_MIRROR, SETTINGS_REGISTRY, SettingsService,
        UV_PYPI_MIRROR, UV_PYTHON_MIRROR,
    },
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct MirrorSettingsUpdate {
    pub package_url: String,
    pub runtime_url: String,
}

#[derive(Serialize, ToSchema)]
pub struct MirrorSettings {
    pub package_url: String,
    pub runtime_url: String,
    pub package_override: Option<String>,
    pub runtime_override: Option<String>,
}

impl MirrorSettings {
    fn keys(env_type: &str) -> Result<(&'static str, &'static str)> {
        match RuntimeKind::from_env_type(env_type) {
            Some(RuntimeKind::Python) => Ok((UV_PYPI_MIRROR, UV_PYTHON_MIRROR)),
            Some(RuntimeKind::Node) => Ok((NPM_REGISTRY_MIRROR, PNPM_NODE_DIST_MIRROR)),
            Some(RuntimeKind::Shell) | None => Err(AppError::ValidationError(
                "请选择 Python 或 Node.js 依赖源".to_owned(),
            )),
        }
    }

    fn environment_override(key: &str) -> Option<String> {
        let variable = SETTINGS_REGISTRY.get(key)?.env_var?;
        std::env::var(variable)
            .ok()
            .filter(|value| !value.is_empty())
            .map(|_| variable.to_owned())
    }

    pub(super) fn validate_url(value: &str) -> Result<String> {
        let value = value.trim();
        let url = reqwest::Url::parse(value).map_err(|_| {
            AppError::ValidationError("请输入完整的 HTTP 或 HTTPS 源地址".to_owned())
        })?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || value.chars().any(char::is_whitespace)
            || url.fragment().is_some()
        {
            return Err(AppError::ValidationError(
                "源地址必须是有效的 HTTP 或 HTTPS 地址，不能包含空白或片段".to_owned(),
            ));
        }
        Ok(value.to_owned())
    }

    async fn load(settings: &SettingsService, env_type: &str) -> Result<Self> {
        let (package, runtime) = Self::keys(env_type)?;
        Ok(Self {
            package_url: settings.get(package).await?,
            runtime_url: settings.get(runtime).await?,
            package_override: Self::environment_override(package),
            runtime_override: Self::environment_override(runtime),
        })
    }

    async fn save(
        settings: &SettingsService,
        env_type: &str,
        payload: MirrorSettingsUpdate,
    ) -> Result<()> {
        let (package, runtime) = Self::keys(env_type)?;
        let package_url = Self::validate_url(&payload.package_url)?;
        let runtime_url = Self::validate_url(&payload.runtime_url)?;
        for (key, value) in [(package, &package_url), (runtime, &runtime_url)] {
            if let Some(variable) = Self::environment_override(key)
                && settings.get(key).await? != *value
            {
                return Err(AppError::ValidationError(format!(
                    "当前源由环境变量 {variable} 覆盖，请修改 Docker 配置并重新创建容器"
                )));
            }
        }
        settings
            .set_many(&[(package, &package_url), (runtime, &runtime_url)])
            .await
    }

    pub(super) async fn save_package_source(
        settings: &SettingsService,
        env_type: &str,
        value: &str,
    ) -> Result<()> {
        let (key, _) = Self::keys(env_type)?;
        let value = Self::validate_url(value)?;
        if let Some(variable) = Self::environment_override(key)
            && settings.get(key).await? != value
        {
            return Err(AppError::ValidationError(format!(
                "当前源由环境变量 {variable} 覆盖，请修改 Docker 配置并重新创建容器"
            )));
        }
        settings.set(key, &value, None).await
    }
}

#[utoipa::path(get, path = "/api/v1/environments/mirrors/{env_type}",
    params(("env_type" = String, Path, description = "python or node")),
    responses((status = 200, description = "Effective dependency and runtime sources", body = ApiResponse<MirrorSettings>)),
    tag = "Environments", security(("session_cookie" = [])))]
pub async fn get_mirror_settings(
    State(state): State<AppState>,
    Path(env_type): Path<String>,
) -> Result<ApiResponse<MirrorSettings>> {
    Ok(ApiResponse::success(
        MirrorSettings::load(&state.settings, &env_type).await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/environments/mirrors/{env_type}",
    params(("env_type" = String, Path, description = "python or node")),
    request_body = MirrorSettingsUpdate,
    responses((status = 200, description = "Save dependency and runtime sources atomically")),
    tag = "Environments", security(("session_cookie" = [])))]
pub async fn update_mirror_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Path(env_type): Path<String>,
    Json(payload): Json<MirrorSettingsUpdate>,
) -> Result<ApiResponse<()>> {
    MirrorSettings::save(&state.settings, &env_type, payload).await?;
    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "更新依赖源",
        "environment",
        Some(env_type),
        Some("更新了依赖包和运行时下载源".to_owned()),
        Some(ip),
    )
    .await;
    Ok(ApiResponse::success(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_sources_and_preserves_paths() {
        assert_eq!(
            MirrorSettings::validate_url(" https://example.com/simple/ ").unwrap(),
            "https://example.com/simple/"
        );
        for value in [
            "",
            "example.com",
            "file:///tmp/index",
            "https://example.com/a b",
            "https://example.com/#fragment",
        ] {
            assert!(MirrorSettings::validate_url(value).is_err(), "{value}");
        }
        assert!(MirrorSettings::keys("sh").is_err());
    }

    #[tokio::test]
    async fn saves_both_sources_without_changing_unrelated_settings() {
        use niupanel_core::event_bus::EventBus;
        use niupanel_core::settings::SettingsManager;
        use sea_orm::{ConnectionTrait, Database, Schema};
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let schema = Schema::new(db.get_database_backend());
        db.execute_raw(
            db.get_database_backend()
                .build(&schema.create_table_from_entity(niupanel_entity::settings::Entity)),
        )
        .await
        .unwrap();
        let service = SettingsService::new(db.clone(), EventBus::new());
        service
            .set("system.name", "Keep this name", None)
            .await
            .unwrap();
        MirrorSettings::save(
            &service,
            "python",
            MirrorSettingsUpdate {
                package_url: "https://pypi.org/simple".to_owned(),
                runtime_url: "https://example.com/python/".to_owned(),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            service.get(UV_PYPI_MIRROR).await.unwrap(),
            "https://pypi.org/simple"
        );
        assert_eq!(
            service.get(UV_PYTHON_MIRROR).await.unwrap(),
            "https://example.com/python/"
        );
        assert_eq!(service.get("system.name").await.unwrap(), "Keep this name");
        assert!(
            MirrorSettings::save(
                &service,
                "python",
                MirrorSettingsUpdate {
                    package_url: "https://changed.example/simple".to_owned(),
                    runtime_url: "invalid".to_owned()
                }
            )
            .await
            .is_err()
        );
        assert_eq!(
            SettingsManager::get(&db, UV_PYPI_MIRROR).await.unwrap(),
            "https://pypi.org/simple"
        );
    }
}
