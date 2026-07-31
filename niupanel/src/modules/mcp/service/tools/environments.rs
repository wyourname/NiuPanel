use super::super::*;
use rmcp::{tool, tool_router};

#[tool_router(router = environment_tool_router, vis = "pub(crate)")]
impl PanelMcpServer {
    #[tool(description = "List Python, Node.js and shell environments available in NiuPanel")]
    async fn environments_list(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<EnvironmentListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::EnvList)?;
        let environments = EnvironmentUseCase::from_state(&self.state)
            .list_environments()
            .await
            .map_err(tool_error)?;
        self.audit(&user, "environments_list", None).await;
        Ok(Json(EnvironmentListOutput {
            items: environments
                .into_iter()
                .map(|environment| EnvironmentOutput {
                    name: environment.name,
                    version: environment.version,
                    env_type: environment.env_type,
                    is_installed: environment.is_installed,
                    recorded_packages: environment.recorded_packages,
                })
                .collect(),
        }))
    }

    #[tool(description = "Get one NiuPanel runtime environment by exact name")]
    async fn environments_get(
        &self,
        Parameters(EnvironmentGetParams { name }): Parameters<EnvironmentGetParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<EnvironmentOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::EnvRead)?;
        let environment = EnvironmentUseCase::from_state(&self.state)
            .list_environments()
            .await
            .map_err(tool_error)?
            .into_iter()
            .find(|environment| environment.name == name)
            .ok_or_else(|| ErrorData::invalid_params("Environment not found", None))?;
        self.audit(&user, "environments_get", Some(name)).await;
        Ok(Json(EnvironmentOutput {
            name: environment.name,
            version: environment.version,
            env_type: environment.env_type,
            is_installed: environment.is_installed,
            recorded_packages: environment.recorded_packages,
        }))
    }

    #[tool(description = "List Python and Node.js versions available for installation")]
    async fn environments_versions(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<EnvironmentVersionsOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::EnvRead)?;
        let manifest = EnvironmentUseCase::from_state(&self.state)
            .list_available_versions()
            .await
            .map_err(tool_error)?;
        self.audit(&user, "environments_versions", None).await;
        Ok(Json(EnvironmentVersionsOutput { manifest }))
    }

    #[tool(description = "List packages installed in a Python, Node.js or shell environment")]
    async fn environments_packages_list(
        &self,
        Parameters(params): Parameters<EnvironmentPackagesParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<EnvironmentPackagesOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::EnvRead)?;
        let usecase = EnvironmentUseCase::from_state(&self.state);
        let packages = match params.runtime {
            PackageRuntime::Python => {
                usecase
                    .list_python_packages(required_environment_name(&params.name)?)
                    .await
            }
            PackageRuntime::Node => {
                usecase
                    .list_node_packages(required_environment_name(&params.name)?)
                    .await
            }
            PackageRuntime::Shell => usecase.list_shell_packages().await,
        }
        .map_err(tool_error)?;
        let resource_id = params.name.clone();
        self.audit(&user, "environments_packages_list", resource_id)
            .await;
        Ok(Json(EnvironmentPackagesOutput {
            runtime: params.runtime,
            name: params.name,
            packages,
        }))
    }

    #[tool(description = "Create a Python environment or install a Node.js version")]
    async fn environments_create(
        &self,
        Parameters(params): Parameters<EnvironmentCreateParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<EnvironmentJobOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::EnvCreate)?;
        let version = required_value(params.version, "version")?;
        let usecase = EnvironmentUseCase::from_state(&self.state);
        let job_id = match params.runtime {
            ManagedEnvironmentRuntime::Python => {
                usecase
                    .create_python_env(&user, "mcp".to_string(), CreateEnvRequest { version })
                    .await
            }
            ManagedEnvironmentRuntime::Node => {
                usecase
                    .create_node_env(&user, "mcp".to_string(), CreateEnvRequest { version })
                    .await
            }
        }
        .map_err(tool_error)?;
        self.audit(&user, "environments_create", Some(job_id.to_string()))
            .await;
        Ok(Json(EnvironmentJobOutput {
            job_id,
            accepted: true,
            message: "Environment creation job accepted".to_string(),
        }))
    }

    #[tool(description = "Install packages into a Python, Node.js or shell environment")]
    async fn environments_install_packages(
        &self,
        Parameters(params): Parameters<EnvironmentInstallPackagesParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<EnvironmentJobOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::EnvUpdate)?;
        let packages = required_packages(params.packages)?;
        let usecase = EnvironmentUseCase::from_state(&self.state);
        let job_id = match params.runtime {
            PackageRuntime::Python => {
                usecase
                    .install_python_packages(
                        &user,
                        "mcp".to_string(),
                        required_environment_name(&params.name)?.to_string(),
                        InstallPackageRequest { packages },
                    )
                    .await
            }
            PackageRuntime::Node => {
                usecase
                    .install_node_packages(
                        &user,
                        "mcp".to_string(),
                        required_environment_name(&params.name)?.to_string(),
                        InstallPackageRequest { packages },
                    )
                    .await
            }
            PackageRuntime::Shell => {
                usecase
                    .install_shell_packages(
                        &user,
                        "mcp".to_string(),
                        InstallPackageRequest { packages },
                    )
                    .await
            }
        }
        .map_err(tool_error)?;
        self.audit(
            &user,
            "environments_install_packages",
            Some(job_id.to_string()),
        )
        .await;
        Ok(Json(EnvironmentJobOutput {
            job_id,
            accepted: true,
            message: "Package installation job accepted".to_string(),
        }))
    }

    #[tool(
        description = "Uninstall one package from a Python, Node.js or shell environment. This is destructive"
    )]
    async fn environments_uninstall_package(
        &self,
        Parameters(params): Parameters<EnvironmentUninstallPackageParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<EnvironmentActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::EnvDelete)?;
        let package = required_value(params.package, "package")?;
        let usecase = EnvironmentUseCase::from_state(&self.state);
        let (job_id, message) = match params.runtime {
            PackageRuntime::Python => {
                let job_id = usecase
                    .uninstall_python_package(
                        &user,
                        "mcp".to_string(),
                        required_environment_name(&params.name)?.to_string(),
                        package,
                    )
                    .await
                    .map_err(tool_error)?;
                (
                    Some(job_id),
                    format!("Package uninstall job {job_id} accepted"),
                )
            }
            PackageRuntime::Node => {
                let job_id = usecase
                    .uninstall_node_package(
                        &user,
                        "mcp".to_string(),
                        required_environment_name(&params.name)?.to_string(),
                        package,
                    )
                    .await
                    .map_err(tool_error)?;
                (
                    Some(job_id),
                    format!("Package uninstall job {job_id} accepted"),
                )
            }
            PackageRuntime::Shell => {
                usecase
                    .uninstall_shell_package(&user, "mcp".to_string(), package)
                    .await
                    .map_err(tool_error)?;
                (None, "System package uninstalled".to_string())
            }
        };
        self.audit(&user, "environments_uninstall_package", params.name.clone())
            .await;
        Ok(Json(EnvironmentActionOutput {
            job_id,
            accepted: true,
            message,
        }))
    }

    #[tool(
        description = "Delete a Python environment or uninstall a Node.js version. This is destructive"
    )]
    async fn environments_delete(
        &self,
        Parameters(params): Parameters<EnvironmentDeleteParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<EnvironmentActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::EnvDelete)?;
        let name = required_value(params.name, "name")?;
        let usecase = EnvironmentUseCase::from_state(&self.state);
        match params.runtime {
            ManagedEnvironmentRuntime::Python => {
                usecase
                    .delete_python_env(&user, "mcp".to_string(), name.clone())
                    .await
            }
            ManagedEnvironmentRuntime::Node => {
                usecase
                    .delete_node_env(&user, "mcp".to_string(), name.clone())
                    .await
            }
        }
        .map_err(tool_error)?;
        self.audit(&user, "environments_delete", Some(name)).await;
        Ok(Json(EnvironmentActionOutput {
            job_id: None,
            accepted: true,
            message: "Environment deleted".to_string(),
        }))
    }

    #[tool(description = "Set the system default Node.js version")]
    async fn environments_set_default(
        &self,
        Parameters(EnvironmentSetDefaultParams { name }): Parameters<EnvironmentSetDefaultParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<EnvironmentActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::EnvUpdate)?;
        let name = required_value(name, "name")?;
        EnvironmentUseCase::from_state(&self.state)
            .set_node_default(&user, "mcp".to_string(), name.clone())
            .await
            .map_err(tool_error)?;
        self.audit(&user, "environments_set_default", Some(name))
            .await;
        Ok(Json(EnvironmentActionOutput {
            job_id: None,
            accepted: true,
            message: "Default Node.js version updated".to_string(),
        }))
    }
}
