use tauri::command;
use std::path::PathBuf;

fn get_gen_base_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("gen")
}

/// 获取日志文件路径
#[command]
pub async fn get_log_path() -> Result<String, String> {
    let log_dir = get_gen_base_path().join("logs");
    Ok(log_dir.to_string_lossy().to_string())
}

/// 读取最近的日志
#[command]
pub async fn read_recent_logs(lines: Option<usize>) -> Result<String, String> {
    let log_path = get_gen_base_path()
        .join("logs")
        .join("autofree.log");

    if !log_path.exists() {
        return Ok("(暂无日志文件)".to_string());
    }

    let content = std::fs::read_to_string(&log_path)
        .map_err(|e| format!("读取日志失败: {}", e))?;

    let line_count = lines.unwrap_or(200);
    let all_lines: Vec<&str> = content.lines().collect();
    let start = all_lines.len().saturating_sub(line_count);
    Ok(all_lines[start..].join("\n"))
}

/// 读取最近的 AI 调用历史
#[command]
pub async fn read_call_history(lines: Option<usize>) -> Result<String, String> {
    let history_path = get_gen_base_path()
        .join("call-history")
        .join("call-history.jsonl");

    if !history_path.exists() {
        return Ok("[]".to_string());
    }

    let content = std::fs::read_to_string(&history_path)
        .map_err(|e| format!("读取调用历史失败: {}", e))?;

    let line_count = lines.unwrap_or(100);
    let all_lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    let start = all_lines.len().saturating_sub(line_count);
    let recent = &all_lines[start..];

    // 返回 JSON 数组格式
    Ok(format!("[{}]", recent.join(",")))
}
