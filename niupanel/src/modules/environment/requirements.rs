use niupanel_common::error::Result;
use niupanel_core::environment::merged_requirements;
use niupanel_entity::environments;
use sea_orm::{ActiveModelTrait, DatabaseConnection, IntoActiveModel, Set};

pub(super) const VERSIONED_PACKAGE_SEPARATORS: &[char] = &['=', '>', '<', '~', '!', '[', ';'];
pub(super) const NODE_PACKAGE_SEPARATORS: &[char] = &['@'];
pub(super) const SHELL_PACKAGE_SEPARATORS: &[char] = &['@', '='];

pub(super) async fn merge_or_insert_requirements(
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

pub(super) async fn merge_requirements(
    db: &DatabaseConnection,
    env_model: environments::Model,
    packages: &[String],
) -> Result<()> {
    let requirements = merged_requirements(
        env_model.requirements.as_deref(),
        packages,
        &env_model.env_type,
    );
    update_requirements(db, env_model, requirements).await
}

pub(super) async fn remove_requirement(
    db: &DatabaseConnection,
    env_model: environments::Model,
    package: &str,
    separators: &[char],
) -> Result<()> {
    let requirements =
        requirements_without_package(env_model.requirements.as_deref(), package, separators);
    update_requirements(db, env_model, requirements).await
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

async fn update_requirements(
    db: &DatabaseConnection,
    env_model: environments::Model,
    requirements: String,
) -> Result<()> {
    let mut active = env_model.into_active_model();
    active.requirements = Set(Some(requirements));
    active.updated_at = Set(chrono::Utc::now().into());
    active.update(db).await?;
    Ok(())
}

fn requirements_without_package(
    existing_requirements: Option<&str>,
    package: &str,
    separators: &[char],
) -> String {
    parse_requirements(existing_requirements)
        .into_iter()
        .filter(|requirement| {
            let requirement_name = package_base_name(requirement, separators);
            if separators == VERSIONED_PACKAGE_SEPARATORS {
                !requirement_name
                    .split(['-', '_', '.'])
                    .filter(|part| !part.is_empty())
                    .map(str::to_ascii_lowercase)
                    .eq(package
                        .split(['-', '_', '.'])
                        .filter(|part| !part.is_empty())
                        .map(str::to_ascii_lowercase))
            } else {
                requirement_name != package && requirement != package
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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

fn package_base_name<'a>(package: &'a str, separators: &[char]) -> &'a str {
    let start = usize::from(package.starts_with('@'));
    let end = package[start..]
        .find(|character| separators.contains(&character))
        .map(|index| start + index)
        .unwrap_or(package.len());
    package[..end].trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_versioned_scoped_node_packages_and_python_extras() {
        assert_eq!(
            requirements_without_package(
                Some("@scope/pkg@^2\nother@1"),
                "@scope/pkg",
                NODE_PACKAGE_SEPARATORS
            ),
            "other@1"
        );
        assert_eq!(
            requirements_without_package(
                Some("requests[socks]>=2\nflask"),
                "requests",
                VERSIONED_PACKAGE_SEPARATORS
            ),
            "flask"
        );
        assert_eq!(
            requirements_without_package(
                Some("Some_Pkg[extra]>=2\nflask"),
                "some-pkg",
                VERSIONED_PACKAGE_SEPARATORS
            ),
            "flask"
        );
    }

    #[test]
    fn merged_requirements_trims_existing_and_appends_unique_packages() {
        let packages = vec!["requests".to_string(), "uvicorn".to_string()];

        let merged = merged_requirements(Some(" requests\n\nflask "), &packages, "python");

        assert_eq!(merged, "flask\nrequests\nuvicorn");
    }

    #[test]
    fn requirements_without_package_removes_versioned_package_by_base_name() {
        let requirements = Some("requests==2.31.0\nflask>=3\nuvicorn");

        let updated =
            requirements_without_package(requirements, "requests", VERSIONED_PACKAGE_SEPARATORS);

        assert_eq!(updated, "flask>=3\nuvicorn");
    }

    #[test]
    fn requirements_without_package_removes_shell_package_by_base_name() {
        let requirements = Some("curl=8.0\nnodejs@20\nvim");

        let updated =
            requirements_without_package(requirements, "nodejs", SHELL_PACKAGE_SEPARATORS);

        assert_eq!(updated, "curl=8.0\nvim");
    }
}
