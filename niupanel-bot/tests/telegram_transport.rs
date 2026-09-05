use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::Uri;
use axum::response::{IntoResponse, Response};
use niupanel_bot::telegram::agent::{
    TelegramAgentError, TelegramAgentHandler, TelegramAgentResponse,
};
use niupanel_bot::telegram::context::NotificationContextStore;
use niupanel_bot::telegram::ledger::TelegramMessageLedger;
use niupanel_bot::telegram::runtime::TelegramAgentRuntime;
use niupanel_bot::telegram::{TelegramBotConfig, TelegramChatBinding};
use niupanel_bot::{TelegramBot, telegram};
use niupanel_core::event_bus::EventBus;
use sea_orm::Database;
use serde_json::{Value, json};
use teloxide::Bot;
use teloxide::types::{Me, Message};
use tokio::sync::{Mutex, Notify, mpsc};
use tokio_util::sync::CancellationToken;

const TOKEN: &str = "123456:test-token";

#[derive(Debug, Clone)]
struct ApiCall {
    method: String,
    body: Value,
}

#[derive(Clone)]
struct FakeTelegramState {
    calls: Arc<Mutex<Vec<ApiCall>>>,
    updates: Arc<Mutex<VecDeque<Value>>>,
    file: Arc<Vec<u8>>,
    next_message_id: Arc<AtomicI32>,
}

struct FakeTelegram {
    base_url: String,
    state: FakeTelegramState,
    shutdown: CancellationToken,
}

impl FakeTelegram {
    async fn start(file: impl Into<Vec<u8>>) -> Self {
        let state = FakeTelegramState {
            calls: Arc::new(Mutex::new(Vec::new())),
            updates: Arc::new(Mutex::new(VecDeque::new())),
            file: Arc::new(file.into()),
            next_message_id: Arc::new(AtomicI32::new(1_000)),
        };
        let router = Router::new()
            .fallback(fake_telegram_endpoint)
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fake Telegram listener");
        let address = listener.local_addr().expect("listener address");
        let shutdown = CancellationToken::new();
        let server_shutdown = shutdown.clone();
        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(server_shutdown.cancelled_owned())
                .await
                .expect("fake Telegram server");
        });
        Self {
            base_url: format!("http://{address}"),
            state,
            shutdown,
        }
    }

    fn bot(&self) -> Bot {
        Bot::new(TOKEN).set_api_url(self.base_url.parse().expect("fake API URL"))
    }

    async fn push_update(&self, update: Value) {
        self.state.updates.lock().await.push_back(update);
    }

    async fn calls(&self) -> Vec<ApiCall> {
        self.state.calls.lock().await.clone()
    }
}

