use niupanel_plugin::{PluginInvokeRequest, PluginService};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};

static TEST_DATA_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();

#[tokio::test]
async fn invokes_external_plugin_package_when_requested() {
    let Some(source) = std::env::var_os("NIUPANEL_TEST_PLUGIN_DIR") else {
        return;
    };
    init_test_config();
    let root = tempfile::tempdir().expect("temp plugin root");
    let service = PluginService::new(root.path().join("plugins"), "plugin");
    let installed = service
        .install_from_dir(Path::new(&source), true)
        .expect("install external plugin");
    let response = service
        .invoke_plugin(
            &installed.manifest.id,
            PluginInvokeRequest {
                action: "health".to_string(),
                input: serde_json::Value::Null,
                timeout_sec: Some(10),
            },
        )
        .await
        .expect("invoke external plugin health");
    assert_eq!(response.output["ok"], true);
}

#[tokio::test]
async fn installs_and_invokes_first_enabled_process_plugin() {
    init_test_config();
    let root = tempfile::tempdir().expect("temp root");
    let package = tempfile::tempdir().expect("temp package");
    write_test_plugin(package.path());

    let service = PluginService::new(root.path().join("compiler"), "compiler");
    let installed = service
        .install_from_dir(package.path(), true)
        .expect("install plugin");

    assert_eq!(installed.manifest.id, "echo-compiler");
    assert!(installed.enabled);
    assert!(
        root.path()
            .join("compiler")
            .join("echo-compiler")
            .join("plugin.json")
            .is_file()
    );

    let response = service
        .invoke_first_enabled(PluginInvokeRequest {
            action: "versions".to_string(),
            input: serde_json::Value::Null,
            timeout_sec: Some(5),
        })
        .await
        .expect("invoke plugin")
        .expect("enabled plugin");

    assert_eq!(response.plugin_id, "echo-compiler");
    assert_eq!(response.extension_point, "compiler");
    assert_eq!(response.output, json!({ "versions": ["3.11"] }));
}

#[tokio::test]
async fn json_lines_plugin_can_call_injected_tools() {
    init_test_config();
    let root = tempfile::tempdir().expect("temp root");
    let package = tempfile::tempdir().expect("temp package");
    write_tool_calling_plugin(package.path());

    let service = PluginService::new(root.path().join("agents"), "agents");
    service
        .install_from_dir(package.path(), true)
        .expect("install plugin");

    let called = Arc::new(AtomicBool::new(false));
    let called_by_handler = called.clone();
    let response = service
        .invoke_plugin_with_tools(
            "tool-calling-agent",
            PluginInvokeRequest {
                action: "chat".to_string(),
                input: json!({ "message": "use MCP" }),
                timeout_sec: Some(5),
            },
            vec![json!({
                "name": "mcp__echo__echo",
                "description": "Echo text",
                "input_schema": {
                    "type": "object",
                    "properties": { "text": { "type": "string" } }
                }
            })],
            move |call| {
                assert_eq!(call.tool, "mcp__echo__echo");
                assert_eq!(call.input, json!({ "text": "hello MCP" }));
                called_by_handler.store(true, Ordering::SeqCst);
                Box::pin(async {
                    Ok(json!({ "content": [{ "type": "text", "text": "hello MCP" }] }))
                })
            },
        )
        .await
        .expect("invoke plugin with tools");

    assert!(called.load(Ordering::SeqCst));
    assert_eq!(response.output, json!({ "tool_result_received": true }));
}

