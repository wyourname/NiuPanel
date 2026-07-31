/// 系统设置键名定义
pub mod settings {
    pub const SYSTEM_MAX_CONCURRENCY: &str = "system.max_concurrency";
    pub const SYSTEM_TIMEZONE: &str = "system.timezone";
    pub const SYSTEM_GITHUB_PROXY: &str = "system.github_proxy_url";
    pub const SYSTEM_UPDATE_CHANNEL: &str = "system.update_channel";
    pub const SYSTEM_LOG_RETENTION: &str = "system.log_retention_days";
    pub const SYSTEM_SESSION_KEY: &str = "system.session_key";
    pub const SYSTEM_API_KEY: &str = "system.api_key";

    pub const AUTH_MAX_SESSIONS: &str = "auth.max_sessions";

    pub const NOTIFY_WEBHOOK_URL: &str = "notify.webhook_url";
    pub const NOTIFY_EVENTS: &str = "notify.events";

    pub const MAIL_HOST: &str = "mail.host";
    pub const MAIL_TO: &str = "mail.to";
    pub const MAIL_USERNAME: &str = "mail.username";
    pub const MAIL_PASSWORD: &str = "mail.password";

    pub const UV_PYTHON_MIRROR: &str = "system.uv_python_mirror";
    pub const UV_PYPI_MIRROR: &str = "system.uv_pypi_mirror";

    pub const SYSTEM_DEFAULT_PYTHON_VERSION: &str = "system.default_python_version";
    pub const SYSTEM_DEFAULT_NODE_VERSION: &str = "system.default_node_version";
    pub const SYSTEM_ONBOARDING_DONE: &str = "system.onboarding_done";
    pub const PNPM_NODE_DIST_MIRROR: &str = "system.pnpm_node_dist_mirror";
    // 仅用于读取升级前的镜像配置，所有新写入都使用 PNPM_NODE_DIST_MIRROR。
    pub const LEGACY_FNM_NODE_DIST_MIRROR: &str = "system.fnm_node_dist_mirror";
    pub const NPM_REGISTRY_MIRROR: &str = "system.npm_registry_mirror";
    pub const NODE_DEFAULT_PACKAGES: &str = "system.node_default_packages";

    pub const AGENT_MODEL_ENABLED: &str = "agent.model_enabled";
    pub const AGENT_MODEL_PROVIDER: &str = "agent.model_provider";
    pub const AGENT_MODEL_BASE_URL: &str = "agent.model_base_url";
    pub const AGENT_MODEL_API_KEY: &str = "agent.model_api_key";
    pub const AGENT_MODEL_NAME: &str = "agent.model_name";
    pub const AGENT_MODEL_TEMPERATURE: &str = "agent.model_temperature";
}

/// 默认值定义
pub mod defaults {
    pub const MAX_CONCURRENCY: usize = 15;
    pub const TIMEZONE: &str = "Asia/Shanghai";
    pub const LOG_RETENTION_DAYS: i64 = 15;
    pub const LOG_RETENTION_MIN_DAYS: i64 = 1;
    pub const LOG_RETENTION_MAX_DAYS: i64 = 365;
    pub const MAX_SESSIONS: i32 = 2;
}

/// 脚本执行相关常量
pub mod script {
    /// 常见的解释器名称列表，用于在参数解析时忽略
    pub const INTERPRETERS: &[&str] = &[
        "python", "python3", "python2", "py", "node", "nodejs", "pnpm", "pnpx", "npm", "deno",
        "sh", "bash", "zsh", "go", "cargo", "java", "php", "ruby", "perl",
    ];
}
