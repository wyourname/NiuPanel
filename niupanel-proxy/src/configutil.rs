use crate::error::Errors;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub cfhost: String,
    pub cfip: String,
    pub token: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub passwd: String,
    pub log: PathBuf,
    pub loglevel: String,
}

impl Config {
    pub fn load_from_file(path: &PathBuf) -> Result<Self, Errors> {
        let content = fs::read_to_string(path).map_err(Errors::IoError)?;
        let config: Config = serde_json::from_str(&content).map_err(Errors::ConfigError)?;

        if config.cfhost.is_empty() {
            return Err(Errors::ValidationError(
                "Missing the required config 'cfhost'!".into(),
            ));
        }
        Ok(config)
    }

    pub fn from_json(content: &str) -> Result<Self, Errors> {
        let config: Config = serde_json::from_str(content).map_err(Errors::ConfigError)?;
        Ok(config)
    }
}

// Default configuration
impl Default for Config {
    fn default() -> Self {
        Self {
            cfhost: String::new(),
            cfip: "104.16.0.0".into(), // CFIP
            token: String::new(),
            host: "127.0.0.1".into(),
            port: 4514,
            user: String::new(),
            passwd: String::new(),
            log: PathBuf::new(),
            loglevel: "info".into(),
        }
    }
}