#[tokio::test]
async fn process_sandbox_hides_host_files_and_clears_host_environment() {
    init_test_config();
    let root = tempfile::tempdir().expect("temp root");
    let package = tempfile::tempdir().expect("temp package");
    let host_guard_parent = std::env::var_os("CARGO_TARGET_DIR")
        .map(Into::into)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("workspace root")
                .join("target")
        });
    fs::create_dir_all(&host_guard_parent).expect("create host guard parent");
    let host_guard_dir = tempfile::tempdir_in(host_guard_parent).expect("host guard dir");
    let host_database = host_guard_dir.path().join("niupanel-dev.db");
    fs::write(&host_database, b"SQLite format 3\0panel-owned").expect("write database guard");
    let host_guard = host_guard_dir.path().join("host-guard.txt");
    fs::write(&host_guard, "unchanged").expect("write host guard");
    write_sandbox_probe_plugin(package.path(), &host_database, &host_guard);

    let service = PluginService::new(root.path().join("plugins"), "plugin");
    service
        .install_from_dir(package.path(), true)
        .expect("install sandbox probe");

    let response = service
        .invoke_plugin(
            "sandbox-probe",
            PluginInvokeRequest {
                action: "probe".to_string(),
                input: serde_json::Value::Null,
                timeout_sec: Some(5),
            },
        )
        .await
        .expect("invoke sandbox probe");

    assert_eq!(response.output["host_file_visible"], false);
    assert_eq!(response.output["host_file_modified"], false);
    assert_eq!(response.output["etc_passwd_visible"], false);
    assert_eq!(response.output["network_socket_created"], false);
    assert_eq!(response.output["database_url"], "missing");
    assert_eq!(response.output["custom_value"], "available");
    assert_eq!(response.output["plugin_data"], "plugin-owned");
    if unsafe { libc::geteuid() } == 0 {
        assert_eq!(response.output["runtime_uid"], "65534");
    }
    assert_eq!(
        fs::read_to_string(host_guard).expect("read host guard"),
        "unchanged"
    );
}

#[test]
fn rejects_reserved_plugin_environment_variables() {
    init_test_config();
    let root = tempfile::tempdir().expect("temp root");
    let package = tempfile::tempdir().expect("temp package");
    write_test_plugin(package.path());

    let manifest_path = package.path().join("plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).expect("read manifest"))
            .expect("parse manifest");
    manifest["env"] = json!({ "DATABASE_URL": "sqlite:///host/niupanel.db" });
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");

    let service = PluginService::new(root.path().join("plugins"), "plugin");
    let error = service
        .install_from_dir(package.path(), false)
        .expect_err("reserved environment must be rejected");
    assert!(error.to_string().contains("DATABASE_URL"));
}

fn write_test_plugin(path: &Path) {
    write_test_plugin_with_version(path, "0.1.0");
}

fn init_test_config() {
    let root = TEST_DATA_DIR.get_or_init(|| tempfile::tempdir().expect("test data dir"));
    let plugins_dir = root.path().join("plugins");
    let runtimes_dir = root.path().join("runtimes");
    fs::create_dir_all(&plugins_dir).expect("plugins data dir");
    fs::create_dir_all(&runtimes_dir).expect("runtimes data dir");
    let config = serde_json::from_value(json!({
        "plugins_dir": plugins_dir,
        "runtimes_dir": runtimes_dir
    }))
    .expect("test config");
    let _ = niupanel_common::config::CONFIG.set(config);
}

#[test]
fn updates_archive_and_rolls_back_plugin_version() {
    let root = tempfile::tempdir().expect("temp root");
    let package_v1 = tempfile::tempdir().expect("temp package v1");
    let package_v2 = tempfile::tempdir().expect("temp package v2");
    write_test_plugin_with_version(package_v1.path(), "0.1.0");
    write_test_plugin_with_version(package_v2.path(), "0.2.0");

    let service = PluginService::new(root.path().join("compiler"), "compiler");
    service
        .install_from_dir(package_v1.path(), true)
        .expect("install plugin");
    let updated = service
        .update_from_dir("echo-compiler", package_v2.path())
        .expect("update plugin");

    assert_eq!(updated.manifest.version, "0.2.0");
    let versions = service
        .list_versions("echo-compiler")
        .expect("list versions");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].version, "0.1.0");

    let restored = service
        .rollback("echo-compiler", &versions[0].id)
        .expect("rollback plugin");
    assert_eq!(restored.manifest.version, "0.1.0");

    let versions = service
        .list_versions("echo-compiler")
        .expect("list versions after rollback");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].version, "0.2.0");
}

