use niupanel_plugin::{
    PluginActionCaller, PluginActionInvokeRequest, PluginInvokeRequest, PluginService,
};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::sync::CancellationToken;

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
    let action =
        std::env::var("NIUPANEL_TEST_PLUGIN_ACTION").unwrap_or_else(|_| "health".to_string());
    let input = std::env::var("NIUPANEL_TEST_PLUGIN_INPUT")
        .ok()
        .map(|value| serde_json::from_str(&value).expect("valid external plugin input"))
        .unwrap_or(serde_json::Value::Null);
    let response = service
        .invoke_plugin(
            &installed.manifest.id,
            PluginInvokeRequest {
                action: action.clone(),
                input,
                timeout_sec: Some(10),
            },
        )
        .await
        .expect("invoke external plugin action");
    if action == "health" {
        assert_eq!(response.output["ok"], true);
    } else {
        assert!(response.output.is_object());
    }
}

#[tokio::test]
async fn action_manifest_filters_callers_and_validates_input() {
    init_test_config();
    let root = tempfile::tempdir().expect("temp root");
    let package = tempfile::tempdir().expect("temp package");
    write_action_plugin(package.path());

    let service = PluginService::new(root.path().join("plugins"), "plugin");
    service
        .install_from_dir(package.path(), true)
        .expect("install action plugin");

    assert_eq!(
        service
            .list_action_plugins(PluginActionCaller::ApiKey)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        service
            .list_plugin_actions("action-echo", PluginActionCaller::Task)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        service
            .list_plugin_actions("action-echo", PluginActionCaller::Telegram)
            .unwrap()
            .len(),
        1
    );
    assert!(
        service
            .invoke_action(
                "action-echo",
                PluginActionCaller::ApiKey,
                PluginActionInvokeRequest {
                    action: "query".to_string(),
                    input: json!({}),
                    client_request_id: None,
                },
            )
            .await
            .is_err()
    );
    let response = service
        .invoke_action(
            "action-echo",
            PluginActionCaller::ApiKey,
            PluginActionInvokeRequest {
                action: "query".to_string(),
                input: json!({"keyword": "test"}),
                client_request_id: None,
            },
        )
        .await
        .expect("invoke action");
    assert_eq!(response.output, json!({"ok": true}));
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
async fn json_lines_plugin_streams_ordered_events_before_final_result() {
    init_test_config();
    let root = tempfile::tempdir().expect("temp root");
    let package = tempfile::tempdir().expect("temp package");
    write_streaming_plugin(package.path());

    let service = PluginService::new(root.path().join("plugins"), "plugin");
    service
        .install_from_dir(package.path(), true)
        .expect("install streaming plugin");

    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured_events = events.clone();
    let response = service
        .invoke_action_stream_with_tools_cancellable(
            "streaming-agent",
            PluginActionCaller::Task,
            PluginActionInvokeRequest {
                action: "chat".to_string(),
                input: json!({"message": "hello"}),
                client_request_id: None,
            },
            Vec::new(),
            niupanel_plugin::PluginStreamHandlers::new(
                |_| {
                    Box::pin(async { Ok(serde_json::Value::Null) })
                        as niupanel_plugin::PluginToolFuture
                },
                move |event| {
                    let captured_events = captured_events.clone();
                    Box::pin(async move {
                        captured_events.lock().expect("events lock").push(event);
                        Ok(())
                    }) as niupanel_plugin::PluginStreamFuture
                },
            ),
            CancellationToken::new(),
        )
        .await
        .expect("streaming invocation");

    let events = events.lock().expect("events lock");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].sequence, 1);
    assert_eq!(events[0].event, "start");
    assert_eq!(events[1].sequence, 2);
    assert_eq!(events[1].event, "delta");
    assert_eq!(events[1].data["text"], "hello");
    assert_eq!(response.output, json!({"message": "hello"}));
}

