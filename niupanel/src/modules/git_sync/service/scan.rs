use regex::Regex;
use sea_orm::DatabaseConnection;
use tokio::io::AsyncReadExt;
use walkdir::WalkDir;

use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};

use crate::modules::git_sync::models::DiscoveredTask;

use super::GitService;

impl GitService {
    pub async fn scan_repo_tasks(db: &DatabaseConnection, id: i32) -> Result<Vec<DiscoveredTask>> {
        let repo = Self::get_repo(db, id).await?;
        let repo_base_dir = Self::get_repo_dir(&repo);

        if !repo_base_dir.exists() {
            return Err(AppError::NotFound(
                "Repository directory not found. Please sync first.".to_string(),
            ));
        }

        static RE_ENV: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        static RE_NAME: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        static RE_CRON: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

        let re_env = RE_ENV
            .get_or_init(|| Regex::new(r#"new\s+Env\(['"](.+?)['"]\)"#).expect("valid env regex"));
        let re_name = RE_NAME
            .get_or_init(|| Regex::new(r#"(@name|Name)[:\s]+(.+)"#).expect("valid name regex"));
        let re_cron = RE_CRON.get_or_init(|| {
            Regex::new(r#"(cron|@cron)[:\s]+(([\d\*/,-]+\s+){4,5}[\d\*/,-]+)"#)
                .expect("valid cron regex")
        });

        let mut discovered = Vec::new();
        let walker = WalkDir::new(&repo_base_dir).into_iter();

        for entry in
            walker.filter_entry(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            if entry.file_type().is_dir() {
                continue;
            }

            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|segment| segment.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !["js", "ts", "py", "sh"].contains(&ext.as_str()) {
                continue;
            }

            let mut file = match tokio::fs::File::open(path).await {
                Ok(file) => file,
                Err(_) => continue,
            };
            let mut buffer = [0; 4096];
            let bytes_read = match file.read(&mut buffer).await {
                Ok(bytes_read) => bytes_read,
                Err(_) => continue,
            };
            let content = String::from_utf8_lossy(&buffer[..bytes_read]);

            let name = re_env
                .captures(&content)
                .and_then(|captures| captures.get(1).map(|matched| matched.as_str().to_string()))
                .or_else(|| {
                    re_name.captures(&content).and_then(|captures| {
                        captures
                            .get(2)
                            .map(|matched| matched.as_str().trim().to_string())
                    })
                })
                .unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|segment| segment.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                });

            let cron = re_cron.captures(&content).and_then(|captures| {
                captures
                    .get(2)
                    .map(|matched| matched.as_str().trim().to_string())
            });

            let relative_path = path
                .strip_prefix(&repo_base_dir)
                .map(|relative| relative.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string());
            let full_server_path = format!(
                "{}/git/{}/{}",
                Config::global().scripts_dir.display(),
                Self::get_repo_dir_name(&repo.repo_url),
                relative_path
            );

            discovered.push(DiscoveredTask {
                name,
                command: Self::build_task_command(&ext, &full_server_path),
                cron,
                file_path: relative_path,
                description: None,
            });
        }

        Ok(discovered)
    }

    fn build_task_command(extension: &str, full_server_path: &str) -> String {
        match extension {
            "js" => format!("node {full_server_path}"),
            "ts" => format!("ts-node {full_server_path}"),
            "py" => format!("python3 {full_server_path}"),
            "sh" => format!("bash {full_server_path}"),
            _ => full_server_path.to_string(),
        }
    }
}
