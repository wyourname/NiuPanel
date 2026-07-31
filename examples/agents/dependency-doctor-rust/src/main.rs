use std::collections::HashSet;
use std::io::{self, Read};

#[derive(Clone)]
struct InputContext {
    env_type: String,
    requirements: String,
    log: String,
    cwd: String,
}

struct Finding {
    title: String,
    detail: String,
    evidence: Option<String>,
}

struct Recommendation {
    title: String,
    detail: String,
}

fn main() {
    let mut body = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut body) {
        emit_error("", "stdin_read_failed", &err.to_string());
        return;
    }

    let request_id = extract_json_string(&body, "request_id").unwrap_or_default();
    let action = extract_json_string(&body, "action").unwrap_or_else(|| "invoke".to_string());
    if !matches!(
        action.as_str(),
        "invoke" | "diagnose" | "analyze_dependencies"
    ) {
        emit_error(
            &request_id,
            "unsupported_action",
            "supported actions: invoke, diagnose, analyze_dependencies",
        );
        return;
    }

    let context = InputContext {
        env_type: extract_json_string(&body, "env_type").unwrap_or_default(),
        requirements: extract_json_string(&body, "requirements").unwrap_or_default(),
        log: extract_json_string(&body, "log")
            .or_else(|| extract_json_string(&body, "log_excerpt"))
            .unwrap_or_default(),
        cwd: extract_json_string(&body, "cwd")
            .or_else(|| extract_json_string(&body, "working_directory"))
            .unwrap_or_default(),
    };

    let output = analyze_dependencies(&context);
    println!(
        "{{\"request_id\":\"{}\",\"ok\":true,\"output\":{}}}",
        json_escape(&request_id),
        output
    );
}

fn analyze_dependencies(context: &InputContext) -> String {
    let lower_log = context.log.to_lowercase();
    let lower_env = context.env_type.to_lowercase();
    let mut findings = Vec::new();
    let mut recommendations = Vec::new();
    let mut next_actions = Vec::new();
    let mut detected_dependencies = Vec::new();

    for dep in detect_node_missing_modules(&context.log) {
        detected_dependencies.push(dep.clone());
        findings.push(Finding {
            title: "Node.js 模块解析失败".to_string(),
            detail: format!("运行日志显示 Node.js 无法解析依赖 `{dep}`。"),
            evidence: matching_line(&context.log, &dep),
        });
        recommendations.push(Recommendation {
            title: "确认依赖安装位置".to_string(),
            detail: format!(
                "把 `{dep}` 加入任务依赖或全局 Node 默认包，并确认安装目录与当前 Node 版本一致。"
            ),
        });
        next_actions.push(format!("在任务依赖中加入 `{dep}` 后重新运行。"));
    }

    for dep in detect_python_missing_modules(&context.log) {
        detected_dependencies.push(dep.clone());
        findings.push(Finding {
            title: "Python 模块缺失".to_string(),
            detail: format!("运行日志显示 Python 无法导入模块 `{dep}`。"),
            evidence: matching_line(&context.log, &dep),
        });
        recommendations.push(Recommendation {
            title: "补充 Python requirements".to_string(),
            detail: format!("把 `{dep}` 对应的 pip 包加入 requirements，并重新同步环境。"),
        });
        next_actions.push(format!("确认 `{dep}` 在当前 Python 虚拟环境中可导入。"));
    }

    if lower_log.contains("npm err") || lower_log.contains("npm error") {
        findings.push(Finding {
            title: "npm 安装过程报错".to_string(),
            detail: "日志包含 npm 错误，可能与 registry、网络、包版本或 lockfile 冲突有关。"
                .to_string(),
            evidence: matching_any_line(&context.log, &["npm ERR", "npm error"]),
        });
        recommendations.push(Recommendation {
            title: "检查 npm registry 和包版本".to_string(),
            detail:
                "确认容器内 registry 可访问，依赖版本存在，并避免同时混用 npm/pnpm/yarn lockfile。"
                    .to_string(),
        });
        next_actions
            .push("在容器内执行 npm view <package> version 验证 registry 可用。".to_string());
    }

    if lower_env.contains("node") && context.requirements.trim().is_empty() {
        findings.push(Finding {
            title: "Node.js 任务没有依赖声明".to_string(),
            detail: "任务依赖字段为空，但日志可能依赖第三方包；面板不会凭脚本内容自动安装所有包。"
                .to_string(),
            evidence: Some("requirements empty".to_string()),
        });
        recommendations.push(Recommendation {
            title: "显式声明第三方依赖".to_string(),
            detail: "把脚本 require/import 的第三方包写入任务依赖，或配置系统 Node 默认包。"
                .to_string(),
        });
    }

    if lower_log.contains("cwd:") || !context.cwd.trim().is_empty() {
        next_actions.push("确认脚本运行 cwd 与 node_modules 所在目录是否一致。".to_string());
    }

    detected_dependencies.sort();
    detected_dependencies.dedup();
    dedupe_findings(&mut findings);
    dedupe_recommendations(&mut recommendations);
    dedupe_strings(&mut next_actions);

    let severity = if findings
        .iter()
        .any(|finding| finding.title.contains("模块") || finding.title.contains("安装"))
    {
        "critical"
    } else if findings.is_empty() {
        "info"
    } else {
        "warning"
    };

    let summary = if findings.is_empty() {
        "没有发现明确的依赖解析异常；需要更多日志才能判断。".to_string()
    } else if detected_dependencies.is_empty() {
        format!("发现 {} 个依赖或包管理器相关问题。", findings.len())
    } else {
        format!(
            "发现依赖解析失败，重点检查：{}。",
            detected_dependencies.join(", ")
        )
    };

    let confidence = if detected_dependencies.is_empty() && findings.is_empty() {
        0.35
    } else if detected_dependencies.is_empty() {
        0.68
    } else {
        0.9
    };

    format!(
        "{{\"summary\":\"{}\",\"severity\":\"{}\",\"confidence\":{},\"detected_dependencies\":{},\"findings\":{},\"recommendations\":{},\"next_actions\":{},\"metadata\":{{\"env_type\":\"{}\",\"has_requirements\":{},\"has_log\":{}}}}}",
        json_escape(&summary),
        severity,
        confidence,
        json_string_array(&detected_dependencies),
        findings_json(&findings),
        recommendations_json(&recommendations),
        json_string_array(&next_actions),
        json_escape(&context.env_type),
        !context.requirements.trim().is_empty(),
        !context.log.trim().is_empty()
    )
}

