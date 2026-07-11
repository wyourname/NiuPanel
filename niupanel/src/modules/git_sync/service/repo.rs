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
        let active = git_repositories::ActiveModel {
            name: Set(req.name),
            repo_url: Set(req.repo_url),
            branch: Set(req.branch.unwrap_or_else(|| "main".to_string())),
            auth_token: Set(req.auth_token),
            proxy_url: Set(req.proxy_url),
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
        let repo = Self::get_repo(db, id).await?;
        let mut active: git_repositories::ActiveModel = repo.into();
        active.name = Set(req.name);
        active.repo_url = Set(req.repo_url);
        if let Some(branch) = req.branch {
            active.branch = Set(branch);
        }
        active.auth_token = Set(req.auth_token);
        active.proxy_url = Set(req.proxy_url);
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
