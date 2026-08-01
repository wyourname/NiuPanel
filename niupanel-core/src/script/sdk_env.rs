use crate::script::interpreter::node::NodeEnvironment;
use niupanel_common::config::Config;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn inject_python_sdk_path(env: &mut HashMap<String, String>) {
    let sdk_paths = config_paths(&Config::global().extra_python_path);
    if sdk_paths.is_empty() {
        return;
    }

    let existing = env
        .get("PYTHONPATH")
        .cloned()
        .or_else(|| std::env::var("PYTHONPATH").ok());
    if let Some(joined) = merge_path_like_env(sdk_paths, existing.as_deref()) {
        env.insert("PYTHONPATH".to_string(), joined);
    }
}

pub fn inject_node_sdk_path(env: &mut HashMap<String, String>) {
    let mut node_paths = config_paths(&Config::global().extra_node_path);

    let existing = env
        .get("NODE_PATH")
        .cloned()
        .or_else(|| std::env::var("NODE_PATH").ok());
    if let Some(existing) = existing.as_deref() {
        append_unique(&mut node_paths, std::env::split_paths(existing).collect());
    }

    if let Some(joined) = join_paths_lossy(&node_paths) {
        env.insert("NODE_PATH".to_string(), joined);
    }
}

pub fn inject_node_sdk_preload(env: &mut HashMap<String, String>) {
    let Some(preload_path) = first_config_path(&Config::global().extra_node_path)
        .map(|path| path.join("niu").join("preload.js"))
        .filter(|path| path.exists())
    else {
        ensure_node_memory_limit(env);
        return;
    };

    append_node_option(env, format!("--require={}", preload_path.to_string_lossy()));
    ensure_node_memory_limit(env);
}

pub fn ensure_node_memory_limit(env: &mut HashMap<String, String>) {
    let mut options = existing_node_options(env);
    if !options.contains("--max-old-space-size") {
        if !options.is_empty() {
            options.push(' ');
        }
        options.push_str("--max-old-space-size=2048");
        env.insert("NODE_OPTIONS".to_string(), options);
    }
}

/// 注入默认 Node.js 运行时及其版本共享的第三方依赖。
///
/// Python、Shell 等非 Node 执行器可以通过 `subprocess` 或子进程调用 `node`。
/// 仅将 Node 二进制加入 `PATH` 会让该子进程找不到由 NiuPanel 管理的 npm
/// 依赖，因此这里必须与 NodeStrategy 使用同一份 `PATH` 和 `NODE_PATH` 配置。
pub async fn inject_default_node_runtime_environment(env: &mut HashMap<String, String>) {
    let Ok(version) = NodeEnvironment::resolve_default_version().await else {
        return;
    };

    let node_modules = NodeEnvironment::shared_node_modules_for_version(&version);
    let bin_dir = NodeEnvironment::shared_bin_for_version(&version);
    inject_node_runtime_environment_for_paths(env, &node_modules, &bin_dir);
    inject_node_shared_dependency_loader(env, &version, &node_modules);
}

/// Makes bare ESM imports fall back to the managed dependency directory.
///
/// CommonJS already uses `NODE_PATH`, but Node's ESM resolver intentionally
/// ignores it. The loader preserves normal local resolution and only handles
/// missing bare package specifiers.
pub fn inject_node_shared_dependency_loader(
    env: &mut HashMap<String, String>,
    version: &str,
    node_modules: &Path,
) {
    let Some(sdk_path) =
        first_config_path(&Config::global().extra_node_path).map(|path| path.join("niu"))
    else {
        return;
    };
    let loader_path = sdk_path.join("shared-dependencies-loader.mjs");
    let register_path = sdk_path.join("shared-dependencies-register.mjs");
    let option = if node_supports_module_register(version) && register_path.is_file() {
        format!("--import={}", register_path.to_string_lossy())
    } else if loader_path.is_file() {
        format!("--experimental-loader={}", loader_path.to_string_lossy())
    } else {
        return;
    };

    env.insert(
        "NIUPANEL_NODE_SHARED_MODULES".to_string(),
        node_modules.to_string_lossy().to_string(),
    );
    append_node_option(env, option);
}

fn inject_node_runtime_environment_for_paths(
    env: &mut HashMap<String, String>,
    node_modules: &std::path::Path,
    bin_dir: &std::path::Path,
) {
    NodeEnvironment::inject_dependency_paths(env, node_modules, bin_dir);
}

fn config_paths(value: &Option<String>) -> Vec<PathBuf> {
    value
        .as_deref()
        .map(std::env::split_paths)
        .map(Iterator::collect)
        .unwrap_or_default()
}

fn first_config_path(value: &Option<String>) -> Option<PathBuf> {
    config_paths(value).into_iter().next()
}

fn existing_node_options(env: &HashMap<String, String>) -> String {
    env.get("NODE_OPTIONS")
        .cloned()
        .or_else(|| std::env::var("NODE_OPTIONS").ok())
        .unwrap_or_default()
}

fn append_node_option(env: &mut HashMap<String, String>, option: String) {
    let mut options = existing_node_options(env);
    if !options.split_whitespace().any(|item| item == option) {
        if !options.is_empty() {
            options.push(' ');
        }
        options.push_str(&option);
    }
    env.insert("NODE_OPTIONS".to_string(), options);
}

fn node_supports_module_register(version: &str) -> bool {
    let mut parts = version
        .trim()
        .trim_start_matches('v')
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok());
    let major = parts.next().unwrap_or_default();
    let minor = parts.next().unwrap_or_default();
    major > 20 || (major == 20 && minor >= 6)
}

fn merge_path_like_env(prefixes: Vec<PathBuf>, existing: Option<&str>) -> Option<String> {
    let mut paths = prefixes;
    if let Some(existing) = existing {
        append_unique(&mut paths, std::env::split_paths(existing).collect());
    }
    join_paths_lossy(&paths)
}

fn join_paths_lossy(paths: &[PathBuf]) -> Option<String> {
    std::env::join_paths(paths)
        .ok()
        .map(|joined| joined.to_string_lossy().to_string())
}

fn append_unique(target: &mut Vec<PathBuf>, entries: Vec<PathBuf>) {
    for entry in entries {
        if !target.contains(&entry) {
            target.push(entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_runtime_environment_exposes_binary_and_shared_dependencies() {
        let node_modules = PathBuf::from("/managed/node/node_modules");
        let bin_dir = node_modules.join(".bin");
        let mut env = HashMap::new();
        inject_node_runtime_environment_for_paths(&mut env, &node_modules, &bin_dir);

        let path_entries = std::env::split_paths(
            &env.get("PATH")
                .expect("Node runtime should prepend its bin directory"),
        )
        .collect::<Vec<_>>();
        let node_path_entries = std::env::split_paths(
            &env.get("NODE_PATH")
                .expect("Node runtime should expose shared node_modules"),
        )
        .collect::<Vec<_>>();

        assert_eq!(path_entries.first(), Some(&bin_dir));
        assert_eq!(node_path_entries.first(), Some(&node_modules));
    }

    #[test]
    fn module_register_support_matches_supported_node_versions() {
        assert!(node_supports_module_register("20.6.0"));
        assert!(node_supports_module_register("22.11.0"));
        assert!(!node_supports_module_register("20.5.1"));
        assert!(!node_supports_module_register("18.20.4"));
    }
}
