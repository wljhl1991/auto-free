use async_trait::async_trait;
use super::{truncate_str, IAssetProvider, ProviderError};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus};
use tokio_tungstenite::{connect_async, tungstenite};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::Engine;

type HmacSha256 = Hmac<Sha256>;

/// 讯飞星火聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparkMessage {
    pub role: String,
    pub content: String,
}

/// 讯飞星火请求参数
#[derive(Debug, Serialize)]
struct SparkRequest {
    header: SparkHeader,
    parameter: SparkParameter,
    payload: SparkPayload,
}

#[derive(Debug, Serialize)]
struct SparkHeader {
    app_id: String,
    uid: String,
}

#[derive(Debug, Serialize)]
struct SparkParameter {
    chat: SparkChatParam,
}

#[derive(Debug, Serialize)]
struct SparkChatParam {
    domain: String,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct SparkPayload {
    message: SparkMessagePayload,
}

#[derive(Debug, Serialize)]
struct SparkMessagePayload {
    text: Vec<SparkMessage>,
}

/// 讯飞星火响应
#[derive(Debug, Deserialize)]
struct SparkResponse {
    header: SparkResponseHeader,
    payload: SparkResponsePayload,
}

#[derive(Debug, Deserialize)]
struct SparkResponseHeader {
    #[allow(dead_code)]
    code: i32,
    #[allow(dead_code)]
    message: String,
    #[allow(dead_code)]
    sid: String,
    #[allow(dead_code)]
    status: u32,
}

#[derive(Debug, Deserialize)]
struct SparkResponsePayload {
    choices: SparkChoices,
    #[allow(dead_code)]
    usage: Option<SparkUsage>,
}

#[derive(Debug, Deserialize)]
struct SparkChoices {
    text: Vec<SparkText>,
    #[allow(dead_code)]
    status: u32,
}

#[derive(Debug, Deserialize)]
struct SparkText {
    #[allow(dead_code)]
    content: String,
    #[allow(dead_code)]
    role: String,
    #[allow(dead_code)]
    index: i32,
}

#[derive(Debug, Deserialize)]
struct SparkUsage {
    #[allow(dead_code)]
    text: SparkTextUsage,
}

#[derive(Debug, Deserialize)]
struct SparkTextUsage {
    #[allow(dead_code)]
    question_tokens: u32,
    #[allow(dead_code)]
    prompt_tokens: u32,
    #[allow(dead_code)]
    completion_tokens: u32,
}

pub struct XfyunSparkProvider {
    config: AIProviderConfig,
    app_id: String,
    api_secret: String,
    api_key: String,
    default_domain: String,
    ws_url: String,
    asset_base_path: PathBuf,
}

impl XfyunSparkProvider {
    pub fn new(config: &AIProviderConfig, asset_base_path: PathBuf) -> Result<Self, ProviderError> {
        let extra_params = config.auth_config.extra_params.as_ref();

        let app_id = config.auth_config.api_key
            .as_ref()
            .map(|k| k.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("讯飞星火 appId 未配置".to_string()))?;

        let api_secret = extra_params
            .and_then(|p| p.get("apiSecret"))
            .map(|f| f.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("讯飞星火 apiSecret 未配置".to_string()))?;

        let api_key = extra_params
            .and_then(|p| p.get("apiKeyReal"))
            .map(|f| f.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("讯飞星火 apiKeyReal 未配置".to_string()))?;

        let default_domain = config.models
            .iter()
            .find(|m| m.is_default)
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "lite".to_string());

        let ws_url = config.models
            .iter()
            .find(|m| m.is_default)
            .and_then(|m| {
                let ep = m.endpoint.trim().to_string();
                if ep.is_empty() { None } else { Some(ep) }
            })
            .unwrap_or_else(|| "wss://spark-api.xf-yun.com/v1.1/chat".to_string());

        Ok(Self {
            config: config.clone(),
            app_id,
            api_secret,
            api_key,
            default_domain,
            ws_url,
            asset_base_path,
        })
    }