fn write_test_plugin_with_version(path: &Path, version: &str) {
    fs::write(
        path.join("plugin.json"),
        format!(
            r#"{{
  "schema_version": 1,
  "id": "echo-compiler",
  "name": "Echo Compiler",
  "version": "{version}",
  "description": "Test compiler plugin",
  "extension_points": ["compiler"],
  "runtime": "process",
  "protocol": "single_shot",
  "entry": "run.sh"
}}"#
        ),
    )
    .expect("write manifest");

    fs::write(
        path.join("run.sh"),
        r#"#!/usr/bin/env bash
set -euo pipefail
request="$(cat)"
request_id="$(printf '%s' "$request" | sed -n 's/.*"request_id":"\([^"]*\)".*/\1/p')"
printf '{"request_id":"%s","ok":true,"output":{"versions":["3.11"]}}\n' "$request_id"
"#,
    )
    .expect("write runner");

    make_executable(path.join("run.sh").as_path());
}

fn write_tool_calling_plugin(path: &Path) {
    fs::write(
        path.join("plugin.json"),
        r#"{
  "schema_version": 1,
  "id": "tool-calling-agent",
  "name": "Tool Calling Agent",
  "version": "0.1.0",
  "description": "Test json_lines tool bridge",
  "extension_points": ["agents"],
  "runtime": "process",
  "protocol": "json_lines",
  "entry": "run.sh",
  "capabilities": ["agents.invoke"]
}"#,
    )
    .expect("write manifest");

    fs::write(
        path.join("run.sh"),
        r#"#!/usr/bin/env bash
set -euo pipefail
while IFS= read -r request; do
  request_id="$(printf '%s' "$request" | sed -n 's/.*"request_id":"\([^"]*\)".*/\1/p')"
  case "$request" in
    *'"name":"mcp__echo__echo"'*) ;;
    *) exit 2 ;;
  esac
  printf '{"type":"tool_call","request_id":"%s","call_id":"call-1","tool":"mcp__echo__echo","input":{"text":"hello MCP"}}\n' "$request_id"
  IFS= read -r tool_result
  case "$tool_result" in
    *'"type":"tool_result"'*'"ok":true'*) ;;
    *) exit 3 ;;
  esac
  printf '{"request_id":"%s","ok":true,"output":{"tool_result_received":true}}\n' "$request_id"
done
"#,
    )
    .expect("write runner");

    make_executable(path.join("run.sh").as_path());
}

fn write_sandbox_probe_plugin(path: &Path, host_database: &Path, host_guard: &Path) {
    fs::write(
        path.join("plugin.json"),
        r#"{
  "schema_version": 1,
  "id": "sandbox-probe",
  "name": "Sandbox Probe",
  "version": "0.1.0",
  "description": "Verifies process sandbox boundaries",
  "runtime": "process",
  "protocol": "single_shot",
  "entry": "run.sh",
  "env": { "CUSTOM_VALUE": "available" }
}"#,
    )
    .expect("write sandbox manifest");

    let host_database = host_database.display().to_string().replace('"', "\\\"");
    let host_guard = host_guard.display().to_string().replace('"', "\\\"");
    fs::write(
        path.join("run.sh"),
        format!(
            r#"#!/usr/bin/env bash
set -euo pipefail
request="$(cat)"
request_id="$(printf '%s' "$request" | sed -n 's/.*"request_id":"\([^"]*\)".*/\1/p')"
host_file_visible=false
if cat "{host_database}" >/dev/null 2>&1; then host_file_visible=true; fi
host_file_modified=false
if printf 'modified' >> "{host_guard}" 2>/dev/null; then host_file_modified=true; fi
etc_passwd_visible=false
if cat /etc/passwd >/dev/null 2>&1; then etc_passwd_visible=true; fi
network_socket_created=false
if python3 -B -c 'import socket; socket.socket()' >/dev/null 2>&1; then network_socket_created=true; fi
printf 'plugin-owned' > "$NIUPANEL_PLUGIN_DATA_DIR/state.txt"
database_url="${{DATABASE_URL:-missing}}"
custom_value="${{CUSTOM_VALUE:-missing}}"
plugin_data="$(cat "$NIUPANEL_PLUGIN_DATA_DIR/state.txt")"
runtime_uid="$(id -u)"
printf '{{"request_id":"%s","ok":true,"output":{{"host_file_visible":%s,"host_file_modified":%s,"etc_passwd_visible":%s,"network_socket_created":%s,"database_url":"%s","custom_value":"%s","plugin_data":"%s","runtime_uid":"%s"}}}}\n' \
  "$request_id" "$host_file_visible" "$host_file_modified" "$etc_passwd_visible" "$network_socket_created" "$database_url" "$custom_value" "$plugin_data" "$runtime_uid"
"#
        ),
    )
    .expect("write sandbox runner");
    make_executable(path.join("run.sh").as_path());
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).expect("runner metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("runner permissions");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}
