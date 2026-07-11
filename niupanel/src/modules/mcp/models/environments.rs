use super::*;

#[derive(Debug, Serialize, JsonSchema)]
pub struct EnvironmentOutput {
    pub name: String,
    pub version: Option<String>,
    pub env_type: String,
    pub is_installed: bool,
    pub recorded_packages: Option<usize>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct EnvironmentListOutput {
    pub items: Vec<EnvironmentOutput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnvironmentGetParams {
    pub name: String,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ManagedEnvironmentRuntime {
    Python,
    Node,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PackageRuntime {
    Python,
    Node,
    Shell,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnvironmentCreateParams {
    pub runtime: ManagedEnvironmentRuntime,
    #[schemars(description = "Python or Node.js version accepted by the environment manager")]
    pub version: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnvironmentDeleteParams {
    pub runtime: ManagedEnvironmentRuntime,
    #[schemars(description = "Exact environment name returned by environments_list")]
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnvironmentPackagesParams {
    pub runtime: PackageRuntime,
    #[schemars(description = "Required for python and node; omit for shell packages")]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnvironmentInstallPackagesParams {
    pub runtime: PackageRuntime,
    #[schemars(description = "Required for python and node; omit for shell packages")]
    pub name: Option<String>,
    #[schemars(
        description = "One or more package specifications accepted by pip, npm or the system package manager"
    )]
    pub packages: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnvironmentUninstallPackageParams {
    pub runtime: PackageRuntime,
    #[schemars(description = "Required for python and node; omit for shell packages")]
    pub name: Option<String>,
    pub package: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnvironmentSetDefaultParams {
    #[schemars(description = "Exact Node.js environment name returned by environments_list")]
    pub name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct EnvironmentVersionsOutput {
    #[schemars(description = "Raw version manifest returned by the configured runtime source")]
    pub manifest: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct EnvironmentPackagesOutput {
    pub runtime: PackageRuntime,
    pub name: Option<String>,
    #[schemars(description = "Raw package-list output from the runtime package manager")]
    pub packages: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct EnvironmentJobOutput {
    pub job_id: i32,
    pub accepted: bool,
    pub message: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct EnvironmentActionOutput {
    pub job_id: Option<i32>,
    pub accepted: bool,
    pub message: String,
}
