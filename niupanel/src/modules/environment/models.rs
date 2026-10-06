use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Debug, ToSchema)]
pub struct EnvironmentInfo {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub env_type: String,
    pub is_installed: bool,
    pub recorded_packages: Option<usize>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateEnvRequest {
    pub version: String,
}

impl CreateEnvRequest {
    pub fn normalized_version(&self, env_type: &str) -> niupanel_common::error::Result<String> {
        let version = self.version.trim().trim_start_matches('v');
        let parts: Vec<_> = version.split('.').collect();
        if parts.len() > 3
            || (env_type == "node" && parts.len() != 3)
            || parts
                .iter()
                .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(niupanel_common::error::AppError::ValidationError(
                "请输入有效版本号；Node.js 需要完整版本，例如 22.23.1，Python 可填写 3.12"
                    .to_owned(),
            ));
        }
        Ok(version.to_owned())
    }
}

#[derive(Deserialize, ToSchema)]
pub struct InstallPackageRequest {
    pub packages: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SetMirrorSourceRequest {
    pub mirror_url: String,
}

impl InstallPackageRequest {
    pub fn validate(&self) -> niupanel_common::error::Result<()> {
        if self.packages.is_empty()
            || self
                .packages
                .iter()
                .any(|package| package.trim().is_empty() || package.trim_start().starts_with('-'))
        {
            return Err(niupanel_common::error::AppError::ValidationError(
                "请填写依赖包名，不要填写命令行选项".to_owned(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_requires_exact_version_and_python_accepts_minor_version() {
        let request = CreateEnvRequest {
            version: "22".to_owned(),
        };
        assert!(request.normalized_version("node").is_err());
        let request = CreateEnvRequest {
            version: " v22.23.1 ".to_owned(),
        };
        assert_eq!(request.normalized_version("node").unwrap(), "22.23.1");
        let request = CreateEnvRequest {
            version: "3.12".to_owned(),
        };
        assert_eq!(request.normalized_version("python").unwrap(), "3.12");
        let request = CreateEnvRequest {
            version: "../3.12".to_owned(),
        };
        assert!(request.normalized_version("python").is_err());
    }
}
