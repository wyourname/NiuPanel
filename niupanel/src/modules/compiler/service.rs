use super::models::EncryptCodeRequest;
use crate::modules::plugins::service::unified_plugin_service;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_plugin::PluginInvokeRequest;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use tokio::fs;

pub async fn supported_versions() -> Result<Vec<String>> {
    if let Some(versions) = plugin_supported_versions().await? {
        return Ok(versions);
    }

    Err(compiler_plugin_required("compiler.versions"))
}

pub async fn encrypt(req: EncryptCodeRequest) -> Result<String> {
    if let Some(filename) = encrypt_with_plugin(&req).await? {
        return Ok(filename);
    }

    Err(compiler_plugin_required("compiler.encrypt"))
}

async fn plugin_supported_versions() -> Result<Option<Vec<String>>> {
    let response = unified_plugin_service()
        .invoke_first_enabled_with_capability(
            "compiler.versions",
            PluginInvokeRequest {
                action: "versions".to_string(),
                input: Value::Null,
                timeout_sec: None,
            },
        )
        .await?;

    response
        .map(|response| parse_versions_output(response.output))
        .transpose()
}

async fn encrypt_with_plugin(req: &EncryptCodeRequest) -> Result<Option<String>> {
    let compile_dir = ensure_compile_dir().await?;
    let target_filename = new_compiled_filename();
    let mut input = serde_json::to_value(req)?;
    if let Value::Object(object) = &mut input {
        object.insert(
            "output_dir".to_string(),
            Value::String(compile_dir.display().to_string()),
        );
        object.insert(
            "target_filename".to_string(),
            Value::String(target_filename.clone()),
        );
    }

    let response = unified_plugin_service()
        .invoke_first_enabled_with_capability(
            "compiler.encrypt",
            PluginInvokeRequest {
                action: "encrypt".to_string(),
                input,
                timeout_sec: None,
            },
        )
        .await?;

    if let Some(response) = response {
        return Ok(Some(
            parse_encrypt_output(response.output, &compile_dir, &target_filename).await?,
        ));
    }

    Ok(None)
}

fn compiler_plugin_required(capability: &str) -> AppError {
    AppError::ValidationError(format!(
        "No enabled compiler plugin declared capability '{}'. Install a compiler plugin.",
        capability
    ))
}

fn parse_versions_output(output: Value) -> Result<Vec<String>> {
    let values = if let Some(values) = output.as_array() {
        values
    } else if let Some(values) = output.get("versions").and_then(Value::as_array) {
        values
    } else {
        return Err(AppError::ValidationError(
            "Compiler plugin versions output must be an array or {\"versions\": [...]}".to_string(),
        ));
    };

    values
        .iter()
        .map(|value| {
            value.as_str().map(str::to_string).ok_or_else(|| {
                AppError::ValidationError(
                    "Compiler plugin versions output must contain only strings".to_string(),
                )
            })
        })
        .collect()
}

async fn parse_encrypt_output(
    output: Value,
    compile_dir: &Path,
    target_filename: &str,
) -> Result<String> {
    if let Some(file_content) = output.get("file_content").and_then(Value::as_str) {
        return write_compiled_file(file_content, target_filename.to_string()).await;
    }

    if let Some(filename) = output.get("filename").and_then(Value::as_str) {
        validate_compiled_filename(filename)?;
        let target_path = compile_dir.join(filename);
        if !target_path.is_file() {
            return Err(AppError::ValidationError(format!(
                "Compiler plugin returned filename '{}' but the file does not exist",
                filename
            )));
        }
        return Ok(filename.to_string());
    }

    Err(AppError::ValidationError(
        "Compiler plugin encrypt output must include file_content or filename".to_string(),
    ))
}

async fn write_compiled_file(content: &str, filename: String) -> Result<String> {
    let compile_dir = ensure_compile_dir().await?;
    let target_path = compile_dir.join(&filename);
    validate_compiled_filename(&filename)?;
    fs::write(&target_path, content).await?;
    Ok(filename)
}

async fn ensure_compile_dir() -> Result<PathBuf> {
    let compile_dir = Config::global().scripts_dir.join("compile");
    if !compile_dir.exists() {
        fs::create_dir_all(&compile_dir).await?;
    }
    Ok(compile_dir)
}

fn new_compiled_filename() -> String {
    format!("compiled_{}.py", nanoid::nanoid!(8))
}

fn validate_compiled_filename(filename: &str) -> Result<()> {
    let mut components = Path::new(filename).components();
    let is_safe = matches!(components.next(), Some(Component::Normal(name)) if !name.is_empty())
        && components.next().is_none();
    if is_safe {
        Ok(())
    } else {
        Err(AppError::ValidationError(
            "Compiled filename must be a single file name".to_string(),
        ))
    }
}
