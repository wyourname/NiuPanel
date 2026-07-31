use crate::common::session_layer::{
    DynamicSessionLayer, default_cookie_name_from_session_key, default_session_expiry,
};
use anyhow::{Context, Result};
use sea_orm::DatabaseConnection;
use tower_cookies::{Key, cookie::SameSite};
use tower_sessions_sqlx_store::SqliteStore;
use tower_sessions_sqlx_store::sqlx::SqlitePool;

pub async fn build_session_layer(
    db: &DatabaseConnection,
    session_key: &str,
    secure_cookie: bool,
) -> Result<DynamicSessionLayer<SqliteStore>> {
    anyhow::ensure!(
        session_key.as_bytes().len() >= 64,
        "Session key must contain at least 64 bytes"
    );
    let sqlx_pool: SqlitePool = db.get_sqlite_connection_pool().clone();
    let session_store = SqliteStore::new(sqlx_pool);
    session_store
        .migrate()
        .await
        .context("Session store migration failed")?;

    let key = Key::from(session_key.as_bytes());
    let fallback_cookie_name = default_cookie_name_from_session_key(session_key);

    Ok(
        DynamicSessionLayer::new(session_store, key, fallback_cookie_name)
            .with_expiry(default_session_expiry())
            .with_http_only(true)
            .with_same_site(SameSite::Lax)
            .with_secure(secure_cookie),
    )
}
