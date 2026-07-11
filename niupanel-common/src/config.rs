use super::logger::info;
use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_server_addr")]
    pub server_addr: String,

    // SESSION_KEY is needed before the settings registry is available. When it
    // is omitted, generate an ephemeral key instead of sharing a repository-wide
    // default across every deployment.
    #[serde(default = "default_session_key")]
    pub session_key: String,

    #[serde(default = "default_database_url")]
    pub database_url: String,

    #[serde(default = "default_scripts_dir")]
    pub scripts_dir: PathBuf,
    #[serde(default = "default_jobs_dir")]
    pub jobs_dir: PathBuf,
    #[serde(default = "default_logs_dir")]
    pub logs_dir: PathBuf,
    #[serde(default = "default_runtimes_dir")]
    pub runtimes_dir: PathBuf,
    #[serde(default = "default_share_output_dir")]
    pub share_output_dir: PathBuf,
    #[serde(default = "default_share_import_dir")]
    pub share_import_dir: PathBuf,
    #[serde(default = "default_backup_dir")]
    pub backup_dir: PathBuf,
    #[serde(default = "default_restore_dir")]
    pub restore_dir: PathBuf,
    #[serde(default = "default_agents_dir")]
    pub agents_dir: PathBuf,
    #[serde(default = "default_plugins_dir")]
    pub plugins_dir: PathBuf,
    #[serde(default = "default_system_dir")]
    pub system_dir: PathBuf,
    #[serde(default = "default_bundled_web_dir")]
    pub bundled_web_dir: PathBuf,
    #[serde(default)]
    pub allow_missing_bundled_web: bool,
    #[serde(default = "default_plugin_signature_required")]
    pub plugin_signature_required: bool,
    #[serde(default)]
    pub trusted_plugin_public_keys: Vec<String>,
    #[serde(default = "default_mcp_allowed_hosts")]
    pub mcp_allowed_hosts: Vec<String>,

    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub log_filters: String,

    #[serde(default = "default_cors_origins")]
    pub cors_allowed_origins: Vec<String>,

    // Runtime SDK paths computed at startup
    #[serde(skip)]
    pub extra_python_path: Option<String>,
    #[serde(skip)]
    pub extra_node_path: Option<String>,
}

fn default_log_level() -> String {
    "info".to_owned()
}

fn default_cors_origins() -> Vec<String> {
    vec![]
}

fn default_server_addr() -> String {
    "0.0.0.0:7788".to_owned()
}

fn default_session_key() -> String {
    nanoid::nanoid!(64)
}

fn default_database_url() -> String {
    "sqlite://data/database/niupanel.db?mode=rwc".to_owned()
}

fn default_scripts_dir() -> PathBuf {
    "data/scripts".into()
}

fn default_jobs_dir() -> PathBuf {
    "data/jobs".into()
}

fn default_logs_dir() -> PathBuf {
    "data/logs".into()
}

fn default_runtimes_dir() -> PathBuf {
    "data/runtimes".into()
}

fn default_share_output_dir() -> PathBuf {
    "data/share/output".into()
}

fn default_share_import_dir() -> PathBuf {
    "data/share/import".into()
}

fn default_backup_dir() -> PathBuf {
    "data/share/backup".into()
}

fn default_restore_dir() -> PathBuf {
    "data/share/restore".into()
}

fn default_agents_dir() -> PathBuf {
    "data/agents".into()
}

fn default_plugins_dir() -> PathBuf {
    "data/plugins".into()
}

fn default_system_dir() -> PathBuf {
    "data/system".into()
}

fn default_bundled_web_dir() -> PathBuf {
    "web".into()
}

fn default_mcp_allowed_hosts() -> Vec<String> {
    vec!["localhost".into(), "127.0.0.1".into(), "::1".into()]
}

fn default_plugin_signature_required() -> bool {
    true
}

pub static CONFIG: OnceLock<Config> = OnceLock::new();

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let config = envy::from_env::<Config>()?;
        config.validate()?;
        Ok(config)
    }

    pub fn global() -> &'static Config {
        CONFIG.get().expect("Config is not initialized")
    }

    pub fn validate(&self) -> Result<()> {
        let dirs = vec![
            ("scripts_dir", &self.scripts_dir),
            ("jobs_dir", &self.jobs_dir),
            ("logs_dir", &self.logs_dir),
            ("runtimes_dir", &self.runtimes_dir),
            ("share_output_dir", &self.share_output_dir),
            ("share_import_dir", &self.share_import_dir),
            ("backup_dir", &self.backup_dir),
            ("restore_dir", &self.restore_dir),
            ("agents_dir", &self.agents_dir),
            ("plugins_dir", &self.plugins_dir),
            ("system_dir", &self.system_dir),
        ];

        for (name, path) in dirs {
            if !path.exists() {
                info!(
                    "Directory {} ({}) does not exist. Creating it...",
                    name,
                    path.display()
                );
                std::fs::create_dir_all(path).map_err(|e| {
                    anyhow::anyhow!("Failed to create {} at {}: {}", name, path.display(), e)
                })?;
            }
        }

        // Ensure database directory exists if it's sqlite
        if self.database_url.starts_with("sqlite://") {
            let path_str = self.database_url.trim_start_matches("sqlite://");
            // remove query params if any
            let path_clean = path_str.split('?').next().unwrap_or(path_str);
            let db_path = PathBuf::from(path_clean);
            if let Some(parent) = db_path.parent() {
                if !parent.exists() {
                    info!(
                        "Database directory {} does not exist. Creating it...",
                        parent.display()
                    );
                    std::fs::create_dir_all(parent).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to create database directory {}: {}",
                            parent.display(),
                            e
                        )
                    })?;
                }
            }
        }

        Ok(())
    }
}
