use crate::runtime::RuntimeManager;
use crate::script::interpreter::node::NodeEnvironment;
use crate::settings::{
    NPM_REGISTRY_MIRROR, PNPM_NODE_DIST_MIRROR, SettingsService, UV_PYPI_MIRROR, UV_PYTHON_MIRROR,
};
use crate::task_manager::service::TaskManagerService;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_entity::environments;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use std::collections::HashMap;
use std::path::PathBuf;

const PYTHON_ENV_PREFIX: &str = "venv_";
const NODE_ENV_TYPE: &str = "node";
const PYTHON_ENV_TYPE: &str = "python";
const PNPM_NODE_DIST_MIRROR_ENV: &str = "PNPM_NODE_DIST_MIRROR";
const NPM_CONFIG_REGISTRY_ENV: &str = "npm_config_registry";
const UV_INDEX_URL_ENV: &str = "UV_INDEX_URL";
const UV_PYTHON_INSTALL_MIRROR_ENV: &str = "UV_PYTHON_INSTALL_MIRROR";
const DEFAULT_NODE_DIST_MIRROR: &str = "https://mirrors.ustc.edu.cn/node/";

#[derive(Debug, Clone)]
pub struct InstallRuntimePackagesRequest {
    pub env_name: String,
    pub packages: Vec<String>,
}

pub async fn install_node_packages(
    db: &DatabaseConnection,
    settings: &SettingsService,
    task_manager: &TaskManagerService,
    request: InstallRuntimePackagesRequest,
) -> Result<i32> {
    let packages = request.packages;
    let version = normalize_node_version(&request.env_name)?;
    let db = db.clone();
    let name = request.env_name;
    let mirrors = node_install_mirrors(settings).await;
    task_manager
        .submit_system_task_with_metadata(
            "Install Node packages".to_string(),
            Some(serde_json::json!({
                "kind": "environment-packages", "env_type": "node", "env_name": name,
                "operation": "install", "packages": packages,
            })),
            move |tx| async move {
                let env = RuntimeManager::open_node_environment(Some(&version), Some(mirrors))?;
                env.install_packages(&packages, tx.clone()).await?;
                let existing_env = environments::Entity::find()
                    .filter(environments::Column::Name.eq(&name))
                    .filter(environments::Column::EnvType.eq(NODE_ENV_TYPE))
                    .one(&db)
                    .await?;
                merge_or_insert_requirements(
                    &db,
                    existing_env,
                    &name,
                    NODE_ENV_TYPE,
                    &version,
                    &packages,
                )
                .await?;
                let _ = tx.send("Installation completed successfully.".to_string());
                Ok(())
            },
        )
        .await
}

