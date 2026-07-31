use crate::runtime::RuntimeManager;
use crate::script::exec::cache::DependencyCache;
use crate::script::exec::dependency_lock::DependencyInstallLock;
use crate::script::exec::executor::Executor;
use crate::script::interpreter::node::NodeEnvironment;
use niupanel_common::error::{AppError, Result};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

/// Node.js 执行策略 — 版本共享依赖模式
///
/// 不再有"沙盒"概念：
/// - 运行时：使用 pnpm runtime 管理的版本（来自任务配置或 NiuPanel 默认版本）
/// - 依赖：安装在 data/runtimes/node/shared/<version>/node_modules
/// - 执行：直接调用该版本的 `node`，并注入共享依赖路径
pub struct NodeStrategy {
    env: Option<NodeEnvironment>,
    /// 任务解析后的 Node.js 版本（如 "20.11.1"）
    version: Option<String>,
    mirrors: Option<HashMap<String, String>>,
}

impl NodeStrategy {
    pub fn new(version: Option<&str>, mirrors: Option<HashMap<String, String>>) -> Self {
        Self {
            env: None,
            version: version.and_then(NodeEnvironment::normalize_version),
            mirrors,
        }
    }

    fn runtime_version(&self) -> Result<&str> {
        self.version
            .as_deref()
            .ok_or_else(|| AppError::Generic("Environment not initialized".to_string()))
    }
}

impl Executor for NodeStrategy {
    async fn prepare(&mut self) -> Result<()> {
        let version = if let Some(version) = self.version.clone() {
            version
        } else {
            NodeEnvironment::resolve_default_version()
                .await
                .map_err(|error| {
                    AppError::ValidationError(format!(
                        "No usable default Node.js runtime was found: {error}"
                    ))
                })?
        };

        self.env = Some(
            NodeEnvironment::new(
                std::path::PathBuf::new(),
                version.clone(),
                self.mirrors.clone(),
            )
            .await?,
        );
        self.version = Some(version.clone());

        debug!("NodeStrategy: using resolved version {}", version);

        Ok(())
    }

    async fn install_dependencies(
        &self,
        requirements: Option<&str>,
        _script_dir: &Path,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        let Some(req_str) = requirements else {
            return Ok(());
        };

        let packages: Vec<String> = req_str
            .split_whitespace()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if packages.is_empty() {
            return Ok(());
        }

        let version = self
            .version
            .as_deref()
            .ok_or_else(|| AppError::Generic("Environment not initialized".to_string()))?;
        let node_modules_dir = NodeEnvironment::shared_node_modules_for_version(version);
        let fingerprint = DependencyCache::node_fingerprint(
            &node_modules_dir,
            Some(version),
            req_str,
            self.mirrors.as_ref(),
        )?;
        let fingerprint_path =
            DependencyCache::node_fingerprint_marker_path(&node_modules_dir, &fingerprint);

        if DependencyCache::is_match(&fingerprint_path, &fingerprint).await {
            let _ = sender
                .send("[Node.js] Dependency declaration unchanged, skipping sync.".to_string());
            return Ok(());
        }

        let _install_lock = DependencyInstallLock::acquire(format!("node:{version}")).await;

        if DependencyCache::is_match(&fingerprint_path, &fingerprint).await {
            let _ = sender.send(
                "[Node.js] Dependency declaration already synced by another task.".to_string(),
            );
            return Ok(());
        }

        let env = self
            .env
            .as_ref()
            .ok_or_else(|| AppError::Generic("Environment not initialized".to_string()))?;
        debug!(
            "Node dependency fingerprint changed for {}, syncing full declaration.",
            version
        );
        let _ = sender.send("[Node.js] Dependency declaration changed, syncing...".to_string());
        RuntimeManager::install_node_packages(env, &packages, version, sender.clone()).await?;
        DependencyCache::write(&fingerprint_path, &fingerprint).await?;

        Ok(())
    }

    async fn build_command(
        &self,
        script_path: &Path,
        args: &[String],
        env_vars: &std::collections::HashMap<String, String>,
    ) -> std::process::Command {
        if let Some(env) = &self.env {
            let Ok(version) = self.runtime_version() else {
                let mut c = std::process::Command::new("sh");
                c.args([
                    "-c",
                    "echo 'Error: No Node.js runtime version resolved' >&2; exit 1",
                ]);
                return c;
            };
            RuntimeManager::build_node_command(env, script_path, version, args, env_vars)
                .await
                .unwrap_or_else(|_| {
                    let mut c = std::process::Command::new("sh");
                    c.args([
                        "-c",
                        "echo 'Error: Failed to build Node.js command' >&2; exit 1",
                    ]);
                    c
                })
        } else {
            let mut c = std::process::Command::new("sh");
            c.args([
                "-c",
                "echo 'Error: No Node.js environment configured' >&2; exit 1",
            ]);
            c
        }
    }

    fn name(&self) -> &str {
        "Node.js"
    }
}

#[cfg(test)]
mod tests {
    use super::NodeStrategy;

    #[test]
    fn missing_version_is_resolved_during_prepare() {
        let strategy = NodeStrategy::new(None, None);

        assert!(strategy.runtime_version().is_err());
    }

    #[test]
    fn explicit_version_is_normalized_for_pnpm_runtime() {
        let strategy = NodeStrategy::new(Some("v22.1.0"), None);

        assert_eq!(strategy.runtime_version().unwrap(), "22.1.0");
    }
}
