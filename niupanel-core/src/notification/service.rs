use lettre::message::SinglePart;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{Message, SmtpTransport, Transport};
use niupanel_common::error::{AppError, Result};
use niupanel_common::{debug, error, info};
use serde::Serialize;

#[derive(Serialize)]
struct WebhookPayload {
    msgtype: String,
    text: TextContent,
}

#[derive(Serialize)]
struct TextContent {
    content: String,
}

pub struct NotificationService;

impl NotificationService {
    pub async fn send_webhook(
        client: &reqwest::Client,
        url: &str,
        title: &str,
        content: &str,
    ) -> Result<()> {
        if url.trim().is_empty() {
            return Ok(());
        }

        // DingTalk / WeCom format
        let payload = WebhookPayload {
            msgtype: "text".to_string(),
            text: TextContent {
                content: format!("[NiuPanel] {}\n{}", title, content),
            },
        };

        let res = client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Generic(format!("Failed to send webhook request: {}", e)))?;

        if res.status().is_success() {
            info!("Notification sent to webhook");
            Ok(())
        } else {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            error!("Failed to send notification: HTTP {} - {}", status, text);
            Err(AppError::Generic(format!(
                "Webhook failed: HTTP {} - {}",
                status, text
            )))
        }
    }

    pub async fn send_email(
        host_str: &str,
        username: &str,
        password: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<()> {
        if host_str.trim().is_empty() || to.trim().is_empty() {
            return Err(AppError::Generic(
                "SMTP host or recipient is empty".to_string(),
            ));
        }

        // Parse host and port
        let mut parts = host_str.split(':');
        let host = parts.next().unwrap_or(host_str);
        let port = parts
            .next()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(465);

        let email = Message::builder()
            .from(
                format!("NiuPanel <{}>", username)
                    .parse()
                    .map_err(|e| AppError::Generic(format!("Invalid sender address: {}", e)))?,
            )
            .to(to
                .parse()
                .map_err(|e| AppError::Generic(format!("Invalid recipient address: {}", e)))?)
            .subject(format!("[NiuPanel] {}", subject))
            .singlepart(SinglePart::plain(body.to_string()))
            .map_err(|e| AppError::Generic(format!("Failed to build email: {}", e)))?;

        let creds = Credentials::new(username.to_string(), password.to_string());

        // Open relay or auth
        let mut mailer_builder = SmtpTransport::relay(host)
            .map_err(|e| AppError::Generic(format!("Failed to build SMTP transport: {}", e)))?
            .port(port);

        if port == 465 {
            let tls_parameters = TlsParameters::new(host.to_string())
                .map_err(|e| AppError::Generic(format!("Failed to build TLS parameters: {}", e)))?;
            mailer_builder = mailer_builder.tls(Tls::Wrapper(tls_parameters));
        }

        let mailer = if !username.is_empty() && !password.is_empty() {
            mailer_builder.credentials(creds).build()
        } else {
            mailer_builder.build()
        };

        // Execute in blocking thread because lettre is sync by default
        let result = tokio::task::spawn_blocking(move || mailer.send(&email))
            .await
            .map_err(|e| AppError::Generic(format!("Email task join error: {}", e)))?;

        match result {
            Ok(_) => {
                info!("Email sent successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to send email: {}", e);
                Err(AppError::Generic(format!("Failed to send email: {}", e)))
            }
        }
    }

    pub async fn send_telegram(
        client: &reqwest::Client,
        token: &str,
        admin_chat_id: &str,
        text: &str,
        api_base_url: Option<&str>,
    ) -> Result<()> {
        if token.is_empty() || admin_chat_id.is_empty() {
            return Err(AppError::ValidationError(
                "Telegram Token or Chat ID is missing".to_string(),
            ));
        }

        let base_url = if let Some(url) = api_base_url {
            if url.is_empty() {
                "https://api.telegram.org"
            } else {
                url
            }
        } else {
            "https://api.telegram.org"
        };
        let url = format!("{}/bot{}/sendMessage", base_url, token);

        debug!("Sending Telegram request to: {}", url);

        let payload = serde_json::json!({
            "chat_id": admin_chat_id,
            "text": text,
            "parse_mode": "MarkdownV2"
        });

        let res = client.post(&url).json(&payload).send().await.map_err(|e| {
            error!("Telegram request failed: {}", e);
            AppError::Generic(format!("Failed to send Telegram request: {}", e))
        })?;

        let status = res.status();
        if status.is_success() {
            info!(
                "Telegram notification sent successfully to {}",
                admin_chat_id
            );
            Ok(())
        } else {
            let text = res.text().await.unwrap_or_default();
            error!("Telegram API returned error: HTTP {} - {}", status, text);
            Err(AppError::Generic(format!(
                "Telegram API failed (HTTP {}): {}",
                status, text
            )))
        }
    }
}