impl Drop for FakeTelegram {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

async fn fake_telegram_endpoint(
    State(state): State<FakeTelegramState>,
    uri: Uri,
    body: Bytes,
) -> Response {
    let path = uri.path();
    if path.contains("/file/bot") {
        return state.file.as_ref().clone().into_response();
    }

    let raw_method = path.rsplit('/').next().unwrap_or_default();
    let method = raw_method
        .chars()
        .next()
        .map(|first| {
            format!(
                "{}{}",
                first.to_ascii_lowercase(),
                &raw_method[first.len_utf8()..]
            )
        })
        .unwrap_or_default();
    let body = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
    state.calls.lock().await.push(ApiCall {
        method: method.clone(),
        body: body.clone(),
    });
    let result = match method.as_str() {
        "getMe" => bot_user(),
        "getWebhookInfo" => json!({
            "url": "",
            "has_custom_certificate": false,
            "pending_update_count": 0,
            "allowed_updates": ["message"]
        }),
        "deleteMyCommands" | "setMyCommands" | "sendChatAction" => Value::Bool(true),
        "getFile" => json!({
            "file_id": "file-1",
            "file_unique_id": "unique-1",
            "file_size": state.file.len(),
            "file_path": "logs/task.log"
        }),
        "sendMessage" => telegram_message(
            body.get("chat_id").and_then(Value::as_i64).unwrap_or(1),
            state.next_message_id.fetch_add(1, Ordering::SeqCst),
            body.get("text").and_then(Value::as_str).unwrap_or_default(),
            body.get("message_thread_id").and_then(Value::as_i64),
            true,
        ),
        "editMessageText" => telegram_message(
            body.get("chat_id").and_then(Value::as_i64).unwrap_or(1),
            body.get("message_id").and_then(Value::as_i64).unwrap_or(1) as i32,
            body.get("text").and_then(Value::as_str).unwrap_or_default(),
            None,
            true,
        ),
        "getUpdates" => {
            let update = state.updates.lock().await.pop_front();
            if let Some(update) = update {
                Value::Array(vec![update])
            } else {
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                Value::Array(Vec::new())
            }
        }
        other => panic!("unexpected fake Telegram method: {other}"),
    };
    axum::Json(json!({"ok": true, "result": result})).into_response()
}

fn bot_user() -> Value {
    json!({
        "id": 999,
        "is_bot": true,
        "first_name": "NiuPanel",
        "username": "NiuPanelBot",
        "can_join_groups": true,
        "can_read_all_group_messages": true,
        "supports_inline_queries": false,
        "has_main_web_app": false
    })
}

fn user(id: i64) -> Value {
    json!({"id": id, "is_bot": false, "first_name": format!("User {id}")})
}

fn chat(id: i64) -> Value {
    if id > 0 {
        json!({"id": id, "type": "private", "first_name": "Private"})
    } else {
        json!({"id": id, "type": "supergroup", "title": "Ops", "is_forum": true})
    }
}

fn telegram_message(
    chat_id: i64,
    message_id: i32,
    text: &str,
    thread_id: Option<i64>,
    from_bot: bool,
) -> Value {
    let mut message = json!({
        "message_id": message_id,
        "date": 1,
        "chat": chat(chat_id),
        "from": if from_bot { bot_user() } else { user(chat_id.abs()) },
        "text": text,
    });
    if let Some(thread_id) = thread_id {
        message["message_thread_id"] = json!(thread_id);
        message["is_topic_message"] = json!(true);
    }
    message
}

fn incoming_text(
    chat_id: i64,
    sender_id: i64,
    message_id: i32,
    text: &str,
    thread_id: Option<i64>,
) -> Message {
    let mut message = telegram_message(chat_id, message_id, text, thread_id, false);
    message["from"] = user(sender_id);
    serde_json::from_value(message).expect("incoming message")
}

fn incoming_reply_text(
    chat_id: i64,
    sender_id: i64,
    message_id: i32,
    text: &str,
    thread_id: Option<i64>,
    reply_message_id: i32,
) -> Message {
    let mut message = telegram_message(chat_id, message_id, text, thread_id, false);
    message["from"] = user(sender_id);
    message["reply_to_message"] =
        telegram_message(chat_id, reply_message_id, "Task failed", thread_id, true);
    serde_json::from_value(message).expect("incoming reply message")
}

fn incoming_document(
    chat_id: i64,
    sender_id: i64,
    message_id: i32,
    caption: &str,
    thread_id: i64,
) -> Message {
    let mut message = telegram_message(chat_id, message_id, "", Some(thread_id), false);
    message["from"] = user(sender_id);
    message.as_object_mut().unwrap().remove("text");
    message["caption"] = json!(caption);
    message["document"] = json!({
        "file_id": "file-1",
        "file_unique_id": "unique-1",
        "file_size": 12,
        "file_name": "task.log",
        "mime_type": "text/plain"
    });
    serde_json::from_value(message).expect("incoming document")
}

fn bot_identity() -> Me {
    serde_json::from_value(bot_user()).expect("bot identity")
}

fn binding(
    chat_id: i64,
    thread_id: Option<i32>,
    telegram_user_ids: Vec<&str>,
    interaction_mode: &str,
) -> TelegramChatBinding {
    TelegramChatBinding {
        chat_id: chat_id.to_string(),
        thread_id,
        user_id: 1,
        label: "test".to_string(),
        events: Vec::new(),
        telegram_user_ids: telegram_user_ids
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        interaction_mode: interaction_mode.to_string(),
    }
}

fn runtime(handler: TelegramAgentHandler) -> TelegramAgentRuntime {
    TelegramAgentRuntime::new(handler, 2)
}

#[tokio::test]
async fn authorized_topic_attachment_flows_through_real_bot_http_and_is_deduplicated() {
    let fake = FakeTelegram::start("exit code 1\n").await;
    let calls = Arc::new(AtomicUsize::new(0));
    let (captured_tx, mut captured_rx) = mpsc::unbounded_channel();
    let handler: TelegramAgentHandler = Arc::new({
        let calls = calls.clone();
        move |request, _| {
            let calls = calls.clone();
            let captured_tx = captured_tx.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                captured_tx.send(request).unwrap();
                Ok(TelegramAgentResponse {
                    text: "发现退出码 1".to_string(),
                })
            })
        }
    });
    let runtime = runtime(handler);
    let ledger = TelegramMessageLedger::in_memory();
    let message = incoming_document(-1001, 42, 10, "@NiuPanelBot 分析日志", 7);

    telegram::agent::handle_message(
        fake.bot(),
        message.clone(),
        vec![binding(-1001, Some(7), vec!["42"], "mention")],
        bot_identity(),
        runtime.clone(),
        ledger.clone(),
        NotificationContextStore::default(),
    )
    .await
    .expect("handle document");
    let captured = captured_rx.recv().await.expect("captured request");
    assert_eq!(captured.thread_id, Some(7));
    assert_eq!(captured.telegram_user_id, "42");
    assert_eq!(captured.attachments[0].content, "exit code 1\n");

    telegram::agent::handle_message(
        fake.bot(),
        message,
        vec![binding(-1001, Some(7), vec!["42"], "mention")],
        bot_identity(),
        runtime,
        ledger,
        NotificationContextStore::default(),
    )
    .await
    .expect("deduplicated document");
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let api_calls = fake.calls().await;
    assert_eq!(
        api_calls
            .iter()
            .filter(|call| call.method == "getFile")
            .count(),
        1
    );
    assert!(
        api_calls
            .iter()
            .any(|call| { call.method == "sendMessage" && call.body["message_thread_id"] == 7 })
    );
    assert!(api_calls.iter().any(|call| {
        call.method == "editMessageText" && call.body["text"] == "发现退出码 1"
    }));
}

