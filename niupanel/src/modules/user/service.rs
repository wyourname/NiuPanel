use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use niupanel_common::error::{AppError, Result};
use niupanel_entity::users;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

pub struct UserService;

impl UserService {
    /// 安全更新用户密码
    pub async fn update_password(
        db: &DatabaseConnection,
        user_id: i32,
        new_password: &str,
    ) -> Result<()> {
        let salt = SaltString::generate(&mut OsRng);
        let new_hash = Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| AppError::Generic(format!("Password hashing failed: {}", e)))?
            .to_string();

        let active: users::ActiveModel = users::ActiveModel {
            id: Set(user_id),
            password_hash: Set(new_hash),
            ..Default::default()
        };
        active.update(db).await?;
        Ok(())
    }

    /// 记录用户登录信息
    pub async fn record_login(db: &DatabaseConnection, user_id: i32, ip: String) -> Result<()> {
        let active: users::ActiveModel = users::ActiveModel {
            id: Set(user_id),
            last_login_at: Set(Some(chrono::Utc::now().into())),
            last_login_ip: Set(Some(ip)),
            ..Default::default()
        };
        active.update(db).await?;
        Ok(())
    }
}
