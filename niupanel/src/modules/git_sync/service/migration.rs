use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set, TransactionTrait,
};

use niupanel_common::config::Config;
use niupanel_common::error::Result;
use niupanel_core::settings::SettingsManager;
use niupanel_entity::{git_repositories, settings, tasks};

use super::{
    GitService, KEY_GIT_AUTH_TOKEN, KEY_GIT_AUTO_SYNC, KEY_GIT_BRANCH, KEY_GIT_PROXY,
    KEY_GIT_REPO_URL,
};

impl GitService {
    pub(crate) async fn check_and_migrate_legacy_config(db: &DatabaseConnection) -> Result<()> {
        if git_repositories::Entity::find().count(db).await? > 0 {
            return Ok(());
        }

        let repo_url = SettingsManager::get(db, KEY_GIT_REPO_URL)
            .await
            .unwrap_or_default();
        if repo_url.is_empty() {
            return Ok(());
        }

        let branch = SettingsManager::get(db, KEY_GIT_BRANCH)
            .await
            .unwrap_or_else(|_| "main".to_string());
        let auth_token = SettingsManager::get(db, KEY_GIT_AUTH_TOKEN).await.ok();
        let proxy_url = SettingsManager::get(db, KEY_GIT_PROXY).await.ok();
        let auto_sync = SettingsManager::get(db, KEY_GIT_AUTO_SYNC)
            .await
            .unwrap_or_default()
            == "true";

        let repo = git_repositories::ActiveModel {
            name: Set("Legacy Repository".to_string()),
            repo_url: Set(repo_url),
            branch: Set(branch),
            auth_token: Set(auth_token),
            proxy_url: Set(proxy_url),
            auto_sync: Set(auto_sync),
            ..Default::default()
        };
        repo.insert(db).await?;

        let transaction = db.begin().await?;
        for key in [
            KEY_GIT_REPO_URL,
            KEY_GIT_BRANCH,
            KEY_GIT_AUTH_TOKEN,
            KEY_GIT_PROXY,
            KEY_GIT_AUTO_SYNC,
        ] {
            let _ = settings::Entity::delete_by_id(key).exec(&transaction).await;
        }
        transaction.commit().await?;

        Ok(())
    }

    pub(crate) async fn check_and_migrate_repo_dirs(db: &DatabaseConnection) -> Result<()> {
        for repo in git_repositories::Entity::find().all(db).await? {
            let new_dir_name = Self::get_repo_dir_name(&repo.repo_url);
            let old_dir_name = repo.id.to_string();
            if new_dir_name == old_dir_name {
                continue;
            }

            let old_dir = Config::global().scripts_dir.join("git").join(&old_dir_name);
            let new_dir = Config::global().scripts_dir.join("git").join(&new_dir_name);
            if !old_dir.exists() || new_dir.exists() {
                continue;
            }

            niupanel_common::info!(
                "正在迁移 Git 仓库目录: {} -> {}",
                old_dir_name,
                new_dir_name
            );
            let _ = tokio::fs::rename(&old_dir, &new_dir).await;

            let old_prefix = format!("git/{old_dir_name}/");
            let new_prefix = format!("git/{new_dir_name}/");
            for task in tasks::Entity::find()
                .filter(tasks::Column::Path.starts_with(&old_prefix))
                .all(db)
                .await?
            {
                let mut active: tasks::ActiveModel = task.into();
                let new_path = active.path.as_ref().replacen(&old_prefix, &new_prefix, 1);
                active.path = Set(new_path);
                active.update(db).await?;
            }
        }

        Ok(())
    }
}
