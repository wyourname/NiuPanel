use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use niupanel_common::error::{AppError, Result};
use niupanel_entity::{git_repositories, tasks};

use crate::modules::git_sync::models::{DiscoveredTask, FileEntry, GitRepoRequest};

use super::GitService;

impl GitService {
    pub async fn list_repos(db: &DatabaseConnection) -> Result<Vec<git_repositories::Model>> {
        Self::check_and_migrate_legacy_config(db).await?;
        Self::check_and_migrate_repo_dirs(db).await?;

        git_repositories::Entity::find()
            .order_by_desc(git_repositories::Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }

    pub async fn get_repo(db: &DatabaseConnection, id: i32) -> Result<git_repositories::Model> {
        git_repositories::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(AppError::NotFound("Git repository not found".to_string()))
    }

    pub async fn list_repo_files(
        db: &DatabaseConnection,
        id: i32,
        sub_path: Option<String>,
    ) -> Result<Vec<FileEntry>> {
        let repo = Self::get_repo(db, id).await?;
        let repo_base_dir = Self::get_repo_dir(&repo);

        let target_path = match sub_path {
            Some(path) => {
                let path = path.trim_start_matches('/');
                if path.contains("..") {
                    return Err(AppError::ValidationError(
                        "Invalid path traversal".to_string(),
                    ));
                }
                repo_base_dir.join(path)
            }
            None => repo_base_dir.clone(),
        };

        if !target_path.exists() {
            return Ok(Vec::new());
        }

        let mut read_dir = tokio::fs::read_dir(target_path)
            .await
            .map_err(AppError::Io)?;
        let mut entries = Vec::new();

        while let Some(entry) = read_dir.next_entry().await.map_err(AppError::Io)? {
            let meta = entry.metadata().await.map_err(AppError::Io)?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name == ".git" {
                continue;
            }

            let full_path_buf = entry.path();
            let relative_path = full_path_buf
                .strip_prefix(&repo_base_dir)
                .unwrap_or(&full_path_buf)
                .to_string_lossy()
                .to_string();
            let relative_to_scripts = format!(
                "git/{}/{}",
                Self::get_repo_dir_name(&repo.repo_url),
                relative_path
            );

            entries.push(FileEntry {
                name: file_name,
                path: relative_path,
                full_path: relative_to_scripts,
                is_dir: meta.is_dir(),
                size: meta.len(),
            });
        }

        entries.sort_by(|a, b| {
            if a.is_dir == b.is_dir {
                a.name.cmp(&b.name)
            } else {
                b.is_dir.cmp(&a.is_dir)
            }
        });

        Ok(entries)
    }

    pub async fn import_tasks(
        db: &DatabaseConnection,
        id: i32,
        user_id: i32,
        tasks_to_import: Vec<DiscoveredTask>,
    ) -> Result<()> {
        let repo = Self::get_repo(db, id).await?;
        let dir_name = Self::get_repo_dir_name(&repo.repo_url);

        for task in tasks_to_import {
            let relative_to_scripts = format!("git/{}/{}", dir_name, task.file_path);
            let existing = tasks::Entity::find()
                .filter(tasks::Column::Path.eq(&relative_to_scripts))
                .one(db)
                .await?;

            let (env_type, env_version) = Self::infer_task_environment(&task.file_path);

            if let Some(existing_task) = existing {
                let mut active: tasks::ActiveModel = existing_task.into();
                if let Some(cron) = task.cron {
                    active.cron_schedule = Set(Some(cron));
                }
                active.update(db).await?;
                continue;
            }

            let active = tasks::ActiveModel {
                name: Set(task.name),
                path: Set(relative_to_scripts),
                command: Set(Some(String::new())),
                cron_schedule: Set(task.cron),
                status: Set(niupanel_entity::task_status::TaskStatus::Stopped),
                enabled: Set(true),
                user_id: Set(user_id),
                env_type: Set(env_type),
                env_version: Set(env_version),
                created_at: Set(Utc::now().into()),
                updated_at: Set(Utc::now().into()),
                ..Default::default()
            };
            active.insert(db).await?;
        }

        Ok(())
    }

    pub async fn create_repo(
        db: &DatabaseConnection,
        req: GitRepoRequest,
    ) -> Result<git_repositories::Model> {
        validate_repo_url(&req.repo_url)?;
        validate_repo_name(&req.name)?;
        validate_proxy_url(req.proxy_url.as_deref())?;
        let branch = req.branch.unwrap_or_else(|| "main".to_string());
        validate_branch(&branch)?;
        let active = git_repositories::ActiveModel {
            name: Set(req.name.trim().to_string()),
            repo_url: Set(req.repo_url.trim().to_string()),
            branch: Set(branch),
            auth_token: Set(normalize_optional_secret(req.auth_token)),
            proxy_url: Set(normalize_optional_secret(req.proxy_url)),
            auto_sync: Set(req.auto_sync.unwrap_or(false)),
            ..Default::default()
        };
        active.insert(db).await.map_err(Into::into)
    }

    pub async fn update_repo(
        db: &DatabaseConnection,
        id: i32,
        req: GitRepoRequest,
    ) -> Result<git_repositories::Model> {
        validate_repo_url(&req.repo_url)?;
        validate_repo_name(&req.name)?;
        validate_proxy_url(req.proxy_url.as_deref())?;
        if let Some(branch) = req.branch.as_deref() {
            validate_branch(branch)?;
        }
        let repo = Self::get_repo(db, id).await?;
        let mut active: git_repositories::ActiveModel = repo.into();
        active.name = Set(req.name.trim().to_string());
        active.repo_url = Set(req.repo_url.trim().to_string());
        if let Some(branch) = req.branch {
            active.branch = Set(branch);
        }
        if req.clear_auth_token {
            active.auth_token = Set(None);
        } else if let Some(token) = normalize_optional_secret(req.auth_token) {
            active.auth_token = Set(Some(token));
        }
        active.proxy_url = Set(normalize_optional_secret(req.proxy_url));
        if let Some(auto_sync) = req.auto_sync {
            active.auto_sync = Set(auto_sync);
        }
        active.updated_at = Set(Utc::now().into());
        active.update(db).await.map_err(Into::into)
    }

    pub async fn delete_repo(db: &DatabaseConnection, id: i32) -> Result<()> {
        let repo = Self::get_repo(db, id).await?;
        let repo_dir = Self::get_repo_dir(&repo);
        if repo_dir.exists() {
            let _ = tokio::fs::remove_dir_all(repo_dir).await;
        }

        git_repositories::Entity::delete_by_id(id).exec(db).await?;
        Ok(())
    }

    fn infer_task_environment(file_path: &str) -> (String, Option<String>) {
        if file_path.ends_with(".js") || file_path.ends_with(".ts") {
            ("node".to_string(), None)
        } else if file_path.ends_with(".py") {
            ("python".to_string(), Some("3.11".to_string()))
        } else {
            ("sh".to_string(), None)
        }
    }
}

fn normalize_optional_secret(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_repo_url(repo_url: &str) -> Result<()> {
    let trimmed = repo_url.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError(
            "Git 仓库地址不能为空".to_string(),
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(AppError::ValidationError(
            "Git 仓库地址不能包含控制字符".to_string(),
        ));
    }

    if let Ok(url) = reqwest::Url::parse(trimmed) {
        match url.scheme() {
            "http" | "https" => {
                if url.host_str().is_none()
                    || !url.username().is_empty()
                    || url.password().is_some()
                {
                    return Err(AppError::ValidationError(
                        "HTTP Git 地址必须包含主机且不能内嵌凭据，请使用访问令牌字段".to_string(),
                    ));
                }
            }
            "ssh" => {
                if url.host_str().is_none() || url.password().is_some() {
                    return Err(AppError::ValidationError(
                        "SSH Git 地址格式无效".to_string(),
                    ));
                }
            }
            _ => {
                return Err(AppError::ValidationError(
                    "Git 仓库地址只允许 http、https、ssh 或 user@host:path 格式".to_string(),
                ));
            }
        }
        return Ok(());
    }

    let Some((authority, remote_path)) = trimmed.split_once(':') else {
        return Err(AppError::ValidationError(
            "不允许本地文件路径作为 Git 仓库地址".to_string(),
        ));
    };
    let host = authority
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(authority);
    if host.is_empty()
        || authority.contains(['/', '\\'])
        || authority.chars().any(char::is_whitespace)
        || remote_path.is_empty()
        || remote_path.starts_with(['/', '-'])
        || remote_path.contains('\\')
        || remote_path.chars().any(char::is_whitespace)
    {
        return Err(AppError::ValidationError(
            "SCP 风格 Git 仓库地址格式无效".to_string(),
        ));
    }
    Ok(())
}

fn validate_repo_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
        return Err(AppError::ValidationError(
            "Git 仓库名称长度必须为 1 到 128 个字符".to_string(),
        ));
    }
    Ok(())
}

