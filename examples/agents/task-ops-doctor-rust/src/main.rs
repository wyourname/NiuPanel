use std::collections::HashSet;
use std::io::{self, BufRead, Write};

struct Context {
    request_id: String,
    task_id: i64,
    task_json: String,
    runs_json: String,
    log: String,
    node_settings_json: String,
    python_settings_json: String,
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
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut first = String::new();
    if let Err(err) = reader.read_line(&mut first) {
        emit_error("", "stdin_read_failed", &err.to_string());
        return;
    }
    let request_id = extract_json_string(&first, "request_id").unwrap_or_default();
    let task_id = extract_json_number(&first, "task_id").unwrap_or_default();
    if task_id <= 0 {
        emit_error(&request_id, "invalid_task_id", "input.task_id is required");
        return;
    }

    let task_json = match call_tool(&mut reader, &request_id, "task", "read_task", task_id) {
        Ok(value) => value,
        Err(err) => {
            emit_error(&request_id, "tool_read_task_failed", &err);
            return;
        }
    };
    let runs_json = match call_tool(
        &mut reader,
        &request_id,
        "runs",
        "read_recent_runs",
        task_id,
    ) {
        Ok(value) => value,
        Err(err) => {
            emit_error(&request_id, "tool_read_recent_runs_failed", &err);
            return;
        }
    };
    let log_json = match call_tool(&mut reader, &request_id, "log", "read_log_tail", task_id) {
        Ok(value) => value,
        Err(err) => {
            emit_error(&request_id, "tool_read_log_tail_failed", &err);
            return;
        }
    };
    let env_type = extract_json_string(&task_json, "env_type").unwrap_or_default();
    let node_settings_json = if env_type.to_lowercase().contains("node") {
        call_tool_no_task(&mut reader, &request_id, "node_settings", "read_node_settings")
            .unwrap_or_default()
    } else {
        String::new()
    };
    let python_settings_json = if env_type.to_lowercase().contains("python") {
        call_tool_no_task(
            &mut reader,
            &request_id,
            "python_settings",
            "read_python_settings",
        )
        .unwrap_or_default()
    } else {
        String::new()
    };

    let context = Context {
        request_id,
        task_id,
        task_json,
        runs_json,
        log: extract_json_string(&log_json, "log_excerpt").unwrap_or_default(),
        node_settings_json,
        python_settings_json,
    };
    emit_success(&context);
}

fn call_tool<R: BufRead>(
    reader: &mut R,
    request_id: &str,
    call_id: &str,
    tool: &str,
    task_id: i64,
) -> Result<String, String> {
    println!(
        "{{\"type\":\"tool_call\",\"request_id\":\"{}\",\"call_id\":\"{}\",\"tool\":\"{}\",\"input\":{{\"task_id\":{},\"limit\":5,\"lines\":180}}}}",
        json_escape(request_id),
        json_escape(call_id),
        json_escape(tool),
        task_id
    );
    io::stdout().flush().map_err(|err| err.to_string())?;
    read_tool_result(reader, call_id)
}

fn call_tool_no_task<R: BufRead>(
    reader: &mut R,
    request_id: &str,
    call_id: &str,
    tool: &str,
) -> Result<String, String> {
    println!(
        "{{\"type\":\"tool_call\",\"request_id\":\"{}\",\"call_id\":\"{}\",\"tool\":\"{}\",\"input\":{{}}}}",
        json_escape(request_id),
        json_escape(call_id),
        json_escape(tool)
    );
    io::stdout().flush().map_err(|err| err.to_string())?;
    read_tool_result(reader, call_id)
}

fn read_tool_result<R: BufRead>(reader: &mut R, call_id: &str) -> Result<String, String> {
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|err| err.to_string())?;
    if !line.contains("\"ok\":true") {
        return Err(format!("tool call {call_id} failed: {}", line.trim()));
    }
    Ok(line)
}

fn emit_success(context: &Context) {
    let output = analyze(context);
    println!(
        "{{\"request_id\":\"{}\",\"ok\":true,\"output\":{}}}",
        json_escape(&context.request_id),
        output
    );
}