    /// 发送聊天请求
    pub async fn chat(&self, messages: Vec<SparkMessage>) -> Result<String, ProviderError> {
        let auth_url = self.generate_auth_url();

        log::info!("[XfyunSpark] 连接WebSocket: url={}, domain={}, messages_count={}", 
            self.ws_url, self.default_domain, messages.len());

        let (ws_stream, _) = connect_async(&auth_url)
            .await
            .map_err(|e| {
                log::error!("[XfyunSpark] WebSocket连接失败: {}", e);
                ProviderError::NetworkError(format!("WebSocket连接失败: {}", e))
            })?;
        log::info!("[XfyunSpark] WebSocket连接成功");

        let (mut write, mut read) = ws_stream.split();

        let max_tokens = self.config.models
            .iter()
            .find(|m| m.id == self.default_domain)
            .and_then(|m| m.max_tokens)
            .unwrap_or(2048);

        let request = SparkRequest {
            header: SparkHeader {
                app_id: self.app_id.clone(),
                uid: "autofree_user".to_string(),
            },
            parameter: SparkParameter {
                chat: SparkChatParam {
                    domain: self.default_domain.clone(),
                    temperature: 0.5,
                    max_tokens,
                },
            },
            payload: SparkPayload {
                message: SparkMessagePayload {
                    text: messages,
                },
            },
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| {
                log::error!("[XfyunSpark] 序列化请求失败: {}", e);
                ProviderError::GenerationFailed(format!("序列化请求失败: {}", e))
            })?;

        let truncated_req = if request_json.len() > 500 {
            format!("{}...(共{}字符)", truncate_str(&request_json, 500), request_json.len())
        } else {
            request_json.clone()
        };
        log::info!("[XfyunSpark] 发送请求: {}", truncated_req);

        write.send(tungstenite::Message::Text(request_json.into()))
            .await
            .map_err(|e| {
                log::error!("[XfyunSpark] 发送WebSocket消息失败: {}", e);
                ProviderError::NetworkError(format!("发送WebSocket消息失败: {}", e))
            })?;

        let mut full_content = String::new();

        while let Some(msg) = read.next().await {
            let msg = msg
                .map_err(|e| {
                    log::error!("[XfyunSpark] 接收WebSocket消息失败: {}", e);
                    ProviderError::NetworkError(format!("接收WebSocket消息失败: {}", e))
                })?;

            match msg {
                tungstenite::Message::Text(data) => {
                    let truncated = if data.len() > 500 {
                        format!("{}...(共{}字符)", truncate_str(&data, 500), data.len())
                    } else {
                        data.to_string()
                    };
                    log::debug!("[XfyunSpark] 收到消息: {}", truncated);

                    let response = self.parse_response(&data)?;

                    if response.header.code != 0 {
                        log::error!("[XfyunSpark] 星火API错误: code={}, message={}, sid={}", 
                            response.header.code, response.header.message, response.header.sid);
                        return Err(ProviderError::GenerationFailed(format!(
                            "星火API错误 (code={}): {}", response.header.code, response.header.message
                        )));
                    }

                    for text in &response.payload.choices.text {
                        full_content.push_str(&text.content);
                    }

                    // status=2 表示尾帧，响应结束
                    if response.header.status == 2 {
                        log::info!("[XfyunSpark] 响应完成: content_len={}", full_content.len());
                        super::save_raw_response("xfyun_spark", "chat", &full_content);
                        break;
                    }
                }
                tungstenite::Message::Close(_) => {
                    log::info!("[XfyunSpark] WebSocket连接关闭");
                    break;
                }
                _ => {}
            }
        }

        Ok(full_content)
    }

