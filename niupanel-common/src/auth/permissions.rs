use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use strum_macros::{AsRefStr, Display, EnumString};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, AsRefStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum Permission {
    // 任务相关 (Task)
    #[strum(serialize = "task:list")]
    TaskList,
    #[strum(serialize = "task:read")]
    TaskRead,
    #[strum(serialize = "task:create")]
    TaskCreate,
    #[strum(serialize = "task:update")]
    TaskUpdate,
    #[strum(serialize = "task:delete")]
    TaskDelete,
    #[strum(serialize = "task:run")]
    TaskRun,
    #[strum(serialize = "task:stop")]
    TaskStop,
    #[strum(serialize = "task:*")]
    TaskAll,

    // 变量相关 (Variable)
    #[strum(serialize = "var:list")]
    VarList,
    #[strum(serialize = "var:read")]
    VarRead,
    #[strum(serialize = "var:create")]
    VarCreate,
    #[strum(serialize = "var:update")]
    VarUpdate,
    #[strum(serialize = "var:delete")]
    VarDelete,
    #[strum(serialize = "var:*")]
    VarAll,

    // 密钥相关 (Key)
    #[strum(serialize = "key:list")]
    KeyList,
    #[strum(serialize = "key:create")]
    KeyCreate,
    #[strum(serialize = "key:update")]
    KeyUpdate,
    #[strum(serialize = "key:delete")]
    KeyDelete,
    #[strum(serialize = "key:*")]
    KeyAll,

    // 环境相关 (Environment)
    #[strum(serialize = "env:list")]
    EnvList,
    #[strum(serialize = "env:read")]
    EnvRead,
    #[strum(serialize = "env:create")]
    EnvCreate,
    #[strum(serialize = "env:update")]
    EnvUpdate,
    #[strum(serialize = "env:delete")]
    EnvDelete,
    #[strum(serialize = "env:*")]
    EnvAll,

    // 文件相关 (File)
    #[strum(serialize = "file:list")]
    FileList,
    #[strum(serialize = "file:read")]
    FileRead,
    #[strum(serialize = "file:write")]
    FileWrite,
    #[strum(serialize = "file:delete")]
    FileDelete,
    #[strum(serialize = "file:*")]
    FileAll,

    // 审计相关 (Audit)
    #[strum(serialize = "audit:list")]
    AuditList,
    #[strum(serialize = "audit:*")]
    AuditAll,

    // 设置相关 (Setting)
    #[strum(serialize = "setting:read")]
    SettingRead,
    #[strum(serialize = "setting:update")]
    SettingUpdate,
    #[strum(serialize = "setting:*")]
    SettingAll,

    // 任务记录 (Job)
    #[strum(serialize = "job:list")]
    JobList,
    #[strum(serialize = "job:read")]
    JobRead,
    #[strum(serialize = "job:delete")]
    JobDelete,
    #[strum(serialize = "job:*")]
    JobAll,

    // 概览相关 (Overview)
    #[strum(serialize = "overview:read")]
    OverviewRead,

    // 编译相关 (Compiler)
    #[strum(serialize = "compiler:read")]
    CompilerRead,
    #[strum(serialize = "compiler:run")]
    CompilerRun,

    // 分享相关 (Share)
    #[strum(serialize = "share:list")]
    ShareList,
    #[strum(serialize = "share:create")]
    ShareCreate,
    #[strum(serialize = "share:update")]
    ShareUpdate,
    #[strum(serialize = "share:delete")]
    ShareDelete,
    #[strum(serialize = "share:*")]
    ShareAll,

    // 个人资料 (Profile)
    #[strum(serialize = "profile:read")]
    ProfileRead,
    #[strum(serialize = "profile:update")]
    ProfileUpdate,

    // Webhook相关 (Webhook)
    #[strum(serialize = "webhook:push")]
    WebhookPush,
    #[strum(serialize = "webhook:*")]
    WebhookAll,

    // Git同步相关 (Git)
    #[strum(serialize = "git:read")]
    GitRead,
    #[strum(serialize = "git:config")]
    GitConfig,
    #[strum(serialize = "git:sync")]
    GitSync,
    #[strum(serialize = "git:*")]
    GitAll,

    // 终端相关 (Terminal)
    #[strum(serialize = "terminal:access")]
    TerminalAccess,
    #[strum(serialize = "terminal:*")]
    TerminalAll,

    // 全局通配符
    #[strum(serialize = "*:*")]
    All,
}