pub async fn install_python_packages(
    db: &DatabaseConnection,
    settings: &SettingsService,
    task_manager: &TaskManagerService,
    request: InstallRuntimePackagesRequest,
) -> Result<i32> {
    let python_env = RuntimeEnvKind::Python;
    let venv_path = python_env.env_path(&request.env_name);
    if !venv_path.exists() {
        return Err(AppError::NotFound("Environment not found".to_string()));
    }

    let version = python_env.record_version(&request.env_name);
    let db = db.clone();
    let mirrors = python_install_and_index_mirrors(settings).await;
    let env =
        RuntimeManager::open_python_environment(venv_path, None, Some(mirrors.clone())).await?;
    let requirements = request.packages.join("\n");
    let name = request.env_name;
    task_manager
        .submit_system_task_with_metadata(
            format!("Install packages in {}", name),
            Some(serde_json::json!({"kind": "environment-packages", "env_type": "python", "env_name": name, "operation": "install", "packages": request.packages})),
            move |tx| async move {
                env.install_requirements(&requirements, false, tx.clone())
                    .await?;
                let existing_env = environments::Entity::find()
                    .filter(environments::Column::Name.eq(&name))
                    .filter(environments::Column::EnvType.eq(PYTHON_ENV_TYPE))
                    .one(&db)
                    .await?;
                merge_or_insert_requirements(
                    &db,
                    existing_env,
                    &name,
                    PYTHON_ENV_TYPE,
                    &version,
                    &request.packages,
                )
                .await?;
                let _ = tx.send("Installation completed successfully.".to_string());
                Ok(())
            },
        )
        .await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeEnvKind {
    Python,
    Node,
}

impl RuntimeEnvKind {
    fn env_type(self) -> &'static str {
        match self {
            Self::Python => PYTHON_ENV_TYPE,
            Self::Node => NODE_ENV_TYPE,
        }
    }

    fn runtime_dir(self) -> PathBuf {
        PathBuf::from(&Config::global().runtimes_dir).join(self.env_type())
    }

    fn env_path(self, name: &str) -> PathBuf {
        self.runtime_dir().join(name)
    }

    fn version_from_name(self, name: &str) -> Option<String> {
        match self {
            Self::Python => name.strip_prefix(PYTHON_ENV_PREFIX).map(str::to_string),
            Self::Node => Some(
                name.strip_prefix(PYTHON_ENV_PREFIX)
                    .unwrap_or(name)
                    .split(' ')
                    .next()
                    .unwrap_or(name)
                    .to_string(),
            ),
        }
    }

    fn record_version(self, name: &str) -> String {
        self.version_from_name(name)
            .unwrap_or_else(|| name.to_string())
    }
}

fn normalize_node_version(name: &str) -> Result<String> {
    RuntimeEnvKind::Node
        .version_from_name(name)
        .and_then(|version| NodeEnvironment::normalize_version(&version))
        .ok_or_else(|| AppError::ValidationError("Node version must be specified".to_string()))
}

async fn merge_or_insert_requirements(
    db: &DatabaseConnection,
    existing_env: Option<environments::Model>,
    name: &str,
    env_type: &str,
    version: &str,
    packages: &[String],
) -> Result<()> {
    if let Some(env_model) = existing_env {
        merge_requirements(db, env_model, packages).await
    } else {
        insert_requirements(db, name, env_type, version, packages).await
    }
}

async fn merge_requirements(
    db: &DatabaseConnection,
    env_model: environments::Model,
    packages: &[String],
) -> Result<()> {
    let requirements = merged_requirements(
        env_model.requirements.as_deref(),
        packages,
        &env_model.env_type,
    );
    let mut active = env_model.into_active_model();
    active.requirements = Set(Some(requirements));
    active.updated_at = Set(chrono::Utc::now().into());
    active.update(db).await?;
    Ok(())
}

async fn insert_requirements(
    db: &DatabaseConnection,
    name: &str,
    env_type: &str,
    version: &str,
    packages: &[String],
) -> Result<()> {
    let active = environments::ActiveModel {
        name: Set(name.to_string()),
        env_type: Set(env_type.to_string()),
        version: Set(version.to_string()),
        requirements: Set(Some(merged_requirements(None, packages, env_type))),
        ..Default::default()
    };
    active.insert(db).await?;
    Ok(())
}

/// Keep the latest installed constraint for a package when recording restore dependencies.
pub fn merged_requirements(
    existing_requirements: Option<&str>,
    packages: &[String],
    env_type: &str,
) -> String {
    let mut requirements = parse_requirements(existing_requirements);
    for package in packages {
        let key = requirement_key(package, env_type);
        requirements.retain(|entry| requirement_key(entry, env_type) != key);
        requirements.push(package.trim().to_owned());
    }
    requirements.join("\n")
}

