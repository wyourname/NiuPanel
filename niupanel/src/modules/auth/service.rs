use crate::common::state::AppState;
use crate::modules::auth::models::{
    ForgotPasswordRequest, IdentifyRequest, RegisterRequest, ResetInfoResponse,
    ResetPasswordRequest, SetupStatus, VerifyCodeRequest,
};
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Utc;
pub use niupanel_common::auth::permissions::AuthenticatedUser;
use niupanel_common::auth::validate_password_strength;
use niupanel_common::error::{AppError, Result};
use niupanel_core::audit::service::AuditService;
use niupanel_core::event_bus::SystemEvent as CoreSystemEvent;
use niupanel_core::notification::service::NotificationService;
use niupanel_core::settings::{
    MAIL_HOST, MAIL_PASSWORD, MAIL_TO, MAIL_USERNAME, NOTIFY_EVENTS, SettingsManager,
};
use niupanel_entity::{login_attempts, password_resets, users};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use sea_orm::{DatabaseConnection, DbErr};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::OnceLock;
use tower_sessions_sqlx_store::sqlx::{self, Row, SqlitePool};

pub const USER_ID_KEY: &str = "user_id";
pub const USER_ROLE_KEY: &str = "user_role";
const MAX_FAILURES: i32 = 5;
const COOLDOWN_MINUTES: i64 = 5;
static ADMIN_REGISTRATION_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

pub struct SessionService;
pub struct AuthService;

impl SessionService {
    fn get_pool(db: &DatabaseConnection) -> &SqlitePool {
        db.get_sqlite_connection_pool()
    }

