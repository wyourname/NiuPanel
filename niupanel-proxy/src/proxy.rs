use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use niupanel_common::logger::{debug, error};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

use crate::error::Errors;

const BUFFER_SIZE: usize = 32 * 1024;

// SOCKS5 -> CF
pub async fn proxy_tcp_to_ws<R>(
    mut reader: R,
    mut ws_write: SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>,
) -> Result<(), Errors>
where
    R: AsyncRead + Unpin,
{
    debug!("Starting SOCKS5 to WebSocket proxy");
    let mut buffer = [0u8; BUFFER_SIZE];
    loop {
        match reader.read(&mut buffer).await {
            Ok(n) if n > 0 => {
                ws_write
                    .send(Message::Binary(buffer[..n].to_vec().into()))
                    .await?;
                debug!("SOCKS5 TO WS: sent {} bytes", n);
            }
            Ok(_) => {
                debug!("SOCKS5 closed by client");
                break;
            }
            Err(e) => {
                error!("SOCKS5 error: {}", e);
                break;
            }
        }
    }
    ws_write.send(Message::Close(None)).await?;
    Ok(())
}

// CF -> SOCKS5
pub async fn proxy_ws_to_tcp<W>(
    mut ws_read: SplitStream<WebSocketStream<TlsStream<TcpStream>>>,
    mut writer: W,
) -> Result<(), Errors>
where
    W: AsyncWrite + Unpin,
{
    debug!("Starting WebSocket to SOCKS5 proxy");
    while let Some(msg) = ws_read.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                writer.write_all(&data).await?;
                debug!("WS TO SOCKS5: sent {} bytes", data.len());
            }
            Ok(Message::Close(_)) => {
                debug!("WebSocket closed by server");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                return Err(Errors::WsError(e));
            }
            _ => continue,
        }
    }
    writer.shutdown().await?;
    Ok(())
}
