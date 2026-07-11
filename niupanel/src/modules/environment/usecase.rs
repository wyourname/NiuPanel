use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use niupanel_common::error::Result;
use niupanel_core::audit::service::AuditService;
use niupanel_core::settings::SettingsService;
use niupanel_core::task_manager::service::TaskManagerService;
use sea_orm::DatabaseConnection;

use super::models::{CreateEnvRequest, EnvironmentInfo, InstallPackageRequest};
use super::service::EnvironmentService;

#[derive(Clone)]
pub struct EnvironmentUseCase {
    db: DatabaseConnection,
    settings: SettingsService,
    task_manager: TaskManagerService,
}

impl EnvironmentUseCase {
    pub fn new(
        db: DatabaseConnection,
        settings: SettingsService,
        task_manager: TaskManagerService,
    ) -> Self {
        Self {
            db,
            settings,
            task_manager,
        }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(
            state.db.clone(),
            state.settings.clone(),
            state.task_manager.clone(),
        )
    }

    async fn log_user_action(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        action: &str,
        resource: &str,
        resource_id: Option<String>,
        details: Option<String>,
    ) {
        AuditService::log(
            &self.db,
            Some(user.id),
            "User",
            action,
            resource,
            resource_id,
            details,
            Some(ip),
        )
        .await;
    }

    pub async fn list_environments(&self) -> Result<Vec<EnvironmentInfo>> {
        EnvironmentService::list_environments(&self.db, &self.settings).await
    }

    pub async fn list_available_versions(&self) -> Result<String> {
        EnvironmentService::list_available_versions(&self.settings).await
    }

    pub async fn create_python_env(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        payload: CreateEnvRequest,
    ) -> Result<i32> {
        let version = payload.version.clone();
        let (job_id, venv_name) = EnvironmentService::create_python_env(
            &self.db,
            &self.settings,
            &self.task_manager,
            payload,
        )
        .await?;

        self.log_user_action(
            user,
            ip,
            "创建环境",
            "environment",
            Some(venv_name),
            Some(format!("开始创建 Python 环境: {}", version)),
        )
        .await;

        Ok(job_id)
    }

    pub async fn delete_python_env(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        name: String,
    ) -> Result<()> {
        EnvironmentService::delete_python_env(&self.db, &name).await?;

        self.log_user_action(
            user,
            ip,
            "删除环境",
            "environment",
            Some(name.clone()),
            Some(format!("删除了 Python 环境: {}", name)),
        )
        .await;

        Ok(())
    }

    pub async fn list_python_packages(&self, name: &str) -> Result<String> {
        EnvironmentService::list_python_packages(&self.settings, name).await
    }

    pub async fn install_python_packages(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        name: String,
        payload: InstallPackageRequest,
    ) -> Result<i32> {
        let packages = payload.packages.clone();
        let job_id = EnvironmentService::install_python_packages(
            &self.db,
            &self.settings,
            &self.task_manager,
            name.clone(),
            payload,
        )
        .await?;

        self.log_user_action(
            user,
            ip,
            "安装依赖包",
            "environment",
            Some(name.clone()),
            Some(format!("在环境 {} 中安装了依赖包: {:?}", name, packages)),
        )
        .await;

        Ok(job_id)
    }

    pub async fn uninstall_python_package(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        name: String,
        package: String,
    ) -> Result<i32> {
        let job_id = EnvironmentService::uninstall_python_package(
            &self.db,
            &self.task_manager,
            name.clone(),
            package.clone(),
        )
        .await?;

        self.log_user_action(
            user,
            ip,
            "卸载依赖包",
            "environment",
            Some(name.clone()),
            Some(format!("从环境 {} 中卸载了依赖包: {}", name, package)),
        )
        .await;

        Ok(job_id)
    }

    pub async fn list_node_packages(&self, name: &str) -> Result<String> {
        EnvironmentService::list_node_packages(&self.settings, name).await
    }

