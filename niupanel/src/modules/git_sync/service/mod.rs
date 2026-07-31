mod migration;
mod repo;
mod scan;
mod sync;

use std::path::{Path, PathBuf};

use niupanel_common::config::Config;
use niupanel_entity::git_repositories;
use std::collections::HashMap;

pub const KEY_GIT_REPO_URL: &str = "git.repo_url";
pub const KEY_GIT_BRANCH: &str = "git.branch";
pub const KEY_GIT_AUTH_TOKEN: &str = "git.auth_token";
pub const KEY_GIT_PROXY: &str = "git.proxy";
pub const KEY_GIT_AUTO_SYNC: &str = "git.auto_sync";

pub struct GitService;

impl GitService {
    pub fn get_repo_dir_name(repo_url: &str) -> String {
        let url = repo_url.trim_end_matches(".git").trim_end_matches('/');
        let path = if let Some(pos) = url.find("://") {
            let host_and_path = &url[pos + 3..];
            if let Some(slash_pos) = host_and_path.find('/') {
                &host_and_path[slash_pos + 1..]
            } else {
                host_and_path
            }
        } else if let Some(pos) = url.find('@') {
            if let Some(colon_pos) = url[pos..].find(':') {
                &url[pos + colon_pos + 1..]
            } else {
                url
            }
        } else {
            url
        };
        path.replace(['/', ':', '@', '.'], "_")
    }

    pub fn get_repo_dir(repo: &git_repositories::Model) -> PathBuf {
        let dir_name = Self::get_repo_dir_name(&repo.repo_url);
        Self::git_base_dir().join(dir_name)
    }

    pub(crate) fn git_base_dir() -> PathBuf {
        Path::new(&Config::global().scripts_dir).join("git")
    }

    pub(crate) fn proxy_env(proxy_url: Option<&str>) -> HashMap<String, String> {
        let mut envs = HashMap::new();
        if let Some(proxy) = proxy_url.filter(|proxy| !proxy.is_empty()) {
            envs.insert("http_proxy".to_string(), proxy.to_string());
            envs.insert("https_proxy".to_string(), proxy.to_string());
        }
        envs
    }
}
