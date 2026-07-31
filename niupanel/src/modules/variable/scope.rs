use niupanel_common::error::{AppError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VariableScope {
    Global,
    Script,
}

impl VariableScope {
    pub(super) fn parse(scope: &str) -> Result<Self> {
        match scope {
            "Global" => Ok(VariableScope::Global),
            "Script" => Ok(VariableScope::Script),
            _ => Err(AppError::ValidationError(
                "Invalid scope. Must be 'Global' or 'Script'".to_string(),
            )),
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            VariableScope::Global => "Global",
            VariableScope::Script => "Script",
        }
    }

    pub(super) fn is_script(self) -> bool {
        matches!(self, VariableScope::Script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_scope_values() {
        assert_eq!(
            VariableScope::parse("Global").unwrap(),
            VariableScope::Global
        );
        assert_eq!(
            VariableScope::parse("Script").unwrap(),
            VariableScope::Script
        );
    }

    #[test]
    fn rejects_unknown_scope_values() {
        assert!(VariableScope::parse("Task").is_err());
    }
}