#[tokio::test]
async fn unauthorized_group_sender_never_downloads_document_or_invokes_agent() {
    let fake = FakeTelegram::start("must not be downloaded").await;
    let calls = Arc::new(AtomicUsize::new(0));
    let handler: TelegramAgentHandler = Arc::new({
        let calls = calls.clone();
        move |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Ok(TelegramAgentResponse {
                    text: "unexpected".to_string(),
                })
            })
        }
    });
    telegram::agent::handle_message(
        fake.bot(),
        incoming_document(-1001, 99, 11, "@NiuPanelBot 分析日志", 7),
        vec![binding(-1001, Some(7), vec!["42"], "mention")],
        bot_identity(),
        runtime(handler),
        TelegramMessageLedger::in_memory(),
        NotificationContextStore::default(),
    )
    .await
    .expect("ignore unauthorized sender");

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(
        fake.calls()
            .await
            .iter()
            .all(|call| call.method != "getFile")
    );
}

#[tokio::test]
async fn replying_to_a_notification_injects_its_structured_panel_context() {
    let fake = FakeTelegram::start(Vec::new()).await;
    let (captured_tx, mut captured_rx) = mpsc::unbounded_channel();
    let handler: TelegramAgentHandler = Arc::new(move |request, _| {
        let captured_tx = captured_tx.clone();
        Box::pin(async move {
            captured_tx.send(request).unwrap();
            Ok(TelegramAgentResponse {
                text: "已读取告警上下文".to_string(),
            })
        })
    });
    let contexts = NotificationContextStore::default();
    contexts
        .remember(
            "-1001".to_string(),
            900,
            json!({
                "source": "telegram_notification_reply",
                "event": "task_status_changed",
                "task_id": 77,
                "status": "Failed"
            }),
        )
        .await;

    telegram::agent::handle_message(
        fake.bot(),
        incoming_reply_text(-1001, 42, 12, "诊断", Some(7), 900),
        vec![binding(-1001, Some(7), vec!["42"], "mention")],
        bot_identity(),
        runtime(handler),
        TelegramMessageLedger::in_memory(),
        contexts,
    )
    .await
    .expect("handle notification reply");

    let captured = captured_rx.recv().await.expect("captured request");
    assert_eq!(captured.text, "诊断");
    assert_eq!(captured.panel_context["channel"], "telegram");
    assert_eq!(captured.panel_context["actor"]["user_id"], 1);
    assert_eq!(
        captured.panel_context["notification_reply"],
        json!({
            "source": "telegram_notification_reply",
            "event": "task_status_changed",
            "task_id": 77,
            "status": "Failed"
        })
    );
}

