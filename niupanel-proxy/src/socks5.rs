use futures::StreamExt;
use niupanel_common::logger::{debug, error, info, warn};
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::{ClientConfig, RootCertStore, pki_types::ServerName};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::configutil::Config;
use crate::error::Errors;
use crate::proxy::{proxy_tcp_to_ws, proxy_ws_to_tcp};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Start the SOCKS5 server
pub async fn start_server(
    config: Arc<Config>,
    shutdown: tokio::sync::oneshot::Receiver<()>,
    ready_tx: tokio::sync::oneshot::Sender<std::net::SocketAddr>,
    label: &'static str,
) -> Result<(), Errors> {
    let addr = format!("{}:{}", config.host, config.port);
    let res = TcpListener::bind(&addr).await;

    let listener = match res {
        Ok(l) => l,
        Err(e) => {
            error!(
                "[{}] Failed to start SOCKS5 server on {}: {}",
                label, addr, e
            );
            return Err(e.into());
        }
    };

    let local_addr = listener.local_addr()?;
    info!("[{}] SOCKS5 server started on: {}", label, local_addr);
    let _ = ready_tx.send(local_addr);

    // Initialize TLS connector once
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());

    let client_config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let tls_connector = TlsConnector::from(Arc::new(client_config));
    let mut shutdown_fut = shutdown;

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (stream, peer_addr) = match res {
                    Ok(v) => v,
                    Err(e) => {
                        error!("[{}] Accept error: {}", label, e);
                        continue;
                    }
                };

                let config = Arc::clone(&config);
                let tls_connector = tls_connector.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, config, tls_connector, label).await {
                        debug!("[{}] Client {} error: {:?}", label, peer_addr, e);
                    }
                });
            }
            _ = &mut shutdown_fut => {
                info!("[{}] SOCKS5 server shutting down...", label);
                break;
            }
        }
    }

    Ok(())
}

/// Handle a single SOCKS5 client connection
pub async fn handle_client<S>(
    stream: S,
    config: Arc<Config>,
    tls_connector: TlsConnector,
    label: &'static str,
) -> Result<(), Errors>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    debug!("[{}] Handling internal tunnel connection", label);

    // 1. SOCKS5 Handshake with timeout
    let (target_host, target_port) = match timeout(HANDSHAKE_TIMEOUT, async {
        handshake(&mut reader, &mut writer, &config).await?;
        parse_socks5(&mut reader).await
    })
    .await
    {
        Ok(res) => res?,
        Err(_) => return Err(Errors::HandshakeError("Handshake timed out".into())),
    };

    // 2. Establish TCP connection to Cloudflare
    let tcp = connect_cloudflare(&config, label).await?;

    // 3. Establish TLS connection
    let tls_stream = establish_tls(tcp, tls_connector, &config.cfhost, label).await?;

    // 4. Establish WebSocket tunnel
    let (ws_write, ws_read) =
        establish_ws(tls_stream, &config, &target_host, target_port, label).await?;

    // 5. Send success response to SOCKS5 client
    writer
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await?;

    info!(
        "[{}] Tunnel Open: {}:{} (via {})",
        label, target_host, target_port, config.cfhost
    );

    // 6. Proxy data bi-directionally
    tokio::select! {
        res = proxy_tcp_to_ws(reader, ws_write) => res?,
        res = proxy_ws_to_tcp(ws_read, writer) => res?,
    };

    info!("[{}] Tunnel Closed: {}:{}", label, target_host, target_port);
    Ok(())
}

/// Connect to Cloudflare via IP or Host
async fn connect_cloudflare(config: &Config, label: &'static str) -> Result<TcpStream, Errors> {
    let ws_port = 443;

    // Try IP connection if configured and not default
    if !config.cfip.is_empty() && config.cfip != "104.16.0.0" {
        let addr = format!("{}:{}", config.cfip, ws_port);
        debug!("[{}] Connecting to Cloudflare via IP: {}", label, addr);
        if let Ok(s) = TcpStream::connect(&addr).await {
            return Ok(s);
        }
        warn!(
            "[{}] Failed to connect to CF via IP {}, falling back to Host",
            label, addr
        );
    }

    // Fallback to Host connection
    let addr = format!("{}:{}", config.cfhost, ws_port);
    debug!("[{}] Connecting to Cloudflare via Host: {}", label, addr);
    TcpStream::connect(&addr).await.map_err(Into::into)
}

/// Establish TLS connection over TCP stream
async fn establish_tls(
    tcp: TcpStream,
    tls_connector: TlsConnector,
    cf_host: &str,
    label: &'static str,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>, Errors> {
    let server_name = ServerName::try_from(cf_host)
        .map_err(|_| Errors::TlsError(format!("Invalid DNS name: {}", cf_host)))?;

    debug!("[{}] Performing TLS handshake with {}", label, cf_host);
    tls_connector
        .connect(server_name.to_owned(), tcp)
        .await
        .map_err(|e| {
            error!("[{}] TLS handshake failed with {}: {}", label, cf_host, e);
            Errors::TlsError(e.to_string())
        })
}

/// Establish WebSocket tunnel over TLS stream
async fn establish_ws(
    tls_stream: tokio_rustls::client::TlsStream<TcpStream>,
    config: &Config,
    target_host: &str,
    target_port: u16,
    label: &'static str,
) -> Result<
    (
        futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio_rustls::client::TlsStream<TcpStream>>,
            tokio_tungstenite::tungstenite::Message,
        >,
        futures::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<tokio_rustls::client::TlsStream<TcpStream>>,
        >,
    ),
    Errors,