#[tokio::test]
async fn cancelling_json_lines_invocation_discards_worker_and_releases_capacity() {
    init_test_config();
    let root = tempfile::tempdir().expect("temp root");
    let package = tempfile::tempdir().expect("temp package");
    write_cancellable_plugin(package.path());

    let service = PluginService::new(root.path().join("agents"), "agents");
    service
        .install_from_dir(package.path(), true)
        .expect("install cancellable plugin");

    let marker = niupanel_common::config::Config::global()
        .plugins_dir
        .join(".data/agents/cancellable-agent/started-cancel");
    let _ = fs::remove_file(&marker);
    let cancellation = CancellationToken::new();
    let invocation = {
        let service = service.clone();
        let cancellation = cancellation.clone();
        tokio::spawn(async move {
            service
                .invoke_action_with_tools_cancellable(
                    "cancellable-agent",
                    PluginActionCaller::Telegram,
                    PluginActionInvokeRequest {
                        action: "chat".to_string(),
                        input: json!({"message": "slow-cancel"}),
                        client_request_id: None,
                    },
                    Vec::new(),
                    |_| Box::pin(async { Ok(serde_json::Value::Null) }),
                    cancellation,
                )
                .await
        })
    };

    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !marker.is_file() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("slow worker started");
    cancellation.cancel();
    let error = invocation
        .await
        .expect("invocation task")
        .expect_err("invocation should be cancelled");
    assert!(matches!(error, niupanel_common::error::AppError::Cancelled));

    let response = service
        .invoke_action_with_tools_cancellable(
            "cancellable-agent",
            PluginActionCaller::Telegram,
            PluginActionInvokeRequest {
                action: "chat".to_string(),
                input: json!({"message": "fast"}),
                client_request_id: None,
            },
            Vec::new(),
            |_| Box::pin(async { Ok(serde_json::Value::Null) }),
            CancellationToken::new(),
        )
        .await
        .expect("replacement worker accepts next invocation");
    assert_eq!(response.output, json!({"message": "fast"}));
}

#[tokio::test]
async fn aborting_json_lines_invocation_releases_worker_and_process_capacity() {
    init_test_config();
    let root = tempfile::tempdir().expect("temp root");
    let package = tempfile::tempdir().expect("temp package");
    write_cancellable_plugin(package.path());

    let service = PluginService::new(root.path().join("agents"), "agents");
    service
        .install_from_dir(package.path(), true)
        .expect("install cancellable plugin");

    let marker = niupanel_common::config::Config::global()
        .plugins_dir
        .join(".data/agents/cancellable-agent/started-abort");
    let _ = fs::remove_file(&marker);
    let invocation = {
        let service = service.clone();
        tokio::spawn(async move {
            service
                .invoke_action_with_tools_cancellable(
                    "cancellable-agent",
                    PluginActionCaller::Telegram,
                    PluginActionInvokeRequest {
                        action: "chat".to_string(),
                        input: json!({"message": "slow-abort"}),
                        client_request_id: None,
                    },
                    Vec::new(),
                    |_| Box::pin(async { Ok(serde_json::Value::Null) }),
                    CancellationToken::new(),
                )
                .await
        })
    };

    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !marker.is_file() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("slow worker started");
    invocation.abort();
    assert!(
        invocation
            .await
            .expect_err("invocation task should be aborted")
            .is_cancelled()
    );

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        service.invoke_action_with_tools_cancellable(
            "cancellable-agent",
            PluginActionCaller::Telegram,
            PluginActionInvokeRequest {
                action: "chat".to_string(),
                input: json!({"message": "fast-after-abort"}),
                client_request_id: None,
            },
            Vec::new(),
            |_| Box::pin(async { Ok(serde_json::Value::Null) }),
            CancellationToken::new(),
        ),
    )
    .await
    .expect("replacement invocation did not wait for leaked capacity")
    .expect("replacement worker accepts invocation after task abort");
    assert_eq!(response.output, json!({"message": "fast"}));
}