fn requirement_key(package: &str, env_type: &str) -> String {
    let package = package.trim();
    let start = usize::from(package.starts_with('@'));
    let separators: &[char] = match env_type {
        "python" => &['=', '>', '<', '~', '!', '[', ';', '@', ' '],
        "node" => &['@'],
        _ => &['=', '@'],
    };
    let end = package[start..]
        .find(separators)
        .map_or(package.len(), |index| start + index);
    let name = package[..end].trim();
    if env_type == "python" {
        // PEP 503 normalizes names; distinct environment markers remain separate requirements.
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return package.to_owned();
        }
        let normalized = name
            .split(['-', '_', '.'])
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-")
            .to_ascii_lowercase();
        let marker = package
            .split_once(';')
            .map_or("", |(_, marker)| marker.trim());
        format!("{normalized};{marker}")
    } else {
        name.to_owned()
    }
}

fn parse_requirements(requirements: Option<&str>) -> Vec<String> {
    requirements
        .map(|source| {
            source
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

async fn python_install_and_index_mirrors(settings: &SettingsService) -> HashMap<String, String> {
    let mut mirrors = HashMap::new();
    insert_setting_mirror(
        settings,
        &mut mirrors,
        UV_PYTHON_MIRROR,
        UV_PYTHON_INSTALL_MIRROR_ENV,
    )
    .await;
    insert_setting_mirror(settings, &mut mirrors, UV_PYPI_MIRROR, UV_INDEX_URL_ENV).await;
    mirrors
}

async fn node_install_mirrors(settings: &SettingsService) -> HashMap<String, String> {
    let mut mirrors = HashMap::new();
    let node_mirror = settings
        .get(PNPM_NODE_DIST_MIRROR)
        .await
        .unwrap_or_else(|_| DEFAULT_NODE_DIST_MIRROR.to_string());
    if !node_mirror.is_empty() {
        mirrors.insert(PNPM_NODE_DIST_MIRROR_ENV.to_string(), node_mirror);
    }
    insert_setting_mirror(
        settings,
        &mut mirrors,
        NPM_REGISTRY_MIRROR,
        NPM_CONFIG_REGISTRY_ENV,
    )
    .await;
    mirrors
}

async fn insert_setting_mirror(
    settings: &SettingsService,
    mirrors: &mut HashMap<String, String>,
    setting_key: &str,
    runtime_env_key: &str,
) {
    if let Ok(mirror) = settings.get(setting_key).await
        && !mirror.is_empty()
    {
        mirrors.insert(runtime_env_key.to_string(), mirror);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_requirements_trims_existing_and_appends_unique_packages() {
        let packages = vec!["requests".to_string(), "uvicorn".to_string()];

        let merged = merged_requirements(Some(" requests\n\nflask "), &packages, "python");

        assert_eq!(merged, "flask\nrequests\nuvicorn");
    }

    #[test]
    fn installed_versions_replace_old_restore_constraints() {
        assert_eq!(
            merged_requirements(
                Some("@scope/pkg@1\nother@2"),
                &["@scope/pkg@3".into()],
                "node"
            ),
            "other@2\n@scope/pkg@3"
        );
        assert_eq!(
            merged_requirements(
                Some("Some_Pkg[extra]==1\nflask"),
                &["some-pkg>=2".into()],
                "python"
            ),
            "flask\nsome-pkg>=2"
        );
        assert_eq!(
            merged_requirements(Some("curl=1\ngit"), &["curl=2".into()], "sh"),
            "git\ncurl=2"
        );
        assert_eq!(
            merged_requirements(
                Some("foo==1; python_version < '3.10'"),
                &["foo==2; python_version >= '3.10'".into()],
                "python"
            ),
            "foo==1; python_version < '3.10'\nfoo==2; python_version >= '3.10'"
        );
    }

    #[test]
    fn node_version_from_name_removes_legacy_prefix_and_status_suffix() {
        assert_eq!(
            RuntimeEnvKind::Node.record_version("venv_20.11.1 default"),
            "20.11.1"
        );
        assert_eq!(
            RuntimeEnvKind::Node.record_version("20.11.1 default"),
            "20.11.1"
        );
    }

    #[test]
    fn python_record_version_falls_back_to_name_without_prefix() {
        assert_eq!(
            RuntimeEnvKind::Python.record_version("custom-python"),
            "custom-python"
        );
    }
}