fn analyze(context: &Context) -> String {
    let env_type = extract_json_string(&context.task_json, "env_type").unwrap_or_default();
    let requirements = extract_json_string(&context.task_json, "requirements").unwrap_or_default();
    let enabled = extract_json_bool(&context.task_json, "enabled").unwrap_or(true);
    let lower_log = context.log.to_lowercase();
    let mut findings = Vec::new();
    let mut recommendations = Vec::new();
    let mut next_actions = Vec::new();
    let mut root_causes = Vec::new();
    let mut deps = detect_missing_dependencies(&context.log);

    if !enabled {
        findings.push(Finding {
            title: "任务未启用".to_string(),
            detail: "禁用任务不会按计划自动运行。".to_string(),
            evidence: Some("enabled=false".to_string()),
        });
        recommendations.push(Recommendation {
            title: "确认是否需要启用任务".to_string(),
            detail: "如果期望定时运行，请在任务详情中启用。".to_string(),
        });
    }

    for dep in &deps {
        let is_node = env_type.to_lowercase().contains("node");
        let title = if is_node {
            "Node.js 依赖缺失"
        } else {
            "Python 依赖缺失"
        };
        findings.push(Finding {
            title: title.to_string(),
            detail: format!("日志显示运行时无法解析 `{dep}`。"),
            evidence: matching_line(&context.log, dep),
        });
        recommendations.push(Recommendation {
            title: "补充依赖声明".to_string(),
            detail: format!("把 `{dep}` 加入任务依赖，并确认依赖安装在当前运行环境中。"),
        });
        next_actions.push(format!("补充 `{dep}` 后重新运行任务。"));
    }

    if env_type.to_lowercase().contains("node") && requirements.trim().is_empty() && !deps.is_empty()
    {
        findings.push(Finding {
            title: "Node.js 依赖声明为空".to_string(),
            detail: "任务没有声明依赖，但日志显示脚本需要第三方包。".to_string(),
            evidence: Some("requirements empty".to_string()),
        });
        recommendations.push(Recommendation {
            title: "显式维护任务依赖".to_string(),
            detail: "不要依赖脚本 cwd 下的偶然 node_modules；将第三方包写入任务依赖或系统默认包。"
                .to_string(),
        });
    }

    if lower_log.contains("timeout") || lower_log.contains("timed out") {
        findings.push(Finding {
            title: "运行超时".to_string(),
            detail: "日志包含超时信号，可能是网络、代理或任务超时配置不足。".to_string(),
            evidence: matching_any_line(&context.log, &["timeout", "timed out", "Timeout"]),
        });
        recommendations.push(Recommendation {
            title: "检查网络并调整超时".to_string(),
            detail: "确认目标接口可访问，必要时提高任务超时时间。".to_string(),
        });
    }

    if lower_log.contains("401")
        || lower_log.contains("403")
        || lower_log.contains("unauthorized")
        || lower_log.contains("forbidden")
    {
        findings.push(Finding {
            title: "远端认证失败".to_string(),
            detail: "远端服务拒绝请求，常见原因是 Cookie、Token 或权限失效。".to_string(),
            evidence: matching_any_line(&context.log, &["401", "403", "Unauthorized", "Forbidden"]),
        });
        recommendations.push(Recommendation {
            title: "检查凭据变量".to_string(),
            detail: "轮换或重新登录相关凭据，不要在日志或聊天中明文发送密钥。".to_string(),
        });
    }

    if context.runs_json.matches("\"status\":\"Failed\"").count() >= 3 {
        findings.push(Finding {
            title: "最近多次运行失败".to_string(),
            detail: "最近运行记录显示任务连续失败，优先处理根因后再恢复自动运行。".to_string(),
            evidence: None,
        });
    }

    deps.sort();
    deps.dedup();
    if deps.is_empty() && findings.is_empty() {
        root_causes.push("当前日志不足以确认明确根因。".to_string());
        next_actions.push("重新运行任务并保留完整日志，再进行诊断。".to_string());
    } else if !deps.is_empty() {
        root_causes.push(format!("运行环境缺少依赖：{}。", deps.join(", ")));
    } else if let Some(first) = findings.first() {
        root_causes.push(first.title.clone());
    }

    dedupe_findings(&mut findings);
    dedupe_recommendations(&mut recommendations);
    dedupe_strings(&mut next_actions);
    dedupe_strings(&mut root_causes);

    let severity = if findings
        .iter()
        .any(|finding| finding.title.contains("依赖") || finding.title.contains("认证"))
    {
        "critical"
    } else if findings.is_empty() {
        "info"
    } else {
        "warning"
    };
    let confidence = if deps.is_empty() && findings.is_empty() {
        0.35
    } else if deps.is_empty() {
        0.7
    } else {
        0.92
    };
    let summary = if findings.is_empty() {
        format!("任务 #{} 未发现明确异常。", context.task_id)
    } else if deps.is_empty() {
        format!("任务 #{} 存在 {} 个可疑问题。", context.task_id, findings.len())
    } else {
        format!("任务 #{} 依赖解析失败：{}。", context.task_id, deps.join(", "))
    };

    format!(
        "{{\"summary\":\"{}\",\"severity\":\"{}\",\"confidence\":{},\"root_causes\":{},\"findings\":{},\"recommendations\":{},\"next_actions\":{},\"safe_to_auto_fix\":false,\"metadata\":{{\"task_id\":{},\"env_type\":\"{}\",\"node_settings_seen\":{},\"python_settings_seen\":{}}}}}",
        json_escape(&summary),
        severity,
        confidence,
        json_string_array(&root_causes),
        findings_json(&findings),
        recommendations_json(&recommendations),
        json_string_array(&next_actions),
        context.task_id,
        json_escape(&env_type),
        !context.node_settings_json.is_empty(),
        !context.python_settings_json.is_empty()
    )
}

fn detect_missing_dependencies(log: &str) -> Vec<String> {
    let mut deps = Vec::new();
    for line in log.lines() {
        let lower = line.to_lowercase();
        if (lower.contains("cannot find module")
            || lower.contains("cannot find package")
            || lower.contains("no module named")
            || lower.contains("modulenotfounderror"))
            && quoted_value(line).is_some()
        {
            deps.push(quoted_value(line).unwrap());
        }
    }
    deps
}

fn quoted_value(line: &str) -> Option<String> {
    let start = line.find('\'').or_else(|| line.find('"'))?;
    let quote = line.as_bytes()[start] as char;
    let rest = &line[start + 1..];
    let end = rest.find(quote)?;
    let value = rest[..end].trim();
    (!value.is_empty()).then_some(value.to_string())
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

fn extract_json_number(source: &str, key: &str) -> Option<i64> {
    let needle = format!("\"{key}\"");
    let key_pos = source.find(&needle)?;
    let after_key = &source[key_pos + needle.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    let number = after_colon
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
        .collect::<String>();
    number.parse::<i64>().ok()
}

fn extract_json_bool(source: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let key_pos = source.find(&needle)?;
    let after_key = &source[key_pos + needle.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
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
