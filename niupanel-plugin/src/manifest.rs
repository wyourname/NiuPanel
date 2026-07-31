use super::*;

pub type PluginToolFuture =
    Pin<Box<dyn Future<Output = Result<serde_json::Value>> + Send + 'static>>;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PluginManifest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub license: Option<String>,
    pub runtime: PluginRuntime,
    #[serde(default)]
    pub protocol: PluginProcessProtocol,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub runtime_permissions: Vec<PluginRuntimePermission>,
    #[serde(default)]
    pub timeout_sec: Option<u64>,
    #[serde(default)]
    pub worker: PluginWorkerConfig,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    #[serde(default)]
    pub compatibility: PluginCompatibilityManifest,
    #[serde(default)]
    pub ui: Option<PluginUiManifest>,
    #[serde(default)]
    pub theme: Option<PluginThemeManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PluginThemeManifest {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub light: PluginThemePalette,
    #[serde(default)]
    pub dark: PluginThemePalette,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct PluginThemePalette {
    #[serde(default)]
    pub primary: Option<String>,
    #[serde(default)]
    pub bg_base: Option<String>,
    #[serde(default)]
    pub bg_card: Option<String>,
    #[serde(default)]
    pub bg_subtle: Option<String>,
    #[serde(default)]
    pub bg_soft: Option<String>,
    #[serde(default)]
    pub text_default: Option<String>,
    #[serde(default)]
    pub text_secondary: Option<String>,
    #[serde(default)]
    pub text_muted: Option<String>,
    #[serde(default)]
    pub border_base: Option<String>,
    #[serde(default)]
    pub border_light: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct PluginCompatibilityManifest {
    #[serde(default)]
    pub panel: PluginPanelCompatibility,
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct PluginPanelCompatibility {
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub max_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PluginDependency {
    pub id: String,
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PluginUiManifest {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub mode: PluginUiMode,
    pub entry: String,
    #[serde(default)]
    pub sdk_version: Option<String>,
    #[serde(default)]
    pub display: PluginUiDisplayManifest,
    #[serde(default)]
    pub routes: Vec<PluginUiRoute>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub api: PluginUiApiManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginUiMode {
    #[default]
    VueApp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PluginUiRoute {
    #[serde(default)]
    pub id: Option<String>,
    pub path: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub menu: Option<String>,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub exact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PluginUiDisplayManifest {
    #[serde(default = "default_ui_sidebar")]
    pub sidebar: bool,
    #[serde(default = "default_ui_workspace")]
    pub workspace: bool,
    #[serde(default = "default_ui_mobile")]
    pub mobile: bool,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub layout: PluginUiLayout,
}

impl Default for PluginUiDisplayManifest {
    fn default() -> Self {
        Self {
            sidebar: default_ui_sidebar(),
            workspace: default_ui_workspace(),
            mobile: default_ui_mobile(),
            category: None,
            order: 0,
            layout: PluginUiLayout::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginUiLayout {
    #[default]
    Panel,
    FullBleed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct PluginUiApiManifest {
    #[serde(default)]
    pub allow: Vec<PluginUiApiRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PluginUiApiRule {
    #[serde(default)]
    pub methods: Vec<String>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntime {
    Builtin,
    Declarative,
    Process,
    Wasi,
    Native,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntimePermission {
    NetworkOutbound,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginProcessProtocol {
    #[default]
    SingleShot,
    JsonLines,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PluginWorkerConfig {
    #[serde(default = "default_worker_min")]
    pub min: usize,
    #[serde(default = "default_worker_max")]
    pub max: usize,
    #[serde(default = "default_worker_idle_timeout_sec")]
    pub idle_timeout_sec: u64,
}

impl Default for PluginWorkerConfig {
    fn default() -> Self {
        Self {
            min: default_worker_min(),
            max: default_worker_max(),
            idle_timeout_sec: default_worker_idle_timeout_sec(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessPluginSpec {
    pub plugin_id: String,
    pub version: String,
    pub extension_point: String,
    pub plugin_dir: PathBuf,
    pub entry: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub runtime_permissions: Vec<PluginRuntimePermission>,
    pub timeout_sec: Option<u64>,
    pub worker: PluginWorkerConfig,
    pub capabilities: Vec<String>,
    pub tools: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PluginInvokeRequest {
    #[serde(default = "default_invoke_action")]
    pub action: String,
    #[serde(default)]
    pub input: serde_json::Value,
    pub timeout_sec: Option<u64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginInvokeResponse {
    pub plugin_id: String,
    pub extension_point: String,
    pub version: String,
    pub runtime: PluginRuntime,
    pub request_id: String,
    pub output: serde_json::Value,
    pub stderr: Option<String>,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcessPluginRequest {
    pub protocol_version: u32,
    pub request_id: String,
    pub plugin_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    pub extension_point: String,
    pub action: String,
    pub input: serde_json::Value,
    pub capabilities: Vec<String>,
    pub tools: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcessPluginToolCall {
    #[serde(rename = "type")]
    pub kind: String,
    pub request_id: String,
    pub call_id: String,
    pub tool: String,
    #[serde(default)]
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcessPluginToolResult {
    #[serde(rename = "type")]
    pub kind: String,
    pub request_id: String,
    pub call_id: String,
    pub ok: bool,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<ProcessPluginError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcessPluginResponse {
    pub request_id: Option<String>,
    pub ok: bool,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<ProcessPluginError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcessPluginError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PluginState {
    pub enabled: bool,
    pub installed_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub active_version: Option<String>,
    #[serde(default)]
    pub package_sha256: Option<String>,
    #[serde(default)]
    pub history: Vec<PluginVersionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PluginVersionRecord {
    pub id: String,
    pub version: String,
    pub archived_at: String,
    pub path: String,
    #[serde(default)]
    pub package_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginRecord {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub active_version: Option<String>,
    pub source: PluginSource,
    pub status: PluginStatus,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginSource {
    Local,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    Enabled,
    Disabled,
    Error,
}