fn detect_node_missing_modules(log: &str) -> Vec<String> {
    let mut modules = Vec::new();
    for line in log.lines() {
        let lower = line.to_lowercase();
        if (lower.contains("cannot find module")
            || lower.contains("cannot find package")
            || lower.contains("err_module_not_found"))
            && quoted_value(line).is_some()
        {
            modules.push(quoted_value(line).unwrap());
        }
    }
    modules
}

fn detect_python_missing_modules(log: &str) -> Vec<String> {
    let mut modules = Vec::new();
    for line in log.lines() {
        let lower = line.to_lowercase();
        if (lower.contains("no module named") || lower.contains("modulenotfounderror"))
            && quoted_value(line).is_some()
        {
            modules.push(quoted_value(line).unwrap());
        }
    }
    modules
}

fn quoted_value(line: &str) -> Option<String> {
    let start = line.find('\'').or_else(|| line.find('"'))?;
    let quote = line.as_bytes()[start] as char;
    let rest = &line[start + 1..];
    let end = rest.find(quote)?;
    let value = rest[..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn matching_line(log: &str, needle: &str) -> Option<String> {
    log.lines()
        .rev()
        .find(|line| line.contains(needle))
        .map(trim_evidence)
}

fn matching_any_line(log: &str, needles: &[&str]) -> Option<String> {
    log.lines()
        .rev()
        .find(|line| needles.iter().any(|needle| line.contains(needle)))
        .map(trim_evidence)
}

fn trim_evidence(line: &str) -> String {
    line.trim().chars().take(500).collect()
}

fn dedupe_findings(findings: &mut Vec<Finding>) {
    let mut seen = HashSet::new();
    findings.retain(|finding| seen.insert(finding.title.clone()));
}

fn dedupe_recommendations(recommendations: &mut Vec<Recommendation>) {
    let mut seen = HashSet::new();
    recommendations.retain(|item| seen.insert(item.title.clone()));
}

fn dedupe_strings(items: &mut Vec<String>) {
    let mut seen = HashSet::new();
    items.retain(|item| seen.insert(item.clone()));
}

fn findings_json(findings: &[Finding]) -> String {
    let items = findings
        .iter()
        .map(|finding| {
            format!(
                "{{\"title\":\"{}\",\"detail\":\"{}\",\"evidence\":{}}}",
                json_escape(&finding.title),
                json_escape(&finding.detail),
                optional_json_string(finding.evidence.as_deref())
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", items.join(","))
}

fn recommendations_json(recommendations: &[Recommendation]) -> String {
    let items = recommendations
        .iter()
        .map(|item| {
            format!(
                "{{\"title\":\"{}\",\"detail\":\"{}\"}}",
                json_escape(&item.title),
                json_escape(&item.detail)
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", items.join(","))
}

fn json_string_array(items: &[String]) -> String {
    let items = items
        .iter()
        .map(|item| format!("\"{}\"", json_escape(item)))
        .collect::<Vec<_>>();
    format!("[{}]", items.join(","))
}

fn optional_json_string(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn extract_json_string(source: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let key_pos = source.find(&needle)?;
    let after_key = &source[key_pos + needle.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }
    parse_json_string(after_colon)
}

fn parse_json_string(source: &str) -> Option<String> {
    let mut escaped = false;
    let mut result = String::new();
    let mut chars = source.chars();
    if chars.next()? != '"' {
        return None;
    }
    while let Some(ch) = chars.next() {
        if escaped {
            match ch {
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                '/' => result.push('/'),
                'b' => result.push('\u{0008}'),
                'f' => result.push('\u{000c}'),
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                'u' => {
                    let code = chars.by_ref().take(4).collect::<String>();
                    if let Ok(value) = u16::from_str_radix(&code, 16) {
                        if let Some(decoded) = char::from_u32(value as u32) {
                            result.push(decoded);
                        }
                    }
                }
                _ => result.push(ch),
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(result);
        } else {
            result.push(ch);
        }
    }
    None
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn emit_error(request_id: &str, code: &str, message: &str) {
    println!(
        "{{\"request_id\":\"{}\",\"ok\":false,\"error\":{{\"code\":\"{}\",\"message\":\"{}\"}}}}",
        json_escape(request_id),
        json_escape(code),
        json_escape(message)
    );
}
