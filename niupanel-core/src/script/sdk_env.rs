use crate::script::interpreter::node::NodeEnvironment;
use niupanel_common::config::Config;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

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

    let preload_flag = format!("--require={}", preload_path.to_string_lossy());
    let mut options = existing_node_options(env);
    if !options.split_whitespace().any(|item| item == preload_flag) {
        if !options.is_empty() {
            options.push(' ');
        }
        options.push_str(&preload_flag);
    }

    env.insert("NODE_OPTIONS".to_string(), options);
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

pub async fn inject_node_runtime_path(env: &mut HashMap<String, String>) {
    let Some(node_bin_dir) = NodeEnvironment::get_default_node_bin_dir().await else {
        return;
    };

    let mut path_entries = Vec::new();
    path_entries.push(node_bin_dir);

    let existing = env
        .get("PATH")
        .cloned()
        .map(OsString::from)
        .or_else(|| std::env::var_os("PATH"));
    if let Some(existing) = existing {
        append_unique(
            &mut path_entries,
            std::env::split_paths(&existing).collect(),
        );
    }

    if let Some(joined) = join_paths_lossy(&path_entries) {
        env.insert("PATH".to_string(), joined);
    }
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
