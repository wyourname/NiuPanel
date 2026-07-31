use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

const TOKEN_PREFIX: &str = "niu-internal-v1";
const USER_TOKEN_PREFIX: &str = "niu-internal-user-v1";
const HMAC_BLOCK_SIZE: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalSdkClaims {
    pub task_id: i32,
    pub run_id: Option<i32>,
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalUserClaims {
    pub user_id: i32,
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InternalSdkTokenError {
    InvalidFormat,
    InvalidSignature,
    Expired,
}

pub fn is_internal_sdk_token(token: &str) -> bool {
    token.starts_with(TOKEN_PREFIX)
}

pub fn is_internal_user_token(token: &str) -> bool {
    token.starts_with(USER_TOKEN_PREFIX)
}

pub fn issue_internal_sdk_token(
    secret: &str,
    task_id: i32,
    run_id: Option<i32>,
    ttl_secs: i64,
) -> String {
    let ttl_secs = ttl_secs.max(1);
    issue_internal_sdk_token_with_now(secret, task_id, run_id, ttl_secs, unix_timestamp())
}

pub fn verify_internal_sdk_token(
    secret: &str,
    token: &str,
) -> Result<InternalSdkClaims, InternalSdkTokenError> {
    verify_internal_sdk_token_with_now(secret, token, unix_timestamp())
}

pub fn issue_internal_user_token(secret: &str, user_id: i32, ttl_secs: i64) -> String {
    let ttl_secs = ttl_secs.max(1);
    issue_internal_user_token_with_now(secret, user_id, ttl_secs, unix_timestamp())
}

pub fn verify_internal_user_token(
    secret: &str,
    token: &str,
) -> Result<InternalUserClaims, InternalSdkTokenError> {
    verify_internal_user_token_with_now(secret, token, unix_timestamp())
}

fn issue_internal_sdk_token_with_now(
    secret: &str,
    task_id: i32,
    run_id: Option<i32>,
    ttl_secs: i64,
    now: i64,
) -> String {
    let expires_at = now.saturating_add(ttl_secs);
    let run_part = run_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "none".to_string());
    let payload = format!("{TOKEN_PREFIX}.{task_id}.{run_part}.{expires_at}");
    let signature = hmac_sha256_hex(secret.as_bytes(), payload.as_bytes());
    format!("{payload}.{signature}")
}

fn issue_internal_user_token_with_now(
    secret: &str,
    user_id: i32,
    ttl_secs: i64,
    now: i64,
) -> String {
    let expires_at = now.saturating_add(ttl_secs);
    let payload = format!("{USER_TOKEN_PREFIX}.{user_id}.{expires_at}");
    let signature = hmac_sha256_hex(secret.as_bytes(), payload.as_bytes());
    format!("{payload}.{signature}")
}

fn verify_internal_sdk_token_with_now(
    secret: &str,
    token: &str,
    now: i64,
) -> Result<InternalSdkClaims, InternalSdkTokenError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 5 || parts[0] != TOKEN_PREFIX {
        return Err(InternalSdkTokenError::InvalidFormat);
    }

    let payload = parts[..4].join(".");
    let expected_signature = hmac_sha256_hex(secret.as_bytes(), payload.as_bytes());
    if !constant_time_eq(expected_signature.as_bytes(), parts[4].as_bytes()) {
        return Err(InternalSdkTokenError::InvalidSignature);
    }

    let task_id = parts[1]
        .parse::<i32>()
        .map_err(|_| InternalSdkTokenError::InvalidFormat)?;
    let run_id = if parts[2] == "none" {
        None
    } else {
        Some(
            parts[2]
                .parse::<i32>()
                .map_err(|_| InternalSdkTokenError::InvalidFormat)?,
        )
    };
    let expires_at = parts[3]
        .parse::<i64>()
        .map_err(|_| InternalSdkTokenError::InvalidFormat)?;

    if now > expires_at {
        return Err(InternalSdkTokenError::Expired);
    }

    Ok(InternalSdkClaims {
        task_id,
        run_id,
        expires_at,
    })
}