impl Permission {
    /// 获取权限所属的资源部分
    pub fn resource(&self) -> &'static str {
        match self {
            Permission::TaskList
            | Permission::TaskRead
            | Permission::TaskCreate
            | Permission::TaskUpdate
            | Permission::TaskDelete
            | Permission::TaskRun
            | Permission::TaskStop
            | Permission::TaskAll => "task",

            Permission::VarList
            | Permission::VarRead
            | Permission::VarCreate
            | Permission::VarUpdate
            | Permission::VarDelete
            | Permission::VarAll => "var",

            Permission::KeyList
            | Permission::KeyCreate
            | Permission::KeyUpdate
            | Permission::KeyDelete
            | Permission::KeyAll => "key",

            Permission::EnvList
            | Permission::EnvRead
            | Permission::EnvCreate
            | Permission::EnvUpdate
            | Permission::EnvDelete
            | Permission::EnvAll => "env",

            Permission::FileList
            | Permission::FileRead
            | Permission::FileWrite
            | Permission::FileDelete
            | Permission::FileAll => "file",

            Permission::AuditList | Permission::AuditAll => "audit",

            Permission::SettingRead | Permission::SettingUpdate | Permission::SettingAll => {
                "setting"
            }

            Permission::JobList
            | Permission::JobRead
            | Permission::JobDelete
            | Permission::JobAll => "job",

            Permission::ShareList
            | Permission::ShareCreate
            | Permission::ShareUpdate
            | Permission::ShareDelete
            | Permission::ShareAll => "share",

            Permission::OverviewRead => "overview",

            Permission::CompilerRead | Permission::CompilerRun => "compiler",

            Permission::ProfileRead | Permission::ProfileUpdate => "profile",

            Permission::WebhookPush | Permission::WebhookAll => "webhook",

            Permission::GitRead
            | Permission::GitConfig
            | Permission::GitSync
            | Permission::GitAll => "git",

            Permission::TerminalAccess | Permission::TerminalAll => "terminal",

            Permission::All => "*",
        }
    }

    /// 获取该资源对应的通配符权限项
    pub fn get_resource_wildcard(&self) -> Option<Permission> {
        match self.resource() {
            "task" => Some(Permission::TaskAll),
            "var" => Some(Permission::VarAll),
            "key" => Some(Permission::KeyAll),
            "env" => Some(Permission::EnvAll),
            "file" => Some(Permission::FileAll),
            "audit" => Some(Permission::AuditAll),
            "setting" => Some(Permission::SettingAll),
            "job" => Some(Permission::JobAll),
            "share" => Some(Permission::ShareAll),
            "webhook" => Some(Permission::WebhookAll),
            "git" => Some(Permission::GitAll),
            "terminal" => Some(Permission::TerminalAll),
            _ => None,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, AsRefStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum UserRole {
    Admin,
    System,
    User,
    #[strum(serialize = "api_client")]
    ApiClient,
}

pub fn parse_permissions(perm_str: &str) -> HashSet<Permission> {
    perm_str
        .split(',')
        .map(|s| s.trim())
        .filter_map(|s| s.parse::<Permission>().ok())
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub id: i32,
    pub role: UserRole,
    pub permissions: HashSet<Permission>,
    pub ip: String,
}

impl AuthenticatedUser {
    pub fn has_permission(&self, required: Permission) -> bool {
        // 1. Admin 角色具有隐式全权限 (超级快速路径)
        if self.role == UserRole::Admin {
            return true;
        }

        // 2. 检查是否有精确匹配的权限 或 全局通配符 *:* (O(1) 路径)
        if self.permissions.contains(&required) || self.permissions.contains(&Permission::All) {
            return true;
        }

        // 3. 检查是否存在资源级通配符，如 task:* (O(1) 路径)
        if let Some(wildcard) = required.get_resource_wildcard() {
            if self.permissions.contains(&wildcard) {
                return true;
            }
        }

        false
    }
}
