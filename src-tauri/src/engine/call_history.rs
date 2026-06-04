use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// 单次 AI 调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallRecord {
    /// 调用时间 (ISO 8601)
    pub timestamp: String,
    /// Provider ID (如 deepseek, siliconflow)
    pub provider_id: String,
    /// 模态 (text, image, video, music, voice)
    pub modality: String,
    /// 模型名称
    pub model: String,
    /// API 端点
    pub endpoint: String,
    /// 请求 prompt（截断到 500 字符）
    pub prompt: String,
    /// 调用耗时（毫秒）
    pub duration_ms: u64,
    /// 结果状态: success / error / timeout
    pub status: String,
    /// 错误信息（如有）
    pub error: Option<String>,
    /// 响应内容（截断到 300 字符）
    pub response_preview: Option<String>,
    /// Token 用量（如有）
    pub token_usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

/// 调用历史记录器
pub struct CallHistory {
    file_path: PathBuf,
    buffer: Mutex<Vec<String>>,
}

impl CallHistory {
    /// 创建记录器，文件路径为 data_dir/autofree/logs/call-history.jsonl
    pub fn new(data_dir: &PathBuf) -> Self {
        let log_dir = data_dir.join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let file_path = log_dir.join("call-history.jsonl");
        Self {
            file_path,
            buffer: Mutex::new(Vec::new()),
        }
    }

    /// 记录一次调用
    pub fn record(&self, record: CallRecord) {
        let line = match serde_json::to_string(&record) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("序列化调用记录失败: {}", e);
                return;
            }
        };

        // 先写入缓冲区
        {
            let mut buf = self.buffer.lock().unwrap();
            buf.push(line.clone());
            // 缓冲区超过 10 条时刷盘
            if buf.len() >= 10 {
                self.flush_locked(&mut buf);
            }
        }

        // 同时用 log::info 输出，方便日志文件也记录
        log::info!(
            "AI调用记录: provider={}, model={}, modality={}, duration={}ms, status={}",
            record.provider_id, record.model, record.modality, record.duration_ms, record.status
        );
    }

    /// 刷盘
    pub fn flush(&self) {
        let mut buf = self.buffer.lock().unwrap();
        self.flush_locked(&mut buf);
    }

    fn flush_locked(&self, buf: &mut Vec<String>) {
        if buf.is_empty() {
            return;
        }
        let content: String = buf.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join("\n") + "\n";
        if let Err(e) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, content.as_bytes()))
        {
            log::warn!("写入调用历史文件失败: {}", e);
        }
        buf.clear();
    }
}

impl Drop for CallHistory {
    fn drop(&mut self) {
        self.flush();
    }
}

/// 安全截断字符串
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut boundary = max_len;
    while boundary > 0 && !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    s[..boundary].to_string()
}

/// 构建调用记录的辅助函数
pub fn build_record(
    provider_id: &str,
    modality: &str,
    model: &str,
    endpoint: &str,
    prompt: &str,
    duration_ms: u64,
    status: &str,
    error: Option<String>,
    response_preview: Option<String>,
    token_usage: Option<TokenUsage>,
) -> CallRecord {
    CallRecord {
        timestamp: chrono::Local::now().to_rfc3339(),
        provider_id: provider_id.to_string(),
        modality: modality.to_string(),
        model: model.to_string(),
        endpoint: endpoint.to_string(),
        prompt: truncate(prompt, 500),
        duration_ms,
        status: status.to_string(),
        error,
        response_preview: response_preview.map(|s| truncate(&s, 300)),
        token_usage,
    }
}
