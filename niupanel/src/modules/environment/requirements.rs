use niupanel_common::error::Result;
use niupanel_entity::environments;
use sea_orm::{ActiveModelTrait, DatabaseConnection, IntoActiveModel, Set};

pub(super) const VERSIONED_PACKAGE_SEPARATORS: &[char] = &['=', '>', '<', '~'];
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
    let requirements = merged_requirements(env_model.requirements.as_deref(), packages);
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
        requirements: Set(Some(packages.join("\n"))),
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

fn merged_requirements(existing_requirements: Option<&str>, packages: &[String]) -> String {
    let mut requirements = parse_requirements(existing_requirements);
    for package in packages {
        if !requirements.contains(package) {
            requirements.push(package.clone());
        }
    }
    requirements.join("\n")
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
            requirement_name != package && requirement != package
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
    package
        .split(|character| separators.contains(&character))
        .next()
        .unwrap_or(package)
        .trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_requirements_trims_existing_and_appends_unique_packages() {
        let packages = vec!["requests".to_string(), "uvicorn".to_string()];

        let merged = merged_requirements(Some(" requests\n\nflask "), &packages);

        assert_eq!(merged, "requests\nflask\nuvicorn");
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