#[tokio::test]
async fn cancel_message_interrupts_an_inflight_run_without_waiting_for_it() {
    let fake = FakeTelegram::start(Vec::new()).await;
    let started = Arc::new(Notify::new());
    let handler: TelegramAgentHandler = Arc::new({
        let started = started.clone();
        move |_, cancellation| {
            let started = started.clone();
            Box::pin(async move {
                started.notify_one();
                cancellation.cancelled().await;
                Err(TelegramAgentError::Cancelled)
            })
        }
    });
    let runtime = runtime(handler);
    let ledger = TelegramMessageLedger::in_memory();
    let running = tokio::spawn({
        let bot = fake.bot();
        let runtime = runtime.clone();
        let ledger = ledger.clone();
        async move {
            telegram::agent::handle_message(
                bot,
                incoming_text(42, 42, 20, "检查失败任务", None),
                vec![binding(42, None, Vec::new(), "direct")],
                bot_identity(),
                runtime,
                ledger,
                NotificationContextStore::default(),
            )
            .await
        }
    });
    started.notified().await;

    telegram::agent::handle_message(
        fake.bot(),
        incoming_text(42, 42, 21, "/cancel", None),
        vec![binding(42, None, Vec::new(), "direct")],
        bot_identity(),
        runtime,
        ledger,
        NotificationContextStore::default(),
    )
    .await
    .expect("cancel message");
    tokio::time::timeout(std::time::Duration::from_secs(2), running)
        .await
        .expect("running request stopped")
        .expect("handler task")
        .expect("handler response");

    let calls = fake.calls().await;
    assert!(calls.iter().any(|call| {
        call.method == "sendMessage"
            && call.body["text"]
                .as_str()
                .is_some_and(|text| text.contains("已停止当前会话"))
    }));
    assert!(calls.iter().any(|call| {
        call.method == "editMessageText"
            && call.body["text"]
                .as_str()
                .is_some_and(|text| text.contains("运行已停止"))
    }));
}

#[tokio::test]
async fn long_polling_dispatches_updates_through_the_transport() {
    static TEST_DATA: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let data = TEST_DATA.get_or_init(|| tempfile::tempdir().expect("test data"));
    let config = serde_json::from_value(json!({
        "system_dir": data.path().join("system"),
        "plugins_dir": data.path().join("plugins"),
        "runtimes_dir": data.path().join("runtimes")
    }))
    .expect("test config");
    let _ = niupanel_common::config::CONFIG.set(config);

    let fake = FakeTelegram::start(Vec::new()).await;
    fake.push_update(json!({
        "update_id": 100,
        "message": telegram_message(42, 30, "检查系统", None, false)
    }))
    .await;
    let called = Arc::new(Notify::new());
    let handler: TelegramAgentHandler = Arc::new({
        let called = called.clone();
        move |_, _| {
            let called = called.clone();
            Box::pin(async move {
                called.notify_one();
                Ok(TelegramAgentResponse {
                    text: "系统正常".to_string(),
                })
            })
        }
    });
    let bot = TelegramBot::new(TelegramBotConfig {
        enabled: true,
        token: TOKEN.to_string(),
        api_base_url: Some(fake.base_url.clone()),
        chat_bindings: vec![binding(42, None, Vec::new(), "direct")],
        ..TelegramBotConfig::default()
    })
    .await;
    let cancellation = CancellationToken::new();
    let run = tokio::spawn({
        let cancellation = cancellation.clone();
        async move {
            bot.run(
                EventBus::new(),
                Database::connect("sqlite::memory:")
                    .await
                    .expect("database"),
                handler,
                cancellation,
            )
            .await
        }
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), called.notified())
        .await
        .expect("long polling dispatched update");
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if fake
                .calls()
                .await
                .iter()
                .any(|call| call.method == "editMessageText" && call.body["text"] == "系统正常")
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("long polling response completed");
    cancellation.cancel();
    tokio::time::timeout(std::time::Duration::from_secs(3), run)
        .await
        .expect("bot stopped")
        .expect("bot task")
        .expect("bot run");

    let calls = fake.calls().await;
    assert!(calls.iter().any(|call| call.method == "getUpdates"));
    assert!(
        calls.iter().any(|call| {
            call.method == "editMessageText" && call.body["text"] == "系统正常"
        })
    );
}
