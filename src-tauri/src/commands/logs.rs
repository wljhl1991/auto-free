use tauri::command;
use std::path::PathBuf;

/// 获取日志文件路径
#[command]
pub async fn get_log_path() -> Result<String, String> {
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("autofree")
        .join("logs");
    Ok(log_dir.to_string_lossy().to_string())
}

/// 读取最近的日志
#[command]
pub async fn read_recent_logs(lines: Option<usize>) -> Result<String, String> {
    let log_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("autofree")
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