    /// 生成鉴权 URL
    fn generate_auth_url(&self) -> String {
        // 手动解析 ws_url 获取 host 和 path
        let (host, path) = parse_ws_url(&self.ws_url);

        let date = chrono_now_rfc1123();

        let signature = self.generate_signature(host, path, &date);

        let authorization_origin = format!(
            r#"api_key="{api_key}", algorithm="hmac-sha256", headers="host date request-line", signature="{signature}""#,
            api_key = self.api_key,
            signature = signature,
        );

        let authorization = base64::engine::general_purpose::STANDARD.encode(&authorization_origin);

        format!(
            "{}?authorization={}&date={}&host={}",
            self.ws_url,
            urlencoding_encode(&authorization),
            urlencoding_encode(&date),
            urlencoding_encode(host),
        )
    }

    /// 生成 HMAC-SHA256 签名
    fn generate_signature(&self, host: &str, path: &str, date: &str) -> String {
        let signature_origin = format!(
            "host: {}\ndate: {}\nGET {} HTTP/1.1",
            host, date, path
        );

        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(signature_origin.as_bytes());
        let result = mac.finalize();
        let code_bytes = result.into_bytes();
        base64::engine::general_purpose::STANDARD.encode(code_bytes)
    }

    /// 解析流式响应
    fn parse_response(&self, data: &str) -> Result<SparkResponse, ProviderError> {
        serde_json::from_str(data)
            .map_err(|e| {
                log::error!("[XfyunSpark] 解析星火响应失败: {} (原始数据前200字: {})", e, truncate_str(&data, 200));
                ProviderError::GenerationFailed(format!("解析星火响应失败: {}", e))
            })
    }

    fn generate_cache_key(asset_ref: &AssetRef) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(asset_ref.id.as_bytes());
        hasher.update(asset_ref.prompt.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }
}

/// 手动解析 WebSocket URL，提取 host 和 path
fn parse_ws_url(url: &str) -> (&str, &str) {
    // 期望格式: wss://host/path 或 ws://host/path
    let url = url.strip_prefix("wss://").unwrap_or(url);
    let url = url.strip_prefix("ws://").unwrap_or(url);

    if let Some(slash_pos) = url.find('/') {
        (&url[..slash_pos], &url[slash_pos..])
    } else {
        (url, "/")
    }
}

/// 生成 RFC1123 格式的当前时间
fn chrono_now_rfc1123() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let (year, month, day) = days_to_ymd(days_since_epoch as i32);

    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

    // 1970-01-01 是 Thursday
    let weekday_idx = ((days_since_epoch + 3) % 7) as usize;

    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        weekdays[weekday_idx], day, months[month as usize - 1], year, hours, minutes, seconds
    )
}

fn days_to_ymd(days: i32) -> (i32, i32, i32) {
    let mut year = (10000 * days + 14780) / 3652425;
    let mut day_of_year = days - (365 * year + year / 4 - year / 100 + year / 400);
    if day_of_year < 0 {
        year -= 1;
        day_of_year = days - (365 * year + year / 4 - year / 100 + year / 400);
    }
    let month = (100 * day_of_year + 52) / 3060;
    let day = day_of_year - (30 * month + (3 * (month + 4)) / 7 - 14);
    year += (month + 2) / 12;
    let month = ((month + 2) % 12) + 1;
    (year, month, day + 1)
}