fn verify_internal_user_token_with_now(
    secret: &str,
    token: &str,
    now: i64,
) -> Result<InternalUserClaims, InternalSdkTokenError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 4 || parts[0] != USER_TOKEN_PREFIX {
        return Err(InternalSdkTokenError::InvalidFormat);
    }

    let payload = parts[..3].join(".");
    let expected_signature = hmac_sha256_hex(secret.as_bytes(), payload.as_bytes());
    if !constant_time_eq(expected_signature.as_bytes(), parts[3].as_bytes()) {
        return Err(InternalSdkTokenError::InvalidSignature);
    }

    let user_id = parts[1]
        .parse::<i32>()
        .map_err(|_| InternalSdkTokenError::InvalidFormat)?;
    let expires_at = parts[2]
        .parse::<i64>()
        .map_err(|_| InternalSdkTokenError::InvalidFormat)?;

    if now > expires_at {
        return Err(InternalSdkTokenError::Expired);
    }

    Ok(InternalUserClaims {
        user_id,
        expires_at,
    })
}

fn hmac_sha256_hex(secret: &[u8], message: &[u8]) -> String {
    let mut key = [0u8; HMAC_BLOCK_SIZE];
    if secret.len() > HMAC_BLOCK_SIZE {
        let digest = Sha256::digest(secret);
        key[..digest.len()].copy_from_slice(&digest);
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }

    let mut ipad = [0x36u8; HMAC_BLOCK_SIZE];
    let mut opad = [0x5cu8; HMAC_BLOCK_SIZE];
    for i in 0..HMAC_BLOCK_SIZE {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    hex::encode(outer.finalize())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_token_verifies() {
        let token = issue_internal_sdk_token_with_now("secret", 12, Some(34), 60, 1000);
        let claims = verify_internal_sdk_token_with_now("secret", &token, 1001).unwrap();

        assert_eq!(claims.task_id, 12);
        assert_eq!(claims.run_id, Some(34));
        assert_eq!(claims.expires_at, 1060);
    }

    #[test]
    fn expired_token_is_rejected() {
        let token = issue_internal_sdk_token_with_now("secret", 12, None, 60, 1000);
        let err = verify_internal_sdk_token_with_now("secret", &token, 1061).unwrap_err();

        assert_eq!(err, InternalSdkTokenError::Expired);
    }

    #[test]
    fn tampered_token_is_rejected() {
        let token = issue_internal_sdk_token_with_now("secret", 12, Some(34), 60, 1000);
        let tampered = token.replacen(".12.", ".99.", 1);
        let err = verify_internal_sdk_token_with_now("secret", &tampered, 1001).unwrap_err();

        assert_eq!(err, InternalSdkTokenError::InvalidSignature);
    }

    #[test]
    fn issued_user_token_verifies() {
        let token = issue_internal_user_token_with_now("secret", 42, 60, 1000);
        let claims = verify_internal_user_token_with_now("secret", &token, 1001).unwrap();

        assert_eq!(claims.user_id, 42);
        assert_eq!(claims.expires_at, 1060);
    }

    #[test]
    fn expired_user_token_is_rejected() {
        let token = issue_internal_user_token_with_now("secret", 42, 60, 1000);
        let err = verify_internal_user_token_with_now("secret", &token, 1061).unwrap_err();

        assert_eq!(err, InternalSdkTokenError::Expired);
    }

    #[test]
    fn tampered_user_token_is_rejected() {
        let token = issue_internal_user_token_with_now("secret", 42, 60, 1000);
        let tampered = token.replacen(".42.", ".99.", 1);
        let err = verify_internal_user_token_with_now("secret", &tampered, 1001).unwrap_err();

        assert_eq!(err, InternalSdkTokenError::InvalidSignature);
    }
}