fn validate_branch(branch: &str) -> Result<()> {
    let branch = branch.trim();
    if branch.is_empty()
        || branch.len() > 255
        || branch.starts_with(['-', '/', '.'])
        || branch.ends_with(['/', '.'])
        || branch.ends_with(".lock")
        || branch.contains("..")
        || branch.contains("//")
        || branch.contains("@{")
        || branch == "@"
        || branch
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() || "~^:?*[\\\\".contains(ch))
    {
        return Err(AppError::ValidationError(
            "Git 分支名称格式无效".to_string(),
        ));
    }
    Ok(())
}

fn validate_proxy_url(proxy_url: Option<&str>) -> Result<()> {
    let Some(proxy_url) = proxy_url.map(str::trim).filter(|url| !url.is_empty()) else {
        return Ok(());
    };
    let url = reqwest::Url::parse(proxy_url)
        .map_err(|_| AppError::ValidationError("Git 代理地址格式无效".to_string()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppError::ValidationError(
            "Git 代理地址只允许不含凭据的 http/https URL".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_url_rejects_local_paths_options_and_embedded_credentials() {
        for invalid in [
            "../private-repo",
            "/srv/private-repo",
            "--upload-pack=/tmp/payload",
            "file:///srv/private-repo",
            "https://user:token@example.com/repo.git",
            r"C:\private\repo",
        ] {
            assert!(
                validate_repo_url(invalid).is_err(),
                "{invalid:?} should be rejected"
            );
        }
        assert!(validate_repo_url("https://example.com/org/repo.git").is_ok());
        assert!(validate_repo_url("ssh://git@example.com/org/repo.git").is_ok());
        assert!(validate_repo_url("git@example.com:org/repo.git").is_ok());
    }

    #[test]
    fn branch_validation_rejects_ref_and_option_injection() {
        for invalid in [
            "",
            "--help",
            "../main",
            "feature..name",
            "bad branch",
            "x.lock",
        ] {
            assert!(
                validate_branch(invalid).is_err(),
                "{invalid:?} should be rejected"
            );
        }
        assert!(validate_branch("main").is_ok());
        assert!(validate_branch("feature/safe-name").is_ok());
    }
}
