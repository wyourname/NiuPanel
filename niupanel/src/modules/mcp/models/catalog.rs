use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpToolRiskLevel {
    Read,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpToolInfo {
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub permission: &'static str,
    pub destructive: bool,
    pub risk_level: McpToolRiskLevel,
}

struct McpToolDefinition {
    name: &'static str,
    category: &'static str,
    description: &'static str,
    permission: &'static str,
    destructive: bool,
}

impl From<McpToolDefinition> for McpToolInfo {
    fn from(definition: McpToolDefinition) -> Self {
        Self {
            name: definition.name,
            category: definition.category,
            description: definition.description,
            permission: definition.permission,
            destructive: definition.destructive,
            risk_level: tool_risk_level(definition.name),
        }
    }
}

pub(crate) fn tool_risk_level(name: &str) -> McpToolRiskLevel {
    use McpToolRiskLevel::{High, Low, Medium, Read};
    match name {
        "system_status"
        | "tasks_list"
        | "tasks_get"
        | "tasks_get_logs"
        | "tasks_history"
        | "tasks_get_run_log"
        | "tasks_wait"
        | "environments_list"
        | "environments_get"
        | "environments_versions"
        | "environments_packages_list"
        | "variables_list"
        | "variables_get"
        | "audit_list"
        | "files_list"
        | "files_read"
        | "jobs_list"
        | "jobs_get"
        | "jobs_get_logs"
        | "share_station_stats"
        | "share_station_files"
        | "share_import_sources"
        | "git_repos_list"
        | "git_repo_files"
        | "git_repo_scan_tasks"
        | "system_releases"
        | "system_update_check" => Read,
        "files_create_directory" | "webhook_push" => Low,
        "tasks_create"
        | "tasks_update"
        | "tasks_pause"
        | "tasks_resume"
        | "tasks_enable"
        | "tasks_disable"
        | "environments_create"
        | "environments_set_default"
        | "variables_create"
        | "variables_update"
        | "git_repo_sync" => Medium,
        "tasks_run"
        | "tasks_stop"
        | "tasks_delete"
        | "environments_install_packages"
        | "environments_uninstall_package"
        | "environments_delete"
        | "variables_delete"
        | "files_write"
        | "files_delete"
        | "jobs_cancel" => High,
        _ => High,
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpInfoResponse {
    pub enabled: bool,
    pub endpoint: &'static str,
    pub transport: &'static str,
    pub auth_header: &'static str,
    pub required_permission: Option<&'static str>,
    pub tools: Vec<McpToolInfo>,
}

impl McpInfoResponse {
    pub fn current() -> Self {
        Self {
            enabled: true,
            endpoint: "/mcp",
            transport: "streamable_http",
            auth_header: "X-API-Key",
            required_permission: None,
            tools: vec![
                McpToolDefinition {
                    name: "system_status",
                    category: "system",
                    description: "读取面板版本与任务运行概况",
                    permission: "overview:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_list",
                    category: "tasks",
                    description: "搜索并筛选任务列表",
                    permission: "task:list",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_get",
                    category: "tasks",
                    description: "读取任务配置与当前状态",
                    permission: "task:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_get_logs",
                    category: "tasks",
                    description: "读取任务最新日志",
                    permission: "task:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_history",
                    category: "tasks",
                    description: "读取任务历史运行记录",
                    permission: "task:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_get_run_log",
                    category: "tasks",
                    description: "读取指定历史运行的日志",
                    permission: "task:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_wait",
                    category: "tasks",
                    description: "等待指定任务运行进入终态",
                    permission: "task:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_create",
                    category: "tasks",
                    description: "创建任务",
                    permission: "task:create",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_update",
                    category: "tasks",
                    description: "更新任务配置",
                    permission: "task:update",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_delete",
                    category: "tasks",
                    description: "删除任务",
                    permission: "task:delete",
                    destructive: true,
                },
                McpToolDefinition {
                    name: "tasks_run",
                    category: "tasks",
                    description: "启动任务",
                    permission: "task:run",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_stop",
                    category: "tasks",
                    description: "停止正在运行的任务",
                    permission: "task:stop",
                    destructive: true,
                },
                McpToolDefinition {
                    name: "tasks_pause",
                    category: "tasks",
                    description: "暂停正在运行的任务",
                    permission: "task:stop",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_resume",
                    category: "tasks",
                    description: "恢复已暂停的任务",
                    permission: "task:stop",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_enable",
                    category: "tasks",
                    description: "启用任务调度",
                    permission: "task:update",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "tasks_disable",
                    category: "tasks",
                    description: "停用任务调度",
                    permission: "task:update",
                    destructive: true,
                },
                McpToolDefinition {
                    name: "environments_list",
                    category: "environments",
                    description: "读取可用运行环境",
                    permission: "env:list",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "environments_get",
                    category: "environments",
                    description: "读取指定运行环境详情",
                    permission: "env:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "environments_versions",
                    category: "environments",
                    description: "读取可安装的 Python 和 Node.js 版本",
                    permission: "env:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "environments_packages_list",
                    category: "environments",
                    description: "读取运行环境或系统的软件包列表",
                    permission: "env:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "environments_create",
                    category: "environments",
                    description: "创建 Python 环境或安装 Node.js 版本",
                    permission: "env:create",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "environments_install_packages",
                    category: "environments",
                    description: "向运行环境或系统安装软件包",
                    permission: "env:update",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "environments_uninstall_package",
                    category: "environments",
                    description: "从运行环境或系统卸载软件包",
                    permission: "env:delete",
                    destructive: true,
                },
                McpToolDefinition {
                    name: "environments_delete",
                    category: "environments",
                    description: "删除 Python 环境或卸载 Node.js 版本",
                    permission: "env:delete",
                    destructive: true,
                },
                McpToolDefinition {
                    name: "environments_set_default",
                    category: "environments",
                    description: "设置系统默认 Node.js 版本",
                    permission: "env:update",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "variables_list",
                    category: "variables",
                    description: "列出变量元数据，不返回变量值",
                    permission: "var:list",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "variables_get",
                    category: "variables",
                    description: "读取指定变量及其敏感值",
                    permission: "var:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "variables_create",
                    category: "variables",
                    description: "创建变量",
                    permission: "var:create",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "variables_update",
                    category: "variables",
                    description: "更新变量",
                    permission: "var:update",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "variables_delete",
                    category: "variables",
                    description: "删除变量",
                    permission: "var:delete",
                    destructive: true,
                },
                McpToolDefinition {
                    name: "audit_list",
                    category: "audit",
                    description: "读取最近的审计记录",
                    permission: "audit:list",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "files_list",
                    category: "files",
                    description: "列出脚本目录内容",
                    permission: "file:list",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "files_read",
                    category: "files",
                    description: "读取脚本文件内容",
                    permission: "file:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "files_write",
                    category: "files",
                    description: "写入脚本文件内容",
                    permission: "file:write",
                    destructive: true,
                },
                McpToolDefinition {
                    name: "files_create_directory",
                    category: "files",
                    description: "创建脚本目录",
                    permission: "file:write",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "files_delete",
                    category: "files",
                    description: "删除脚本文件或目录",
                    permission: "file:delete",
                    destructive: true,
                },
                McpToolDefinition {
                    name: "jobs_list",
                    category: "jobs",
                    description: "列出系统后台作业",
                    permission: "job:list",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "jobs_get",
                    category: "jobs",
                    description: "读取后台作业详情",
                    permission: "job:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "jobs_get_logs",
                    category: "jobs",
                    description: "读取后台作业日志",
                    permission: "job:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "jobs_cancel",
                    category: "jobs",
                    description: "取消正在运行的后台作业",
                    permission: "job:*",
                    destructive: true,
                },
                McpToolDefinition {
                    name: "webhook_push",
                    category: "notifications",
                    description: "通过面板通知渠道推送消息",
                    permission: "webhook:push",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "share_station_stats",
                    category: "share",
                    description: "读取分享中转站容量与配置状态",
                    permission: "share:list",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "share_station_files",
                    category: "share",
                    description: "列出分享中转站文件，不返回密码",
                    permission: "share:list",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "share_import_sources",
                    category: "share",
                    description: "读取已导入任务的来源摘要",
                    permission: "share:list",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "git_repos_list",
                    category: "git",
                    description: "列出 Git 仓库及同步状态，不返回凭据",
                    permission: "git:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "git_repo_files",
                    category: "git",
                    description: "浏览指定 Git 仓库文件",
                    permission: "git:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "git_repo_scan_tasks",
                    category: "git",
                    description: "扫描指定 Git 仓库中的可导入任务",
                    permission: "git:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "git_repo_sync",
                    category: "git",
                    description: "同步指定 Git 仓库",
                    permission: "git:sync",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "system_releases",
                    category: "system",
                    description: "读取已安装的 Panel Release 与回退状态",
                    permission: "overview:read",
                    destructive: false,
                },
                McpToolDefinition {
                    name: "system_update_check",
                    category: "system",
                    description: "按当前通道检查 Panel 更新",
                    permission: "overview:read",
                    destructive: false,
                },
            ]
            .into_iter()
            .map(McpToolInfo::from)
            .collect(),
        }
    }
}
