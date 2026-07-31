use std::process::Output;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use niupanel_common::error::{AppError, Result};
use niupanel_core::sys::tools::ToolService;
use niupanel_entity::git_repositories;

use crate::modules::git_sync::models::SyncResult;

use super::GitService;

impl GitService {
    fn git_bin() -> Result<std::path::PathBuf> {
        ToolService::ensure_git()
    }

    pub async fn sync_repo(db: &DatabaseConnection, id: i32) -> Result<SyncResult> {
        let repo = Self::get_repo(db, id).await?;
        let repo_dir = Self::get_repo_dir(&repo);

        let git_base_dir = Self::git_base_dir();
        if !git_base_dir.exists() {
            tokio::fs::create_dir_all(&git_base_dir)
                .await
                .map_err(AppError::Io)?;
        }
        if !repo_dir.exists() {
            tokio::fs::create_dir_all(&repo_dir)
                .await
                .map_err(AppError::Io)?;
        }

        let remote_url = repo.repo_url.clone();
        let sync_result = if !repo_dir.join(".git").exists() {
            Self::clone_repo(db, &repo, &repo_dir, &remote_url).await?
        } else {
            Self::pull_repo(db, &repo, &repo_dir).await?
        };

        let current_commit = Self::read_current_commit(&repo_dir).await;
        Self::update_sync_status(
            db,
            repo.id,
            "Success",
            Some(sync_result.message.clone()),
            Some(current_commit),
        )
        .await?;

        Ok(sync_result)
    }

    pub async fn start_auto_sync_scheduler(db: DatabaseConnection) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
            loop {
                interval.tick().await;
                niupanel_common::info!("开始执行 Git 自动同步任务...");

                let repos = match git_repositories::Entity::find()
                    .filter(git_repositories::Column::AutoSync.eq(true))
                    .all(&db)
                    .await
                {
                    Ok(repos) => repos,
                    Err(error) => {
                        niupanel_common::error!("获取自动同步仓库列表失败: {}", error);
                        continue;
                    }
                };

                for repo in repos {
                    niupanel_common::info!("正在自动同步仓库: {} ({})", repo.name, repo.repo_url);
                    if let Err(error) = Self::sync_repo(&db, repo.id).await {
                        niupanel_common::warn!("仓库 {} 自动同步失败: {}", repo.name, error);
                    }
                }
            }
        });
    }

    async fn clone_repo(
        db: &DatabaseConnection,
        repo: &git_repositories::Model,
        repo_dir: &std::path::Path,
        remote_url: &str,
    ) -> Result<SyncResult> {
        let envs = Self::git_auth_env(repo);
        let args = ["clone", "-b", repo.branch.as_str(), "--", remote_url, "."];
        Self::run_git_command(
            db,
            repo.id,
            "git clone",
            &args,
            repo_dir,
            Some(&envs),
            "Clone failed",
        )
        .await?;

        Ok(SyncResult {
            success: true,
            message: "Repository cloned successfully".to_string(),
            changes: vec!["Full repository cloned".to_string()],
        })
    }

    async fn pull_repo(
        db: &DatabaseConnection,
        repo: &git_repositories::Model,
        repo_dir: &std::path::Path,
    ) -> Result<SyncResult> {
        let envs = Self::git_auth_env(repo);
        let remote_args = ["remote", "set-url", "origin", repo.repo_url.as_str()];
        Self::run_git_command(
            db,
            repo.id,
            "git remote set-url",
            &remote_args,
            repo_dir,
            None,
            "Sanitizing remote URL failed",
        )
        .await?;
        let fetch_args = ["fetch", "origin", repo.branch.as_str()];
        Self::run_git_command(
            db,
            repo.id,
            "git fetch",
            &fetch_args,
            repo_dir,
            Some(&envs),
            "Fetch failed",
        )
        .await?;

        let reset_target = format!("origin/{}", repo.branch);
        let reset_args = ["reset", "--hard", reset_target.as_str()];
        Self::run_git_command(
            db,
            repo.id,
            "git reset",
            &reset_args,
            repo_dir,
            Some(&envs),
            "Reset failed",
        )
        .await?;

        Ok(SyncResult {
            success: true,
            message: "Sync completed successfully".to_string(),
            changes: vec!["Repository updated from remote".to_string()],
        })
    }

    async fn run_git_command(
        db: &DatabaseConnection,
        repo_id: i32,
        command_name: &'static str,
        args: &[&str],
        repo_dir: &std::path::Path,
        envs: Option<&std::collections::HashMap<String, String>>,
        failure_label: &'static str,
    ) -> Result<Output> {
        let output =
            ToolService::capture_output(&Self::git_bin()?, args, Some(repo_dir), envs).await?;
        if output.status.success() {
            return Ok(output);
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Self::update_sync_status(
            db,
            repo_id,
            "Failed",
            Some(format!("{failure_label}: {stderr}")),
            None,
        )
        .await
        .ok();
        Err(AppError::ProcessFailed {
            command: command_name.into(),
            exit_code: output.status.code(),
            stderr,
        })
    }

    async fn update_sync_status(
        db: &DatabaseConnection,
        id: i32,
        status: &str,
        message: Option<String>,
        commit: Option<String>,
    ) -> Result<()> {
        let repo = Self::get_repo(db, id).await?;
        let mut active: git_repositories::ActiveModel = repo.into();
        active.last_sync_status = Set(Some(status.to_string()));
        active.last_sync_at = Set(Some(Utc::now().into()));
        active.last_sync_message = Set(message);
        if let Some(commit) = commit {
            active.current_commit = Set(Some(commit));
        }
        active.update(db).await?;
        Ok(())
    }

    fn git_auth_env(repo: &git_repositories::Model) -> std::collections::HashMap<String, String> {
        let mut envs = Self::proxy_env(repo.proxy_url.as_deref());
        envs.insert("GIT_TERMINAL_PROMPT".to_string(), "0".to_string());
        if let Some(token) = repo.auth_token.as_deref().filter(|token| !token.is_empty()) {
            let credentials =
                openssl::base64::encode_block(format!("x-access-token:{token}").as_bytes());
            envs.insert("GIT_CONFIG_COUNT".to_string(), "1".to_string());
            envs.insert(
                "GIT_CONFIG_KEY_0".to_string(),
                "http.extraHeader".to_string(),
            );
            envs.insert(
                "GIT_CONFIG_VALUE_0".to_string(),
                format!("Authorization: Basic {credentials}"),
            );
        }
        envs
    }

    async fn read_current_commit(repo_dir: &std::path::Path) -> String {
        let git_bin = match Self::git_bin() {
            Ok(path) => path,
            Err(_) => return "unknown".to_string(),
        };
        match ToolService::capture_output(
            &git_bin,
            &["rev-parse", "--short", "HEAD"],
            Some(repo_dir),
            None,
        )
        .await
        {
            Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
            Err(_) => "unknown".to_string(),
        }
    }
}