    pub async fn create_node_env(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        payload: CreateEnvRequest,
    ) -> Result<i32> {
        let version = payload.version.clone();
        let job_id = EnvironmentService::create_node_env(
            &self.db,
            &self.settings,
            &self.task_manager,
            payload,
        )
        .await?;

        self.log_user_action(
            user,
            ip,
            "安装Node版本",
            "environment",
            Some(version.clone()),
            Some(format!("开始下载 Node.js: {}", version)),
        )
        .await;

        Ok(job_id)
    }

    pub async fn delete_node_env(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        name: String,
    ) -> Result<()> {
        let version = EnvironmentService::delete_node_env(&self.db, &name).await?;

        self.log_user_action(
            user,
            ip,
            "卸载Node版本",
            "environment",
            Some(version.clone()),
            Some(format!("卸载了 Node.js 版本: {}", version)),
        )
        .await;

        Ok(())
    }

    pub async fn install_node_packages(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        name: String,
        payload: InstallPackageRequest,
    ) -> Result<i32> {
        let packages_str = payload.packages.join(", ");
        let job_id = EnvironmentService::install_node_packages(
            &self.db,
            &self.settings,
            &self.task_manager,
            name,
            payload,
        )
        .await?;

        self.log_user_action(
            user,
            ip,
            "安装Node版本依赖",
            "environment",
            None,
            Some(format!("安装 Node 版本共享依赖包: {}", packages_str)),
        )
        .await;

        Ok(job_id)
    }

    pub async fn uninstall_node_package(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        name: String,
        package: String,
    ) -> Result<i32> {
        let job_id = EnvironmentService::uninstall_node_package(
            &self.db,
            &self.settings,
            &self.task_manager,
            name,
            package.clone(),
        )
        .await?;

        self.log_user_action(
            user,
            ip,
            "卸载Node版本依赖",
            "environment",
            None,
            Some(format!("卸载 Node 版本共享依赖包: {}", package)),
        )
        .await;

        Ok(job_id)
    }

    pub async fn set_node_default(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        name: String,
    ) -> Result<()> {
        let version = EnvironmentService::set_node_default(&self.settings, &name).await?;

        self.log_user_action(
            user,
            ip,
            "切换Node系统默认版本",
            "environment",
            Some(version.clone()),
            Some(format!("将系统默认 Node.js 版本切换为: {}", version)),
        )
        .await;

        Ok(())
    }

    pub async fn list_shell_packages(&self) -> Result<String> {
        EnvironmentService::list_shell_packages().await
    }

    pub async fn install_shell_packages(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        payload: InstallPackageRequest,
    ) -> Result<i32> {
        let packages = payload.packages.clone();
        let job_id =
            EnvironmentService::install_shell_packages(&self.db, &self.task_manager, payload)
                .await?;

        self.log_user_action(
            user,
            ip,
            "安装系统包",
            "shell",
            None,
            Some(format!("安装了系统软件包: {:?}", packages)),
        )
        .await;

        Ok(job_id)
    }

    pub async fn uninstall_shell_package(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        package: String,
    ) -> Result<()> {
        EnvironmentService::uninstall_shell_package(&self.db, &package).await?;

        self.log_user_action(
            user,
            ip,
            "卸载系统包",
            "shell",
            None,
            Some(format!("卸载了系统软件包: {}", package)),
        )
        .await;

        Ok(())
    }

    pub async fn set_mirror_source(
        &self,
        user: &AuthenticatedUser,
        ip: String,
        env_type: String,
        mirror_url: String,
    ) -> Result<()> {
        EnvironmentService::set_mirror_source(&env_type, &mirror_url).await?;

        self.log_user_action(
            user,
            ip,
            "设置镜像源",
            &env_type,
            None,
            Some(format!("将 {} 的镜像源设置为 {}", env_type, mirror_url)),
        )
        .await;

        Ok(())
    }
}
