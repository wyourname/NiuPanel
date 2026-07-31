use niupanel_common::constants::defaults;
use niupanel_common::constants::settings::*;
use niupanel_common::error::{AppError, Result};
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Clone)]
pub struct SettingDef {
    pub key: &'static str,
    pub description: &'static str,
    pub default_value: String,
    pub validator: fn(&str) -> Result<()>,
    pub env_var: Option<&'static str>,
    pub is_secret: bool,
    pub category: &'static str,
}

fn validate_log_retention(val: &str) -> Result<()> {
    let value = val
        .parse::<i64>()
        .map_err(|_| AppError::Generic("Log retention days must be an integer".to_string()))?;
    if !(defaults::LOG_RETENTION_MIN_DAYS..=defaults::LOG_RETENTION_MAX_DAYS).contains(&value) {
        return Err(AppError::Generic(format!(
            "Log retention days must be between {} and {}",
            defaults::LOG_RETENTION_MIN_DAYS,
            defaults::LOG_RETENTION_MAX_DAYS
        )));
    }
    Ok(())
}

fn validate_max_concurrency(val: &str) -> Result<()> {
    let value = val
        .parse::<usize>()
        .map_err(|_| AppError::Generic("Max concurrency must be an integer".to_string()))?;
    if !(1..=1024).contains(&value) {
        return Err(AppError::Generic(
            "Max concurrency must be between 1 and 1024".to_string(),
        ));
    }
    Ok(())
}

fn validate_max_sessions(val: &str) -> Result<()> {
    let v = val
        .parse::<i32>()
        .map_err(|_| AppError::Generic("Max sessions must be a number".to_string()))?;
    if v < 1 {
        return Err(AppError::Generic(
            "Max sessions must be at least 1".to_string(),
        ));
    }
    Ok(())
}

fn validate_string(_: &str) -> Result<()> {
    Ok(())
}

fn validate_url(val: &str) -> Result<()> {
    if val.is_empty() {
        return Ok(());
    }
    if val.starts_with("http://") || val.starts_with("https://") {
        Ok(())
    } else {
        Err(AppError::Generic(
            "Value must be a valid URL (http/https)".to_string(),
        ))
    }
}

fn validate_bool(val: &str) -> Result<()> {
    match val {
        "true" | "false" | "1" | "0" => Ok(()),
        _ => Err(AppError::Generic("Value must be a boolean".to_string())),
    }
}

fn validate_agent_model_provider(val: &str) -> Result<()> {
    match val {
        "openai_compatible" => Ok(()),
        _ => Err(AppError::Generic(
            "Agent model provider must be openai_compatible".to_string(),
        )),
    }
}

fn validate_agent_temperature(val: &str) -> Result<()> {
    let v = val
        .parse::<f32>()
        .map_err(|_| AppError::Generic("Temperature must be a number".to_string()))?;
    if (0.0..=2.0).contains(&v) {
        Ok(())
    } else {
        Err(AppError::Generic(
            "Temperature must be between 0 and 2".to_string(),
        ))
    }
}

fn validate_update_channel(val: &str) -> Result<()> {
    match val {
        "stable" | "preview" => Ok(()),
        _ => Err(AppError::Generic(
            "Update channel must be stable or preview".to_string(),
        )),
    }
}