/// URL 编码（百分比编码）
fn urlencoding_encode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[async_trait]
impl IAssetProvider for XfyunSparkProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        let messages = vec![
            SparkMessage {
                role: "user".to_string(),
                content: asset_ref.prompt.clone(),
            },
        ];

        let text = self.chat(messages).await?;

        let cache_key = asset_ref.cache_key.clone()
            .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

        let dest_dir = self.asset_base_path.join(&cache_key);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| ProviderError::GenerationFailed(format!("创建资源目录失败: {}", e)))?;

        let dest_path = dest_dir.join(format!("{}.txt", asset_ref.id));
        std::fs::write(&dest_path, &text)
            .map_err(|e| ProviderError::GenerationFailed(format!("写入文本文件失败: {}", e)))?;

        Ok(LocalAsset {
            id: asset_ref.id.clone(),
            asset_type: AssetType::Audio,
            local_path: dest_path.to_string_lossy().to_string(),
            source: AssetSource::AiGenerated,
            cache_key,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }

    async fn check_connectivity(&self) -> Result<ConnectivityCheck, ProviderError> {
        self.check_connectivity_with_prompt("hi").await
    }

    async fn check_connectivity_with_prompt(&self, prompt: &str) -> Result<ConnectivityCheck, ProviderError> {
        let start = SystemTime::now();

        let messages = vec![
            SparkMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            },
        ];

        let request_body = serde_json::to_string(&SparkRequest {
            header: SparkHeader {
                app_id: self.app_id.clone(),
                uid: "autofree_user".to_string(),
            },
            parameter: SparkParameter {
                chat: SparkChatParam {
                    domain: self.default_domain.clone(),
                    temperature: 0.5,
                    max_tokens: 2048,
                },
            },
            payload: SparkPayload {
                message: SparkMessagePayload {
                    text: messages.clone(),
                },
            },
        }).unwrap_or_default();
        let request_headers = r#"{"Authorization":"Bearer ***"}"#.to_string();

        let result = self.chat(messages).await;
        let latency = SystemTime::now()
            .duration_since(start)
            .unwrap_or_default()
            .as_millis() as u64;

        match result {
            Ok(response_text) => Ok(ConnectivityCheck {
                provider_id: self.config.id.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                status: ConnectivityStatus::Ok,
                latency: Some(latency),
                error_message: None,
                quota_info: None,
                response_preview: Some(truncate_str(&response_text, 500).to_string()),
                test_prompt: Some(prompt.to_string()),
                media_url: None,
                media_type: None,
                request_endpoint: Some(self.ws_url.clone()),
                request_model: Some(self.default_domain.clone()),
                request_headers: Some(request_headers.clone()),
                request_body: Some(truncate_str(&request_body, 2000).to_string()),
                response_status: Some(200),
            }),
            Err(ProviderError::AuthFailed(msg)) => Ok(ConnectivityCheck {
                provider_id: self.config.id.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                status: ConnectivityStatus::AuthFailed,
                latency: Some(latency),
                error_message: Some(msg),
                quota_info: None,
                response_preview: None,
                test_prompt: Some(prompt.to_string()),
                media_url: None,
                media_type: None,
                request_endpoint: Some(self.ws_url.clone()),
                request_model: Some(self.default_domain.clone()),
                request_headers: Some(request_headers.clone()),
                request_body: Some(truncate_str(&request_body, 2000).to_string()),
                response_status: Some(401),
            }),
            Err(ProviderError::QuotaExceeded(msg)) => Ok(ConnectivityCheck {
                provider_id: self.config.id.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                status: ConnectivityStatus::QuotaExceeded,
                latency: Some(latency),
                error_message: Some(msg),
                quota_info: None,
                response_preview: None,
                test_prompt: Some(prompt.to_string()),
                media_url: None,
                media_type: None,
                request_endpoint: Some(self.ws_url.clone()),
                request_model: Some(self.default_domain.clone()),
                request_headers: Some(request_headers.clone()),
                request_body: Some(truncate_str(&request_body, 2000).to_string()),
                response_status: Some(429),
            }),
            Err(e) => Ok(ConnectivityCheck {
                provider_id: self.config.id.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                status: ConnectivityStatus::NetworkError,
                latency: Some(latency),
                error_message: Some(format!("{:?}", e)),
                quota_info: None,
                response_preview: None,
                test_prompt: Some(prompt.to_string()),
                media_url: None,
                media_type: None,
                request_endpoint: Some(self.ws_url.clone()),
                request_model: Some(self.default_domain.clone()),
                request_headers: Some(request_headers),
                request_body: Some(truncate_str(&request_body, 2000).to_string()),
                response_status: None,
            }),
        }
    }

    fn supported_modalities(&self) -> Vec<AIModality> {
        vec![AIModality::Text]
    }

    fn provider_id(&self) -> &str {
        &self.config.id
    }
}
