use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct EncryptCodeRequest {
    pub code: String,
    pub versions: Vec<String>,
    pub function_name: String,
    #[serde(default)]
    pub obfuscate: bool,
}