#[tokio::test]
async fn cancellable_invocation_waits_for_a_busy_plugin_worker() {
    init_test_config();
    let root = tempfile::tempdir().expect("temp root");
    let package = tempfile::tempdir().expect("temp package");
    write_cancellable_plugin(package.path());

    let service = PluginService::new(root.path().join("agents"), "agents");
    service
        .install_from_dir(package.path(), true)
        .expect("install cancellable plugin");

    let marker = niupanel_common::config::Config::global()
        .plugins_dir
        .join(".data/agents/cancellable-agent/started-queue");
    let _ = fs::remove_file(&marker);
    let first_cancellation = CancellationToken::new();
    let first = {
        let service = service.clone();
        let cancellation = first_cancellation.clone();
        tokio::spawn(async move {
            service
                .invoke_action_with_tools_cancellable(
                    "cancellable-agent",
                    PluginActionCaller::Telegram,
                    PluginActionInvokeRequest {
                        action: "chat".to_string(),
                        input: json!({"message": "slow-queue"}),
                        client_request_id: None,
                    },
                    Vec::new(),
                    |_| Box::pin(async { Ok(serde_json::Value::Null) }),
                    cancellation,
                )
                .await
        })
    };
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !marker.is_file() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("first worker started");

    let second = {
        let service = service.clone();
        tokio::spawn(async move {
            service
                .invoke_action_with_tools_cancellable(
                    "cancellable-agent",
                    PluginActionCaller::Telegram,
                    PluginActionInvokeRequest {
                        action: "chat".to_string(),
                        input: json!({"message": "queued-fast"}),
                        client_request_id: None,
                    },
                    Vec::new(),
                    |_| Box::pin(async { Ok(serde_json::Value::Null) }),
                    CancellationToken::new(),
                )
                .await
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert!(
        !second.is_finished(),
        "second invocation must queue instead of returning a concurrency error"
    );

    first_cancellation.cancel();
    let first_error = first
        .await
        .expect("first invocation task")
        .expect_err("first invocation should be cancelled");
    assert!(matches!(
        first_error,
        niupanel_common::error::AppError::Cancelled
    ));
    let response = tokio::time::timeout(std::time::Duration::from_secs(3), second)
        .await
        .expect("queued invocation resumed")
        .expect("queued invocation task")
        .expect("queued invocation succeeds");
    assert_eq!(response.output, json!({"message": "fast"}));
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
    assert_eq!(
        response.output["python_process_started"], true,
        "sandboxed Python result: {}",
        response.output
    );
    assert_eq!(response.output["python_exit_code"], 0);
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

#[test]
fn prepared_packages_are_moved_and_keep_package_digests() {
    let root = tempfile::tempdir().expect("temp root");
    let package_v1 = tempfile::tempdir().expect("temp package v1");
    let package_v2 = tempfile::tempdir().expect("temp package v2");
    write_test_plugin_with_version(package_v1.path(), "0.1.0");
    write_test_plugin_with_version(package_v2.path(), "0.2.0");

    let source_v1 = package_v1.path().to_path_buf();
    let source_v2 = package_v2.path().to_path_buf();
    let service = PluginService::new(root.path().join("compiler"), "compiler");
    service
        .install_prepared_dir(&source_v1, true, "digest-v1".to_string())
        .expect("install prepared plugin");
    assert!(!source_v1.exists());

    let updated = service
        .update_prepared_dir("echo-compiler", &source_v2, "digest-v2".to_string())
        .expect("update prepared plugin");
    assert_eq!(updated.manifest.version, "0.2.0");
    assert!(!source_v2.exists());

    let versions = service
        .list_versions("echo-compiler")
        .expect("list archived versions");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].package_sha256.as_deref(), Some("digest-v1"));
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

fn write_action_plugin(path: &Path) {
    fs::write(
        path.join("plugin.json"),
        r#"{
  "schema_version": 2,
  "id": "action-echo",
  "name": "Action Echo",
  "version": "0.1.0",
  "description": "Action contract test plugin",
  "runtime": "process",
  "protocol": "single_shot",
  "entry": "run.sh",
  "actions": [
    {
      "name": "query",
      "callers": ["task", "api_key", "telegram"],
      "timeout_sec": 10,
      "input_schema": {
        "type": "object",
        "required": ["keyword"],
        "properties": {"keyword": {"type": "string"}},
        "additionalProperties": false
      }
    }
  ]
}"#,
    )
    .expect("write action manifest");
    fs::write(
        path.join("run.sh"),
        r#"#!/usr/bin/env bash
set -euo pipefail
request_id="$(sed -n 's/.*"request_id":"\([^"]*\)".*/\1/p')"
printf '{"request_id":"%s","ok":true,"output":{"ok":true}}\n' "$request_id"
"#,
    )
    .expect("write action runner");
    make_executable(path.join("run.sh").as_path());
}

fn write_tool_calling_plugin(path: &Path) {
    fs::write(
        path.join("plugin.json"),
        r#"{
  "schema_version": 2,
  "id": "tool-calling-agent",
  "name": "Tool Calling Agent",
  "version": "0.1.0",
  "description": "Test json_lines tool bridge",
  "extension_points": ["agents"],
  "runtime": "process",
  "protocol": "json_lines",
  "entry": "run.sh",
  "actions": [
    {
      "name": "run",
      "callers": ["task"],
      "timeout_sec": 30
    }
  ]
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

fn write_cancellable_plugin(path: &Path) {
    fs::write(
        path.join("plugin.json"),
        r#"{
  "schema_version": 2,
  "id": "cancellable-agent",
  "name": "Cancellable Agent",
  "version": "0.1.0",
  "description": "Cancellation-safe worker pool test",
  "extension_points": ["agents"],
  "runtime": "process",
  "protocol": "json_lines",
  "entry": "run.sh",
  "worker": {"min": 1, "max": 1, "idle_timeout_sec": 60},
  "actions": [
    {
      "name": "chat",
      "callers": ["telegram"],
      "timeout_sec": 60,
      "input_schema": {
        "type": "object",
        "required": ["message"],
        "properties": {"message": {"type": "string"}},
        "additionalProperties": false
      }
    }
  ]
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
    *'"message":"slow-cancel"'*)
      printf 'started' > "$NIUPANEL_PLUGIN_DATA_DIR/started-cancel"
      sleep 30
      ;;
    *'"message":"slow-abort"'*)
      printf 'started' > "$NIUPANEL_PLUGIN_DATA_DIR/started-abort"
      sleep 30
      ;;
    *'"message":"slow-queue"'*)
      printf 'started' > "$NIUPANEL_PLUGIN_DATA_DIR/started-queue"
      sleep 30
      ;;
  esac
  printf '{"request_id":"%s","ok":true,"output":{"message":"fast"}}\n' "$request_id"
done
"#,
    )
    .expect("write runner");
    make_executable(path.join("run.sh").as_path());
}

fn write_streaming_plugin(path: &Path) {
    fs::write(
        path.join("plugin.json"),
        r#"{
  "schema_version": 2,
  "id": "streaming-agent",
  "name": "Streaming Agent",
  "version": "0.1.0",
  "description": "Streaming protocol test",
  "runtime": "process",
  "protocol": "json_lines",
  "entry": "run.sh",
  "actions": [
    {
      "name": "chat",
      "callers": ["task"],
      "timeout_sec": 10,
      "streaming": true,
      "input_schema": {
        "type": "object",
        "required": ["message"],
        "properties": {"message": {"type": "string"}},
        "additionalProperties": false
      }
    }
  ]
}"#,
    )
    .expect("write streaming manifest");
    fs::write(
        path.join("run.sh"),
        r#"#!/usr/bin/env bash
set -euo pipefail
while IFS= read -r request; do
  request_id="$(printf '%s' "$request" | sed -n 's/.*"request_id":"\([^"]*\)".*/\1/p')"
  case "$request" in
    *'"stream":true'*) ;;
    *) exit 2 ;;
  esac
  printf '{"type":"stream_event","request_id":"%s","sequence":1,"event":"start","data":{"ready":true}}\n' "$request_id"
  printf '{"type":"stream_event","request_id":"%s","sequence":2,"event":"delta","data":{"text":"hello"}}\n' "$request_id"
  printf '{"request_id":"%s","ok":true,"output":{"message":"hello"}}\n' "$request_id"
done
"#,
    )
    .expect("write streaming runner");
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
python_process_started=false
set +e
python_result="$(python3 -B -c 'print("started")' 2>&1)"
python_exit_code=$?
set -e
if [ "$python_result" = started ]; then python_process_started=true; fi
python_error="$(printf '%s' "$python_result" | tr '\n' ' ' | sed 's/"/'"'"'/g')"
printf 'plugin-owned' > "$NIUPANEL_PLUGIN_DATA_DIR/state.txt"
database_url="${{DATABASE_URL:-missing}}"
custom_value="${{CUSTOM_VALUE:-missing}}"
plugin_data="$(cat "$NIUPANEL_PLUGIN_DATA_DIR/state.txt")"
runtime_uid="$(id -u)"
printf '{{"request_id":"%s","ok":true,"output":{{"host_file_visible":%s,"host_file_modified":%s,"etc_passwd_visible":%s,"network_socket_created":%s,"python_process_started":%s,"python_exit_code":%s,"python_error":"%s","database_url":"%s","custom_value":"%s","plugin_data":"%s","runtime_uid":"%s"}}}}\n' \
  "$request_id" "$host_file_visible" "$host_file_modified" "$etc_passwd_visible" "$network_socket_created" "$python_process_started" "$python_exit_code" "$python_error" "$database_url" "$custom_value" "$plugin_data" "$runtime_uid"
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
