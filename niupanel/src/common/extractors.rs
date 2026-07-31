use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::request::Parts,
};
use niupanel_common::config::Config;
use std::net::{IpAddr, SocketAddr};

pub struct RealIp(pub String);

impl<S> FromRequestParts<S> for RealIp
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let addr = ConnectInfo::<SocketAddr>::from_request_parts(parts, state)
            .await
            .ok();
        let peer_ip = addr.map(|ConnectInfo(addr)| addr.ip());

        let Some(peer_ip) = peer_ip else {
            return Ok(RealIp("0.0.0.0".to_string()));
        };
        if !is_trusted_proxy(peer_ip) {
            return Ok(RealIp(peer_ip.to_string()));
        }

        // Forwarded headers are accepted only from explicitly trusted peers.
        // Walk XFF right-to-left so values injected by the original client
        // cannot override the nearest untrusted hop.
        if let Some(xff) = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
        {
            let parsed = xff
                .split(',')
                .map(|value| value.trim().parse::<IpAddr>())
                .collect::<Result<Vec<_>, _>>();
            if let Ok(mut chain) = parsed {
                chain.push(peer_ip);
                if let Some(client_ip) = chain.into_iter().rev().find(|ip| !is_trusted_proxy(*ip)) {
                    return Ok(RealIp(client_ip.to_string()));
                }
            }
        }

        for header in ["cf-connecting-ip", "x-real-ip"] {
            if let Some(ip) = parts
                .headers
                .get(header)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().parse::<IpAddr>().ok())
            {
                return Ok(RealIp(ip.to_string()));
            }
        }

        Ok(RealIp(peer_ip.to_string()))
    }
}

fn is_trusted_proxy(ip: IpAddr) -> bool {
    Config::global()
        .trusted_proxies
        .iter()
        .filter_map(|configured| configured.trim().parse::<IpAddr>().ok())
        .any(|trusted| trusted == ip)
}