> {
    let ws_url = format!("wss://{}/", config.cfhost);
    let mut req = ws_url
        .into_client_request()
        .map_err(|e| Errors::WsError(e))?;

    let headers = req.headers_mut();

    let host_header = HeaderValue::from_str(&config.cfhost)
        .map_err(|_| Errors::InvalidHeader(format!("Invalid cfhost: {}", config.cfhost)))?;
    let token_header = HeaderValue::from_str(&config.token)
        .map_err(|_| Errors::InvalidHeader("Invalid token".into()))?;
    let hostname_header = HeaderValue::from_str(target_host)
        .map_err(|_| Errors::InvalidHeader(format!("Invalid target hostname: {}", target_host)))?;
    let port_header = HeaderValue::from_str(&target_port.to_string())
        .map_err(|_| Errors::InvalidHeader(format!("Invalid target port: {}", target_port)))?;

    headers.insert("Host", host_header);
    headers.insert("Token", token_header);
    headers.insert("Hostname", hostname_header);
    headers.insert("Port", port_header);

    debug!(
        "[{}] Establishing WebSocket tunnel to {} for {}:{}",
        label, config.cfhost, target_host, target_port
    );

    let (ws_stream, _) = timeout(
        HANDSHAKE_TIMEOUT,
        tokio_tungstenite::client_async(req, tls_stream),
    )
    .await
    .map_err(|_| {
        Errors::WsError(tokio_tungstenite::tungstenite::Error::Io(Error::new(
            ErrorKind::TimedOut,
            "WebSocket handshake timed out",
        )))
    })?
    .map_err(|e| {
        error!(
            "[{}] WebSocket handshake failed with {}: {}",
            label, config.cfhost, e
        );
        Errors::WsError(e)
    })?;

    debug!(
        "[{}] WebSocket connection established for {}:{}",
        label, target_host, target_port
    );
    Ok(ws_stream.split())
}

async fn handshake<R, W>(reader: &mut R, writer: &mut W, config: &Config) -> Result<(), Errors>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut init_buf = [0u8; 2];
    reader.read_exact(&mut init_buf).await?;

    if init_buf[0] != 0x05 {
        debug!(
            "Received invalid SOCKS version byte: 0x{:02x} (expected 0x05)",
            init_buf[0]
        );
        writer.write_all(&[0x05, 0xff]).await?;
        return Err(Errors::IoError(Error::new(
            ErrorKind::InvalidData,
            format!("Unsupported SOCKS version: 0x{:02x}", init_buf[0]),
        )));
    }

    let mut methods = vec![0u8; init_buf[1] as usize];
    reader.read_exact(&mut methods).await?;

    if methods.contains(&0x02) {
        writer.write_all(&[0x05, 0x02]).await?;
        auth_user(reader, writer, config).await?;
    } else if methods.contains(&0x00) {
        writer.write_all(&[0x05, 0x00]).await?;
    } else {
        writer.write_all(&[0x05, 0xff]).await?;
        return Err(Errors::IoError(Error::new(
            ErrorKind::PermissionDenied,
            "No acceptable authentication methods",
        )));
    }
    Ok(())
}

async fn auth_user<R, W>(reader: &mut R, writer: &mut W, config: &Config) -> Result<(), Errors>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut version = [0u8; 1];
    reader.read_exact(&mut version).await?;
    if version[0] != 0x01 {
        return Err(Errors::IoError(Error::new(
            ErrorKind::InvalidData,
            "Unsupported authentication version",
        )));
    }

    let mut username_len = [0u8; 1];
    reader.read_exact(&mut username_len).await?;
    let mut username = vec![0u8; username_len[0] as usize];
    reader.read_exact(&mut username).await?;
    let username = String::from_utf8_lossy(&username);

    let mut password_len = [0u8; 1];
    reader.read_exact(&mut password_len).await?;
    let mut password = vec![0u8; password_len[0] as usize];
    reader.read_exact(&mut password).await?;
    let password = String::from_utf8_lossy(&password);

    if username == config.user && password == config.passwd {
        writer.write_all(&[0x01, 0x00]).await?;
        Ok(())
    } else {
        writer.write_all(&[0x01, 0x01]).await?;
        Err(Errors::IoError(Error::new(
            ErrorKind::PermissionDenied,
            "Invalid username or password",
        )))
    }
}

async fn parse_socks5<R>(reader: &mut R) -> Result<(String, u16), Errors>
where
    R: AsyncRead + Unpin,
{
    let mut request = [0u8; 4];
    reader.read_exact(&mut request).await?;

    if request[0] != 0x05 {
        return Err(Errors::IoError(Error::new(
            ErrorKind::InvalidData,
            "Invalid SOCKS5 request",
        )));
    }

    if request[1] != 0x01 {
        return Err(Errors::IoError(Error::new(
            ErrorKind::InvalidData,
            "No support BIND/UDP",
        )));
    }

    let host = match request[3] {
        0x01 => {
            let mut addr = [0u8; 4];
            reader.read_exact(&mut addr).await?;
            addr.iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(".")
        }
        0x03 => {
            let mut len = [0u8; 1];
            reader.read_exact(&mut len).await?;
            let mut domain = vec![0u8; len[0] as usize];
            reader.read_exact(&mut domain).await?;
            String::from_utf8_lossy(&domain).to_string()
        }
        0x04 => {
            let mut addr = [0u8; 16];
            reader.read_exact(&mut addr).await?;
            let ipv6 = std::net::Ipv6Addr::from(addr);
            ipv6.to_string()
        }
        _ => {
            return Err(Errors::IoError(Error::new(
                ErrorKind::InvalidData,
                "Unsupported address type",
            )));
        }
    };

    let mut port_bytes = [0u8; 2];
    reader.read_exact(&mut port_bytes).await?;
    let port = u16::from_be_bytes(port_bytes);

    Ok((host, port))
}