pub static SETTINGS_REGISTRY: LazyLock<HashMap<&'static str, SettingDef>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    let defs = vec![
        SettingDef {
            key: SYSTEM_MAX_CONCURRENCY,
            description: "Max concurrent tasks",
            default_value: defaults::MAX_CONCURRENCY.to_string(),
            validator: validate_max_concurrency,
            env_var: Some("NIU_SYSTEM_MAX_CONCURRENCY"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: SYSTEM_TIMEZONE,
            description: "System Timezone",
            default_value: defaults::TIMEZONE.to_string(),
            validator: validate_string,
            env_var: Some("NIU_SYSTEM_TIMEZONE"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: SYSTEM_GITHUB_PROXY,
            description: "GitHub Proxy URL",
            default_value: "".to_string(),
            validator: validate_url,
            env_var: Some("NIU_SYSTEM_GITHUB_PROXY"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: SYSTEM_UPDATE_CHANNEL,
            description: "System update channel",
            default_value: "stable".to_string(),
            validator: validate_update_channel,
            env_var: Some("NIU_SYSTEM_UPDATE_CHANNEL"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: SYSTEM_LOG_RETENTION,
            description: "Log retention days",
            default_value: defaults::LOG_RETENTION_DAYS.to_string(),
            validator: validate_log_retention,
            env_var: Some("NIU_SYSTEM_LOG_RETENTION"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: SYSTEM_SESSION_KEY,
            description: "System Session Key",
            // Bootstrap generates and persists an installation-specific key.
            // A registry default must never be a usable shared signing secret.
            default_value: String::new(),
            validator: validate_string,
            env_var: Some("NIU_SESSION_KEY"),
            is_secret: true,
            category: "Security",
        },
        SettingDef {
            key: SYSTEM_API_KEY,
            description: "System API Key for SDK",
            default_value: "".to_string(),
            validator: validate_string,
            env_var: Some("NIU_SYSTEM_API_KEY"),
            is_secret: true,
            category: "Security",
        },
        SettingDef {
            key: AUTH_MAX_SESSIONS,
            description: "Maximum concurrent sessions per user",
            default_value: defaults::MAX_SESSIONS.to_string(),
            validator: validate_max_sessions,
            env_var: Some("NIU_AUTH_MAX_SESSIONS"),
            is_secret: false,
            category: "Security",
        },
        SettingDef {
            key: NOTIFY_WEBHOOK_URL,
            description: "Notification Webhook URL",
            default_value: "".to_string(),
            validator: validate_url,
            env_var: Some("NIU_NOTIFY_WEBHOOK_URL"),
            is_secret: true,
            category: "Notification",
        },
        SettingDef {
            key: NOTIFY_EVENTS,
            description: "Notification Events (comma separated)",
            default_value: "failed,login".to_string(),
            validator: validate_string,
            env_var: Some("NIU_NOTIFY_EVENTS"),
            is_secret: false,
            category: "Notification",
        },
        SettingDef {
            key: MAIL_HOST,
            description: "SMTP Host",
            default_value: "".to_string(),
            validator: validate_string,
            env_var: Some("NIU_MAIL_HOST"),
            is_secret: false,
            category: "Notification",
        },
        SettingDef {
            key: MAIL_TO,
            description: "Mail To",
            default_value: "".to_string(),
            validator: validate_string,
            env_var: Some("NIU_MAIL_TO"),
            is_secret: false,
            category: "Notification",
        },
        SettingDef {
            key: MAIL_USERNAME,
            description: "Mail Username",
            default_value: "".to_string(),
            validator: validate_string,
            env_var: Some("NIU_MAIL_USERNAME"),
            is_secret: false,
            category: "Notification",
        },
        SettingDef {
            key: MAIL_PASSWORD,
            description: "Mail Password",
            default_value: "".to_string(),
            validator: validate_string,
            env_var: Some("NIU_MAIL_PASSWORD"),
            is_secret: true,
            category: "Notification",
        },
        SettingDef {
            key: UV_PYTHON_MIRROR,
            description: "UV Python Install Mirror",
            default_value: "https://gh-proxy.com/https://github.com/astral-sh/python-build-standalone/releases/download/".to_string(),
            validator: validate_url,
            env_var: Some("NIU_UV_PYTHON_MIRROR"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: UV_PYPI_MIRROR,
            description: "UV PyPI Mirror",
            default_value: "https://pypi.tuna.tsinghua.edu.cn/simple".to_string(),
            validator: validate_url,
            env_var: Some("NIU_UV_PYPI_MIRROR"),
            is_secret: false,
            category: "System",
        },
        // --- Added from Config ---
        SettingDef {
            key: "system.ip_check_url",
            description: "IP Check URL",
            default_value: "https://ipcheck.365676.xyz".to_string(),
            validator: validate_url,
            env_var: Some("NIU_IP_CHECK_URL"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: "system.station_url",
            description: "Station URL",
            default_value: "".to_string(),
            validator: validate_url,
            env_var: Some("NIU_STATION_URL"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: "system.station_admin_token",
            description: "Station Admin Token",
            default_value: "".to_string(),
            validator: validate_string,
            env_var: Some("NIU_STATION_ADMIN_TOKEN"),
            is_secret: true,
            category: "System",
        },
        SettingDef {
            key: "system.python_default_packages",
            description: "Default Python Packages",
            default_value: "requests,aiohttp,httpx,pycryptodome".to_string(),
            validator: validate_string,
            env_var: Some("NIU_PYTHON_DEFAULT_PACKAGES"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: NODE_DEFAULT_PACKAGES,
            description: "Default Node Packages",
            default_value: "".to_string(),
            validator: validate_string,
            env_var: Some("NIU_NODE_DEFAULT_PACKAGES"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: SYSTEM_DEFAULT_PYTHON_VERSION,
            description: "Default Python Version",
            default_value: "".to_string(), // empty means system or latest
            validator: validate_string,
            env_var: Some("NIU_DEFAULT_PYTHON_VERSION"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: SYSTEM_DEFAULT_NODE_VERSION,
            description: "Default Node Version",
            default_value: "".to_string(),
            validator: validate_string,
            env_var: Some("NIU_DEFAULT_NODE_VERSION"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: SYSTEM_ONBOARDING_DONE,
            description: "Whether onboarding setup has been completed",
            default_value: "false".to_string(),
            validator: validate_string,
            env_var: None,
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: PNPM_NODE_DIST_MIRROR,
            description: "pnpm runtime Node.js distribution mirror URL",
            default_value: "https://mirrors.ustc.edu.cn/node/".to_string(),
            validator: validate_url,
            env_var: Some("PNPM_NODE_DIST_MIRROR"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: NPM_REGISTRY_MIRROR,
            description: "pnpm package registry mirror URL",
            default_value: "https://registry.npmmirror.com/".to_string(),
            validator: validate_url,
            env_var: Some("NPM_CONFIG_REGISTRY"),
            is_secret: false,
            category: "System",
        },
        SettingDef {
            key: AGENT_MODEL_ENABLED,
            description: "Whether agent model enhancement is enabled",
            default_value: "false".to_string(),
            validator: validate_bool,
            env_var: Some("NIU_AGENT_MODEL_ENABLED"),
            is_secret: false,
            category: "Agent",
        },
        SettingDef {
            key: AGENT_MODEL_PROVIDER,
            description: "Agent model provider",
            default_value: "openai_compatible".to_string(),
            validator: validate_agent_model_provider,
            env_var: Some("NIU_AGENT_MODEL_PROVIDER"),
            is_secret: false,
            category: "Agent",
        },
        SettingDef {
            key: AGENT_MODEL_BASE_URL,
            description: "OpenAI-compatible base URL for agent model calls",
            default_value: "http://127.0.0.1:11434/v1".to_string(),
            validator: validate_url,
            env_var: Some("NIU_AGENT_MODEL_BASE_URL"),
            is_secret: false,
            category: "Agent",
        },
        SettingDef {
            key: AGENT_MODEL_API_KEY,
            description: "Agent model API key",
            default_value: "".to_string(),
            validator: validate_string,
            env_var: Some("NIU_AGENT_MODEL_API_KEY"),
            is_secret: true,
            category: "Agent",
        },
        SettingDef {
            key: AGENT_MODEL_NAME,
            description: "Agent model name",
            default_value: "".to_string(),
            validator: validate_string,
            env_var: Some("NIU_AGENT_MODEL_NAME"),
            is_secret: false,
            category: "Agent",
        },
        SettingDef {
            key: AGENT_MODEL_TEMPERATURE,
            description: "Agent model temperature",
            default_value: "0.2".to_string(),
            validator: validate_agent_temperature,
            env_var: Some("NIU_AGENT_MODEL_TEMPERATURE"),
            is_secret: false,
            category: "Agent",
        },
    ];

    for def in defs {
        m.insert(def.key, def);
    }

    m
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_concurrency_rejects_zero_invalid_and_excessive_values() {
        assert!(validate_max_concurrency("0").is_err());
        assert!(validate_max_concurrency("-1").is_err());
        assert!(validate_max_concurrency("1025").is_err());
        assert!(validate_max_concurrency("not-a-number").is_err());
    }

    #[test]
    fn max_concurrency_accepts_supported_boundaries() {
        assert!(validate_max_concurrency("1").is_ok());
        assert!(validate_max_concurrency("1024").is_ok());
    }

    #[test]
    fn log_retention_uses_shared_supported_range() {
        assert!(validate_log_retention(&defaults::LOG_RETENTION_MIN_DAYS.to_string()).is_ok());
        assert!(validate_log_retention(&defaults::LOG_RETENTION_MAX_DAYS.to_string()).is_ok());
        assert!(
            validate_log_retention(&(defaults::LOG_RETENTION_MIN_DAYS - 1).to_string()).is_err()
        );
        assert!(
            validate_log_retention(&(defaults::LOG_RETENTION_MAX_DAYS + 1).to_string()).is_err()
        );
        assert!(validate_log_retention("not-a-number").is_err());
    }

    #[test]
    fn session_key_has_no_shared_registry_default() {
        let definition = SETTINGS_REGISTRY
            .get(SYSTEM_SESSION_KEY)
            .expect("session key setting must exist");
        assert!(definition.default_value.is_empty());
        assert!(definition.is_secret);
    }
}