    pub async fn get_active_sessions_for_user(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<Vec<(String, i64)>> {
        let pool = Self::get_pool(db);
        let now = chrono::Utc::now().timestamp();

        let rows =
            sqlx::query("SELECT id, data, expiry_date FROM tower_sessions WHERE expiry_date > ?")
                .bind(now)
                .fetch_all(pool)
                .await
                .map_err(|e| AppError::Database(DbErr::Custom(e.to_string())))?;

        let mut user_sessions = Vec::new();

        for row in rows {
            let data: Vec<u8> = row.get("data");
            if let Ok(map) = serde_json::from_slice::<HashMap<String, serde_json::Value>>(&data) {
                if let Some(uid) = map.get("user_id").and_then(|v| v.as_i64()) {
                    if uid as i32 == user_id {
                        let id: String = row.get("id");
                        let expiry: i64 = match row.try_get::<i64, _>("expiry_date") {
                            Ok(val) => val,
                            Err(_) => 0, // Handle non-int expiry if needed
                        };
                        user_sessions.push((id, expiry));
                    }
                }
            }
        }
        Ok(user_sessions)
    }

    pub async fn cleanup_user_sessions(
        db: &DatabaseConnection,
        user_id: i32,
        max_sessions: usize,
    ) -> Result<()> {
        let mut sessions = Self::get_active_sessions_for_user(db, user_id).await?;

        sessions.sort_by_key(|k| k.1);

        let current_count = sessions.len();
        // 如果 max_sessions 为 0，则删除所有。
        // 如果 current_count >= max_sessions，则删除多余的。
        if max_sessions == 0 || current_count >= max_sessions {
            let to_remove = if max_sessions == 0 {
                current_count
            } else {
                current_count - max_sessions + 1
            };

            for i in 0..to_remove {
                if let Some((sid, _)) = sessions.get(i) {
                    tracing::info!("Kicking out session: {}", sid);
                    Self::delete_session(db, sid).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn delete_session(db: &DatabaseConnection, session_id: &str) -> Result<()> {
        let pool = Self::get_pool(db);
        sqlx::query("DELETE FROM tower_sessions WHERE id = ?")
            .bind(session_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(DbErr::Custom(e.to_string())))?;
        Ok(())
    }

    // For Settings (List all active)
    pub async fn get_all_active_sessions(db: &DatabaseConnection) -> Result<Vec<(String, i64)>> {
        let pool = Self::get_pool(db);
        let now = chrono::Utc::now().timestamp();

        let rows = sqlx::query("SELECT id, expiry_date FROM tower_sessions")
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(DbErr::Custom(e.to_string())))?;

        let mut sessions = Vec::new();
        for row in rows {
            let id: String = row.try_get("id").unwrap_or_default();
            let expiry: i64 = match row.try_get::<i64, _>("expiry_date") {
                Ok(val) => val,
                Err(_) => {
                    let s: String = row.try_get("expiry_date").unwrap_or_default();
                    if let Ok(val) = s.parse::<i64>() {
                        val
                    } else if let Ok(dt) = chrono::DateTime::<chrono::Utc>::from_str(&s) {
                        dt.timestamp()
                    } else if let Ok(dt) =
                        chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    {
                        dt.and_utc().timestamp()
                    } else {
                        0
                    }
                }
            };
            if expiry > now {
                sessions.push((id, expiry));
            }
        }
        Ok(sessions)
    }
}

impl AuthService {
    pub async fn identify_reset(
        _db: &DatabaseConnection,
        _payload: IdentifyRequest,
    ) -> Result<ResetInfoResponse> {
        // Preserve the API shape without disclosing account existence,
        // email binding state, or the user's email provider.
        Ok(ResetInfoResponse {
            suffix: String::new(),
        })
    }

    pub async fn forgot_password(
        db: &DatabaseConnection,
        payload: ForgotPasswordRequest,
    ) -> Result<()> {
        const MIN_RESPONSE_TIME: std::time::Duration = std::time::Duration::from_millis(250);
        let started_at = tokio::time::Instant::now();
        let email = payload.email.trim().to_string();
        let username = payload.username.trim().to_string();

        let user = users::Entity::find()
            .filter(users::Column::Username.eq(&username))
            .one(db)
            .await?;

        let Some(user) = user.filter(|user| user.email.as_deref() == Some(email.as_str())) else {
            // Keep the public response indistinguishable for unknown users and
            // mismatched email addresses.
            tokio::time::sleep_until(started_at + MIN_RESPONSE_TIME).await;
            return Ok(());
        };

        let last_reset = password_resets::Entity::find()
            .filter(password_resets::Column::UserId.eq(user.id))
            .order_by_desc(password_resets::Column::CreatedAt)
            .one(db)
            .await?;

        if let Some(record) = last_reset {
            let elapsed = Utc::now().timestamp() - record.created_at.timestamp();
            if elapsed < 60 {
                tokio::time::sleep_until(started_at + MIN_RESPONSE_TIME).await;
                return Ok(());
            }
        }

        let alphabet: [char; 62] = [
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g',
            'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x',
            'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
            'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
        ];
        let code = nanoid::nanoid!(6, &alphabet);

        let mut hasher = Sha256::new();
        hasher.update(code.as_bytes());
        let code_hash = hex::encode(hasher.finalize());

        let now = Utc::now();
        let expires_at = now + chrono::Duration::minutes(10);

        password_resets::Entity::delete_many()
            .filter(password_resets::Column::UserId.eq(user.id))
            .exec(db)
            .await?;

        let reset_model = password_resets::ActiveModel {
            user_id: Set(user.id),
            token_hash: Set(code_hash),
            expires_at: Set(expires_at.into()),
            created_at: Set(now.into()),
            ..Default::default()
        };
        reset_model.insert(db).await?;

        let mail_host = SettingsManager::get(db, MAIL_HOST).await?;
        let mail_user = SettingsManager::get(db, MAIL_USERNAME).await?;
        let mail_pass = SettingsManager::get(db, MAIL_PASSWORD).await?;

        if mail_host.is_empty() || mail_user.is_empty() {
            niupanel_common::warn!("密码重置邮件未发送：邮件服务未配置");
            tokio::time::sleep_until(started_at + MIN_RESPONSE_TIME).await;
            return Ok(());
        }

        let recipient_name = user.username;
        let body = format!(
            "您好 {}，\n\n您的验证码是：\n\n【 {} 】\n\n该验证码用于重置密码，将在 10 分钟后过期。请勿将验证码泄露给他人。\n\n来自你的NiuPanel 面板",
            recipient_name, code
        );
        tokio::spawn(async move {
            if let Err(error) = NotificationService::send_email(
                &mail_host,
                &mail_user,
                &mail_pass,
                &email,
                "密码重置验证码",
                &body,
            )
            .await
            {
                niupanel_common::warn!("密码重置邮件发送失败: {}", error);
            }
        });

        tokio::time::sleep_until(started_at + MIN_RESPONSE_TIME).await;
        Ok(())
    }

    pub async fn verify_reset_code(
        state: &AppState,
        ip: &str,
        payload: VerifyCodeRequest,
    ) -> Result<String> {
        const MIN_FAILURE_TIME: std::time::Duration = std::time::Duration::from_millis(150);
        let started_at = tokio::time::Instant::now();
        let email = payload.email.trim();
        Self::check_rate_limit(state, ip, email).await?;

        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .one(&state.db)
            .await?;
        let Some(user) = user else {
            Self::record_login_attempt(state, ip, email, false).await?;
            tokio::time::sleep_until(started_at + MIN_FAILURE_TIME).await;
            return Err(AppError::Auth("验证码错误或已失效".into()));
        };

        let mut hasher = Sha256::new();
        hasher.update(payload.code.as_bytes());
        let code_hash = hex::encode(hasher.finalize());

        let record = password_resets::Entity::find()
            .filter(password_resets::Column::UserId.eq(user.id))
            .filter(password_resets::Column::TokenHash.eq(code_hash))
            .one(&state.db)
            .await?;

        if let Some(record) = record {
            if Utc::now() > record.expires_at {
                Self::record_login_attempt(state, ip, email, false).await?;
                tokio::time::sleep_until(started_at + MIN_FAILURE_TIME).await;
                return Err(AppError::Auth("验证码错误或已失效".into()));
            }

            Self::record_login_attempt(state, ip, email, true).await?;

            let token = nanoid::nanoid!(48);
            let mut token_hasher = Sha256::new();
            token_hasher.update(token.as_bytes());
            let token_hash = hex::encode(token_hasher.finalize());

            let mut active: password_resets::ActiveModel = record.into();
            active.token_hash = Set(token_hash);
            active.expires_at = Set((Utc::now() + chrono::Duration::minutes(15)).into());
            active.update(&state.db).await?;

            Ok(token)
        } else {
            Self::record_login_attempt(state, ip, email, false).await?;
            tokio::time::sleep_until(started_at + MIN_FAILURE_TIME).await;
            Err(AppError::Auth("验证码错误或已失效".into()))
        }
    }

    pub async fn reset_password(
        db: &DatabaseConnection,
        payload: ResetPasswordRequest,
    ) -> Result<()> {
        let mut hasher = Sha256::new();
        hasher.update(payload.token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());

        let reset_record = password_resets::Entity::find()
            .filter(password_resets::Column::TokenHash.eq(token_hash))
            .one(db)
            .await?
            .ok_or(AppError::Auth("重置链接无效或已过期".into()))?;

        if Utc::now() > reset_record.expires_at {
            return Err(AppError::Auth("重置链接已过期".into()));
        }

        let user = users::Entity::find_by_id(reset_record.user_id)
            .one(db)
            .await?
            .ok_or(AppError::NotFound("用户不存在".into()))?;

        validate_password_strength(&payload.new_password)?;

        let salt = SaltString::generate(&mut OsRng);
        let new_hash = Argon2::default()
            .hash_password(payload.new_password.as_bytes(), &salt)
            .map_err(|e| AppError::Generic(format!("Password hashing failed: {}", e)))?
            .to_string();

        let mut user_active: users::ActiveModel = user.into();
        user_active.password_hash = Set(new_hash);
        user_active.update(db).await?;

        password_resets::Entity::delete_many()
            .filter(password_resets::Column::UserId.eq(reset_record.user_id))
            .exec(db)
            .await?;

        SessionService::cleanup_user_sessions(db, reset_record.user_id, 0).await?;
        Ok(())
    }

    pub async fn verify_user_credentials(
        db: &DatabaseConnection,
        username: &str,
        password: &str,
    ) -> Result<Option<(i32, String)>> {
        if let Some(user) = users::Entity::find()
            .filter(users::Column::Username.eq(username))
            .one(db)
            .await?
        {
            if let Ok(parsed_hash) = PasswordHash::new(&user.password_hash) {
                if Argon2::default()
                    .verify_password(password.as_bytes(), &parsed_hash)
                    .is_ok()
                {
                    return Ok(Some((user.id, user.role)));
                }
            }
        }
        Ok(None)
    }

    pub fn send_notification(
        db: DatabaseConnection,
        event_bus: niupanel_core::event_bus::EventBus,
        title: String,
        content: String,
        level: &'static str,
    ) {
        tokio::spawn(async move {
            let events = match SettingsManager::get(&db, NOTIFY_EVENTS).await {
                Ok(s) => s,
                _ => String::new(),
            };
            if events.contains("login") {
                event_bus.publish(CoreSystemEvent::System(
                    niupanel_core::event_bus::SystemNotification::Notification {
                        title,
                        content,
                        level: level.to_string(),
                    },
                ));
            }
        });
    }

    pub async fn check_rate_limit(state: &AppState, ip: &str, username: &str) -> Result<()> {
        let attempt = login_attempts::Entity::find()
            .filter(login_attempts::Column::IpAddress.eq(ip))
            .filter(login_attempts::Column::Username.eq(username))
            .one(&state.db)
            .await?;

        if let Some(record) = attempt {
            if let Some(banned_until) = record.banned_until {
                if Utc::now() < banned_until {
                    let remaining = banned_until.timestamp() - Utc::now().timestamp();
                    return Err(AppError::Forbidden(format!(
                        "账号/IP 已锁定，请等待 {} 秒后重试。",
                        remaining
                    )));
                }
            }
        }
        Ok(())
    }

    pub async fn record_login_attempt(
        state: &AppState,
        ip: &str,
        username: &str,
        success: bool,
    ) -> Result<()> {
        let attempt = login_attempts::Entity::find()
            .filter(login_attempts::Column::IpAddress.eq(ip))
            .filter(login_attempts::Column::Username.eq(username))
            .one(&state.db)
            .await?;

        if success {
            if let Some(record) = attempt {
                let mut active: login_attempts::ActiveModel = record.into();
                active.attempt_count = Set(0);
                active.banned_until = Set(None);
                active.last_attempt_at = Set(Utc::now().into());
                active.update(&state.db).await?;
            }
        } else {
            let now = Utc::now();
            if let Some(record) = attempt {
                let new_count = record.attempt_count + 1;
                let mut active: login_attempts::ActiveModel = record.into();
                active.attempt_count = Set(new_count);
                active.last_attempt_at = Set(now.into());

                if new_count >= MAX_FAILURES {
                    active.banned_until = Set(Some(
                        (now + chrono::Duration::minutes(COOLDOWN_MINUTES)).into(),
                    ));
                }
                active.update(&state.db).await?;
            } else {
                let active = login_attempts::ActiveModel {
                    ip_address: Set(ip.to_string()),
                    username: Set(username.to_string()),
                    attempt_count: Set(1),
                    last_attempt_at: Set(now.into()),
                    ..Default::default()
                };
                active.insert(&state.db).await?;
            }
        }
        Ok(())
    }

    pub async fn get_setup_status(db: &DatabaseConnection) -> Result<SetupStatus> {
        let count = users::Entity::find()
            .filter(users::Column::Id.ne(0))
            .count(db)
            .await?;
        Ok(SetupStatus {
            initialized: count > 0,
        })
    }

    pub async fn register_admin(
        db: &DatabaseConnection,
        ip: String,
        payload: RegisterRequest,
    ) -> Result<()> {
        let _registration_guard = ADMIN_REGISTRATION_LOCK
            .get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await;

        if payload.username.trim().len() < 3 {
            return Err(AppError::ValidationError(
                "用户名长度必须至少为 3 个字符".into(),
            ));
        }

        let email = if let Some(ref e) = payload.email {
            let e = e.trim();
            if e.is_empty() {
                None
            } else {
                if !e.contains('@') || e.len() < 5 {
                    return Err(AppError::ValidationError("无效的邮箱地址".into()));
                }
                Some(e.to_string())
            }
        } else {
            None
        };

        validate_password_strength(&payload.password)?;

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(payload.password.as_bytes(), &salt)
            .map_err(|e| AppError::Generic(format!("密码哈希失败: {}", e)))?
            .to_string();

        let txn = db.begin().await?;
        let user_count = users::Entity::find()
            .filter(users::Column::Id.ne(0))
            .count(&txn)
            .await?;
        if user_count > 0 {
            return Err(AppError::Forbidden(
                "系统已经初始化，无法重复创建管理员。".to_string(),
            ));
        }
        let new_admin = users::ActiveModel {
            username: Set(payload.username.clone()),
            email: Set(email.clone()),
            password_hash: Set(password_hash),
            role: Set("admin".to_string()),
            email_verified: Set(true),
            preferences: Set(Some(serde_json::json!({}))),
            ..Default::default()
        };
        let res = new_admin.insert(&txn).await?;

        if let Some(host) = payload.mail_host {
            if !host.trim().is_empty() {
                SettingsManager::set(&txn, MAIL_HOST, &host, Some("Email")).await?;
            }
        }
        if let Some(user) = payload.mail_username {
            if !user.trim().is_empty() {
                SettingsManager::set(&txn, MAIL_USERNAME, &user, Some("Email")).await?;
            }
        }
        if let Some(pass) = payload.mail_password {
            if !pass.trim().is_empty() {
                SettingsManager::set(&txn, MAIL_PASSWORD, &pass, Some("Email")).await?;
            }
        }
        if let Some(e) = email {
            SettingsManager::set(&txn, MAIL_TO, &e, Some("Email")).await?;
        }

        txn.commit().await?;

        AuditService::log(
            db,
            Some(res.id),
            "User",
            "SYSTEM_INIT",
            "user",
            Some(res.id.to_string()),
            Some(format!("系统初始化：创建管理员 {}", payload.username)),
            Some(ip),
        )
        .await;

        Ok(())
    }
}
