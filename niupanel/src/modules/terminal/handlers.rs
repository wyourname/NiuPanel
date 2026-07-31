use super::service::TerminalSession;
use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::extract::ws::{self, WebSocket};
use axum::{
    extract::{Extension, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;
use utoipa::ToSchema;

static ACTIVE_SESSIONS: AtomicUsize = AtomicUsize::new(0);
const MAX_SESSIONS: usize = 3;

struct HostParts<'a> {
    hostname: &'a str,
    port: Option<u16>,
}

fn is_origin_allowed(origin: &str, host: &str) -> bool {
    if host.is_empty() {
        return true;
    }

    let Some((origin_scheme, origin_host)) = parse_origin(origin) else {
        return false;
    };

    let origin_parts = split_host_port(origin_host);
    let host_parts = split_host_port(host);

    let hostname_match = hosts_match(origin_parts.hostname, host_parts.hostname);

    if !hostname_match {
        return false;
    }

    match host_parts.port {
        Some(host_port) => {
            let origin_port = origin_parts
                .port
                .or_else(|| default_origin_port(origin_scheme));
            origin_port == Some(host_port)
        }
        None => true,
    }
}

fn parse_origin(origin: &str) -> Option<(&str, &str)> {
    let origin = origin.trim();
    let (scheme, rest) = origin.split_once("://")?;
    if scheme != "http" && scheme != "https" {
        return None;
    }

    let host = rest.split('/').next().unwrap_or(rest).trim();
    if host.is_empty() {
        None
    } else {
        Some((scheme, host))
    }
}

fn default_origin_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    }
}

fn hosts_match(left: &str, right: &str) -> bool {
    if left.eq_ignore_ascii_case(right) {
        return true;
    }

    matches!(
        (left, right),
        ("localhost", "127.0.0.1")
            | ("127.0.0.1", "localhost")
            | ("localhost", "::1")
            | ("::1", "localhost")
    )
}

fn split_host_port(host: &str) -> HostParts<'_> {
    let host = host.trim();

    if let Some(rest) = host.strip_prefix('[') {
        if let Some(end_pos) = rest.find(']') {
            let hostname = &rest[..end_pos];
            let port = rest[end_pos + 1..]
                .strip_prefix(':')
                .and_then(|port| port.parse::<u16>().ok());
            return HostParts { hostname, port };
        }
    }

    if let Some(colon_pos) = host.rfind(':') {
        let (h, p) = host.split_at(colon_pos);
        if !h.contains(':') {
            if let Ok(port) = p[1..].parse::<u16>() {
                return HostParts {
                    hostname: h,
                    port: Some(port),
                };
            }
        }
    }

    HostParts {
        hostname: host,
        port: None,
    }
}

fn forwarded_host(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-forwarded-host")
        .and_then(|h| h.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn origin_allows_matching_host_and_port() {
        assert!(is_origin_allowed("http://localhost:7787", "localhost:7787"));
    }

    #[test]
    fn origin_allows_localhost_loopback_equivalence() {
        assert!(is_origin_allowed("http://localhost:7787", "127.0.0.1:7787"));
        assert!(is_origin_allowed("http://[::1]:7787", "localhost:7787"));
    }

    #[test]
    fn origin_rejects_internal_proxy_host_without_forwarded_host() {
        assert!(!is_origin_allowed("http://localhost:7787", "api:7788"));
    }

    #[test]
    fn origin_rejects_port_mismatch() {
        assert!(!is_origin_allowed(
            "http://example.com:3000",
            "example.com:7788"
        ));
    }

    #[test]
    fn origin_allows_default_scheme_ports() {
        assert!(is_origin_allowed("http://example.com", "example.com:80"));
        assert!(is_origin_allowed("https://example.com", "example.com:443"));
    }

    #[test]
    fn forwarded_host_uses_first_non_empty_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("localhost:7787, api:7788"),
        );

        assert_eq!(forwarded_host(&headers), Some("localhost:7787"));
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum TerminalMessage {
    #[serde(rename = "input")]
    Input { data: String },
    #[serde(rename = "resize")]
    Resize { rows: u16, cols: u16 },
}

#[utoipa::path(
    get,
    path = "/api/v1/terminal/ws",
    responses(
        (status = 200, description = "WebSocket terminal connection (upgrade required)")
    ),
    tag = "Terminal",
    security(("session_cookie" = []))
)]
pub async fn terminal_ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
) -> impl IntoResponse {
    niupanel_common::logger::debug!("Terminal WS connection request received");

    // 1. CSWSH 安全校验: 验证 Origin
    let host = forwarded_host(&headers)
        .or_else(|| {
            headers
                .get(header::HOST)
                .and_then(|h| h.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("");
    let raw_host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let origin = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|o| o.to_str().ok());

    if let Some(origin_str) = origin {
        if !is_origin_allowed(origin_str, host) {
            niupanel_common::logger::warn!(
                "Terminal WS rejected: Origin mismatch. Origin: {}, Host: {}, Effective-Host: {}",
                origin_str,
                raw_host,
                host
            );
            return (
                StatusCode::FORBIDDEN,
                "Cross-Site WebSocket Hijacking detected",
            )
                .into_response();
        }
    }

    // 2. 并发限制校验
    if ACTIVE_SESSIONS.load(Ordering::SeqCst) >= MAX_SESSIONS {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Too many active terminal sessions. Please try again later.",
        )
            .into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state, user, ip))
}

async fn handle_socket(socket: WebSocket, state: AppState, user: AuthenticatedUser, ip: String) {
    ACTIVE_SESSIONS.fetch_add(1, Ordering::SeqCst);

    struct SessionGuard;
    impl Drop for SessionGuard {
        fn drop(&mut self) {
            ACTIVE_SESSIONS.fetch_sub(1, Ordering::SeqCst);
        }
    }
    let _guard = SessionGuard;

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    // Initial session with default size
    let session = match TerminalSession::new(24, 80, tx, state.db.clone(), user.id, ip).await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            let _ = sender
                .send(ws::Message::Text(format!("Error: {}", e).into()))
                .await;
            return;
        }
    };

    // Task to send PTY output back to WS
    let mut send_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if sender.send(ws::Message::Binary(data.into())).await.is_err() {
                break;
            }
        }
    });

    // Task to receive WS messages and send to PTY
    let session_recv = session.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                ws::Message::Text(t) => {
                    if let Ok(terminal_msg) = serde_json::from_str::<TerminalMessage>(&t) {
                        match terminal_msg {
                            TerminalMessage::Input { data } => {
                                let _ = session_recv.write(data.as_bytes()).await;
                            }
                            TerminalMessage::Resize { rows, cols } => {
                                let _ = session_recv.resize(rows, cols).await;
                            }
                        }
                    }
                }
                ws::Message::Binary(b) => {
                    let _ = session_recv.write(&b).await;
                }
                ws::Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
