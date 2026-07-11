use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::request::Parts,
};
use std::net::SocketAddr;

pub struct RealIp(pub String);

impl<S> FromRequestParts<S> for RealIp
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // 1. Try CF-Connecting-IP (Cloudflare)
        if let Some(ip) = parts
            .headers
            .get("cf-connecting-ip")
            .and_then(|v| v.to_str().ok())
        {
            return Ok(RealIp(ip.to_string()));
        }

        // 2. Try X-Forwarded-For
        // Format: client, proxy1, proxy2
        if let Some(ip) = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
        {
            return Ok(RealIp(ip));
        }

        // 3. Try X-Real-IP
        if let Some(ip) = parts.headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
            return Ok(RealIp(ip.to_string()));
        }

        // 4. Fallback to ConnectInfo
        let addr = ConnectInfo::<SocketAddr>::from_request_parts(parts, state)
            .await
            .ok();

        let ip = addr
            .map(|ConnectInfo(addr)| addr.ip().to_string())
            .unwrap_or_else(|| "0.0.0.0".to_string());

        Ok(RealIp(ip))
    }
}
