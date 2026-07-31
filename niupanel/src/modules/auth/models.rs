use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct IdentifyRequest {
    #[allow(dead_code)]
    pub username: String,
}

#[derive(Serialize, ToSchema)]
pub struct ResetInfoResponse {
    pub suffix: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    pub username: String,
    pub email: String,
}

#[derive(Deserialize, ToSchema)]
pub struct VerifyCodeRequest {
    pub email: String,
    pub code: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Serialize, ToSchema)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub role: String,
}

#[derive(Serialize, ToSchema)]
#[serde(untagged)]
pub enum LoginResponse {
    Requires2FA { ticket: String },
    Success(UserInfo),
}

#[derive(Deserialize, ToSchema)]
pub struct VerifyLogin2faRequest {
    pub ticket: String,
    pub code: String,
}

#[derive(Serialize, ToSchema)]
pub struct SetupStatus {
    pub initialized: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub mail_host: Option<String>,
    pub mail_username: Option<String>,
    pub mail_password: Option<String>,
}
