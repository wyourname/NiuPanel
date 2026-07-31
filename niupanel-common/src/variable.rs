use crate::error::{AppError, Result};

pub const MAX_VARIABLE_KEY_LEN: usize = 128;
pub const MAX_VARIABLE_VALUE_LEN: usize = 1024 * 1024;
pub const MAX_VARIABLE_REMARKS_LEN: usize = 2_000;
pub const RESERVED_VARIABLE_KEYS: &[&str] = &[
    "NIU_TASK_ID",
    "NIUPANEL_TASK_ID",
    "NIU_TASK_RUN_ID",
    "NIUPANEL_TASK_RUN_ID",
    "NIUPANEL_SDK_CONTEXT",
];

pub fn normalize_variable_key(key: &str) -> Result<String> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::ValidationError("变量名不能为空".to_string()));
    }
    if key.len() > MAX_VARIABLE_KEY_LEN {
        return Err(AppError::ValidationError(format!(
            "变量名不能超过 {MAX_VARIABLE_KEY_LEN} 个字节"
        )));
    }

    let mut chars = key.chars();
    let first = chars.next().expect("key is not empty");
    if !(first == '_' || first.is_ascii_alphabetic())
        || chars.any(|ch| !(ch == '_' || ch.is_ascii_alphanumeric()))
    {
        return Err(AppError::ValidationError(
            "变量名必须匹配 [A-Za-z_][A-Za-z0-9_]*".to_string(),
        ));
    }
    if RESERVED_VARIABLE_KEYS.contains(&key) {
        return Err(AppError::ValidationError(format!(
            "变量名 {key} 由 NiuPanel 运行时保留"
        )));
    }
    Ok(key.to_string())
}

pub fn validate_variable_value(value: &str) -> Result<()> {
    if value.len() > MAX_VARIABLE_VALUE_LEN {
        return Err(AppError::ValidationError(format!(
            "变量值不能超过 {MAX_VARIABLE_VALUE_LEN} 字节"
        )));
    }
    if value.contains('\0') {
        return Err(AppError::ValidationError(
            "变量值不能包含 NUL 字符".to_string(),
        ));
    }
    Ok(())
}

pub fn normalize_variable_remarks(remarks: Option<String>) -> Result<Option<String>> {
    let remarks = remarks
        .map(|remarks| remarks.trim().to_string())
        .filter(|remarks| !remarks.is_empty());
    if remarks
        .as_ref()
        .is_some_and(|value| value.len() > MAX_VARIABLE_REMARKS_LEN)
    {
        return Err(AppError::ValidationError(format!(
            "备注不能超过 {MAX_VARIABLE_REMARKS_LEN} 个字节"
        )));
    }
    Ok(remarks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_portable_keys_and_reserved_runtime_names() {
        assert_eq!(
            normalize_variable_key("  API_TOKEN  ").unwrap(),
            "API_TOKEN"
        );
        assert!(normalize_variable_key("1TOKEN").is_err());
        assert!(normalize_variable_key("TOKEN-NAME").is_err());
        assert!(normalize_variable_key("NIUPANEL_TASK_ID").is_err());
    }

    #[test]
    fn rejects_nul_values_and_normalizes_empty_remarks() {
        assert!(validate_variable_value("a\0b").is_err());
        assert_eq!(
            normalize_variable_remarks(Some("  ".to_string())).unwrap(),
            None
        );
    }
}
