pub mod permissions;
pub mod sdk_token;

use crate::error::{AppError, Result};

pub fn validate_password_strength(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(AppError::ValidationError("密码长度至少为 8 位".into()));
    }

    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_digit = password.chars().any(|c| c.is_numeric());

    if !has_lowercase || !has_uppercase || !has_digit {
        return Err(AppError::ValidationError(
            "密码必须包含大写字母、小写字母和数字".into(),
        ));
    }

    Ok(())
}
