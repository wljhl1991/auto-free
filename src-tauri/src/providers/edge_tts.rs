use async_trait::async_trait;
use super::{IAssetProvider, ProviderError, truncate_str};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite};
use tungstenite::client::IntoClientRequest;
use uuid::Uuid;

const WIN_EPOCH: i64 = 11644473600;
const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const CHROMIUM_VERSION: &str = "143";

/// 默认输出格式（默认使用较高质量）
const DEFAULT_OUTPUT_FORMAT: &str = "audio-24khz-96kbitrate-mono-mp3";

/// Edge TTS 语音质量参数
#[derive(Debug, Clone)]
pub(crate) struct TtsQualityParams {
    /// 语速：如 "+0%", "-10%", "+20%", 范围 -50% ~ +100%
    rate: String,
    /// 音调：如 "+0Hz", "-5Hz", "+10Hz", 范围 -50Hz ~ +50Hz
    pitch: String,
    /// 音量：如 "medium", "loud", "x-loud", "soft", "+10dB", "-5dB"
    volume: String,
    /// 输出格式：音频编码格式和质量
    output_format: String,
    /// 音色：如 "zh-CN-XiaoxiaoNeural", "zh-CN-YunjianNeural" 等
    voice: String,
}

impl Default for TtsQualityParams {
    fn default() -> Self {
        Self {
            rate: "+0%".to_string(),
            pitch: "+0Hz".to_string(),
            volume: "medium".to_string(),
            output_format: DEFAULT_OUTPUT_FORMAT.to_string(),
            voice: String::new(),
        }
    }
}

impl TtsQualityParams {
    /// 从模型的 advanced_params 中提取参数
    fn from_model_advanced_params(config: &AIProviderConfig) -> Self {
        let default_model = config.models.iter().find(|m| m.is_default)
            .or_else(|| config.models.first());
        let mut params = Self::default();
        if let Some(model) = default_model {
            if let Some(ref adv) = model.advanced_params {
                if let Some(v) = adv.get("voice").and_then(|v| v.as_str()) {
                    params.voice = v.to_string();
                }
                if let Some(v) = adv.get("rate").and_then(|v| v.as_str()) {
                    params.rate = v.to_string();
                }
                if let Some(v) = adv.get("pitch").and_then(|v| v.as_str()) {
                    params.pitch = v.to_string();
                }
                if let Some(v) = adv.get("volume").and_then(|v| v.as_str()) {
                    params.volume = v.to_string();
                }
                if let Some(v) = adv.get("output_format").and_then(|v| v.as_str()) {
                    params.output_format = v.to_string();
                }
            }
        }
        params
    }
}

/// Edge TTS 支持的中文语音
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChineseVoice {
    #[serde(rename = "zh-CN-XiaoxiaoNeural")]
    XiaoxiaoNeural, // 女声，温柔
    #[serde(rename = "zh-CN-XiaoyiNeural")]
    XiaoyiNeural, // 女声，活泼
    #[serde(rename = "zh-CN-YunjianNeural")]
    YunjianNeural, // 男声，沉稳
    #[serde(rename = "zh-CN-YunxiNeural")]
    YunxiNeural, // 男声，阳光
    #[serde(rename = "zh-CN-YunxiaNeural")]
    YunxiaNeural, // 男声，少年
    #[serde(rename = "zh-CN-YunyangNeural")]
    YunyangNeural, // 男声，新闻
    #[serde(rename = "zh-CN-liaoning-XiaobeiNeural")]
    XiaobeiNeural, // 东北话女声
    #[serde(rename = "zh-CN-shaanxi-XiaoniNeural")]
    XiaoniNeural, // 陕西话女声
}

impl ChineseVoice {
    /// 获取语音的 SSML 名称
    fn as_str(&self) -> &str {
        match self {
            ChineseVoice::XiaoxiaoNeural => "zh-CN-XiaoxiaoNeural",
            ChineseVoice::XiaoyiNeural => "zh-CN-XiaoyiNeural",
            ChineseVoice::YunjianNeural => "zh-CN-YunjianNeural",
            ChineseVoice::YunxiNeural => "zh-CN-YunxiNeural",
            ChineseVoice::YunxiaNeural => "zh-CN-YunxiaNeural",
            ChineseVoice::YunyangNeural => "zh-CN-YunyangNeural",
            ChineseVoice::XiaobeiNeural => "zh-CN-liaoning-XiaobeiNeural",
            ChineseVoice::XiaoniNeural => "zh-CN-shaanxi-XiaoniNeural",
        }
    }
}

/// 生成 MUID
fn generate_muid() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 16];
    rng.fill(&mut bytes);
    bytes.iter().map(|b| format!("{:02X}", b)).collect()
}

/// 生成 Sec-MS-GEC Token
fn generate_sec_ms_gec() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // 转换到 Windows 时间戳并向下舍入到最近 5 分钟
    let ticks = (now + WIN_EPOCH) - (now + WIN_EPOCH) % 300;
    // 转换为 100 纳秒间隔
    let ticks = ticks as f64 * 1e7;

    let str_to_hash = format!("{:.0}{}", ticks, TRUSTED_CLIENT_TOKEN);
    let mut hasher = Sha256::new();
    hasher.update(str_to_hash.as_bytes());
    let result = hasher.finalize();
    hex::encode_upper(result)
}

/// 获取当前日期字符串格式（JavaScript 风格）
fn date_to_string() -> String {
    use chrono::{Datelike, Timelike, Utc};
    let now = Utc::now();
    let weekday = match now.weekday() {
        chrono::Weekday::Mon => "Mon",
        chrono::Weekday::Tue => "Tue",
        chrono::Weekday::Wed => "Wed",
        chrono::Weekday::Thu => "Thu",
        chrono::Weekday::Fri => "Fri",
        chrono::Weekday::Sat => "Sat",
        chrono::Weekday::Sun => "Sun",
    };
    let month = match now.month() {
        1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr", 5 => "May", 6 => "Jun",
        7 => "Jul", 8 => "Aug", 9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
        _ => "Jan",
    };
    format!("{} {} {} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
            weekday, month, now.day(), now.hour(), now.minute(), now.second())
}



pub struct EdgeTTSProvider {
    default_voice: ChineseVoice,
    config: AIProviderConfig,
    asset_base_path: PathBuf,
}

impl EdgeTTSProvider {
    pub fn new(config: &AIProviderConfig, asset_base_path: &std::path::Path) -> Result<Self, ProviderError> {
        let default_voice = config
            .models
            .iter()
            .find(|m| m.is_default)
            .and_then(|m| match m.id.as_str() {
                "zh-CN-XiaoxiaoNeural" => Some(ChineseVoice::XiaoxiaoNeural),
                "zh-CN-XiaoyiNeural" => Some(ChineseVoice::XiaoyiNeural),
                "zh-CN-YunjianNeural" => Some(ChineseVoice::YunjianNeural),
                "zh-CN-YunxiNeural" => Some(ChineseVoice::YunxiNeural),
                "zh-CN-YunxiaNeural" => Some(ChineseVoice::YunxiaNeural),
                "zh-CN-YunyangNeural" => Some(ChineseVoice::YunyangNeural),
                "zh-CN-liaoning-XiaobeiNeural" => Some(ChineseVoice::XiaobeiNeural),
                "zh-CN-shaanxi-XiaoniNeural" => Some(ChineseVoice::XiaoniNeural),
                _ => None,
            })
            .unwrap_or(ChineseVoice::YunjianNeural);

        Ok(Self {
            default_voice,
            config: config.clone(),
            asset_base_path: asset_base_path.to_path_buf(),
        })
    }

    /// 生成语音
    pub async fn synthesize(
        &self,
        text: &str,
        voice_id: &str,
        params: &TtsQualityParams,
    ) -> Result<Vec<u8>, ProviderError> {
        let voice_id = if voice_id.is_empty() { self.default_voice.as_str() } else { voice_id };
        let ssml = self.build_ssml(text, voice_id, params);
        self.connect_and_synthesize(&ssml, voice_id, &params.output_format).await
    }

    /// 根据 NPC 性别和性格选择语音
    #[allow(dead_code)]
    pub fn select_voice(&self, gender: &str, personality: &str) -> ChineseVoice {
        let lower_personality = personality.to_lowercase();
        match gender.to_lowercase().as_str() {
            "male" | "男" | "男性" => {
                if lower_personality.contains("阳光")
                    || lower_personality.contains("cheerful")
                    || lower_personality.contains("活泼")
                {
                    ChineseVoice::YunxiNeural
                } else {
                    ChineseVoice::YunjianNeural
                }
            }
            "female" | "女" | "女性" => {
                if lower_personality.contains("活泼")
                    || lower_personality.contains("cheerful")
                    || lower_personality.contains("阳光")
                {
                    ChineseVoice::XiaoyiNeural
                } else {
                    ChineseVoice::XiaoxiaoNeural
                }
            }
            _ => ChineseVoice::YunyangNeural, // 旁白 → 新闻风格
        }
    }

    /// 构造 SSML
    fn build_ssml(&self, text: &str, voice_id: &str, params: &TtsQualityParams) -> String {
        format!(
            "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='zh-CN'>\
             <voice name='{}'>\
             <prosody pitch='{}' rate='{}' volume='{}'>\
             {}\
             </prosody>\
             </voice>\
             </speak>",
            voice_id,
            params.pitch,
            params.rate,
            params.volume,
            text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        )
    }

    /// 通过 WebSocket 连接 Edge TTS 并获取音频
    async fn connect_and_synthesize(
        &self,
        ssml: &str,
        voice_id: &str,
        output_format: &str,
    ) -> Result<Vec<u8>, ProviderError> {
        let _request_id = Uuid::new_v4().to_string();
        let connection_id = Uuid::new_v4().to_string();
        let sec_ms_gec = generate_sec_ms_gec();
        let muid = generate_muid();
        let timestamp = date_to_string();

        let ws_url = format!(
            "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={}&ConnectionId={}&Sec-MS-GEC={}&Sec-MS-GEC-Version=1-{}.0.3650.75",
            TRUSTED_CLIENT_TOKEN, connection_id, sec_ms_gec, CHROMIUM_VERSION
        );
        log::info!("[EdgeTTS] 连接WebSocket: voice={}, text_len={}", voice_id, ssml.len());

        // 构建带必要请求头的 WebSocket 请求（微软要求包含多个头部）
        // 使用 IntoClientRequest 让 tungstenite 自动添加 sec-websocket-key 等必需头
        let mut request = ws_url.into_client_request()
            .map_err(|e| ProviderError::NetworkError(format!("构建WebSocket请求失败: {}", e)))?;
        
        // 添加微软服务所需的所有请求头
        request.headers_mut().insert(
            "User-Agent",
            format!("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{}.0.0.0 Safari/537.36 Edg/{}.0.0.0", CHROMIUM_VERSION, CHROMIUM_VERSION).parse()
                .map_err(|e| ProviderError::NetworkError(format!("解析User-Agent失败: {}", e)))?,
        );
        request.headers_mut().insert(
            "Origin",
            "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold".parse()
                .map_err(|e| ProviderError::NetworkError(format!("解析Origin失败: {}", e)))?,
        );
        request.headers_mut().insert(
            "Referer",
            "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold".parse()
                .map_err(|e| ProviderError::NetworkError(format!("解析Referer失败: {}", e)))?,
        );
        request.headers_mut().insert(
            "Pragma",
            "no-cache".parse()
                .map_err(|e| ProviderError::NetworkError(format!("解析Pragma失败: {}", e)))?,
        );
        request.headers_mut().insert(
            "Cache-Control",
            "no-cache".parse()
                .map_err(|e| ProviderError::NetworkError(format!("解析Cache-Control失败: {}", e)))?,
        );
        request.headers_mut().insert(
            "Accept-Encoding",
            "gzip, deflate, br, zstd".parse()
                .map_err(|e| ProviderError::NetworkError(format!("解析Accept-Encoding失败: {}", e)))?,
        );
        request.headers_mut().insert(
            "Accept-Language",
            "en-US,en;q=0.9".parse()
                .map_err(|e| ProviderError::NetworkError(format!("解析Accept-Language失败: {}", e)))?,
        );
        request.headers_mut().insert(
            "Cookie",
            format!("muid={};", muid).parse()
                .map_err(|e| ProviderError::NetworkError(format!("解析Cookie失败: {}", e)))?,
        );
        request.headers_mut().insert(
            "Sec-WebSocket-Version",
            "13".parse()
                .map_err(|e| ProviderError::NetworkError(format!("解析Sec-WebSocket-Version失败: {}", e)))?,
        );

        // 连接 WebSocket
        let (mut ws_stream, _) = connect_async(request)
            .await
            .map_err(|e| {
                log::error!("[EdgeTTS] WebSocket连接失败: {}", e);
                ProviderError::NetworkError(format!("WebSocket连接失败: {}", e))
            })?;
        log::info!("[EdgeTTS] WebSocket连接成功");

        // 发送配置消息
        let config_message = format!(
            "X-Timestamp:{}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n\
             {{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"true\",\"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"{}\"}}}}}}}}",
            timestamp, output_format
        );
        ws_stream
            .send(tungstenite::Message::Text(config_message.into()))
            .await
            .map_err(|e| {
                log::error!("[EdgeTTS] 发送配置消息失败: {}", e);
                ProviderError::NetworkError(format!("发送配置消息失败: {}", e))
            })?;
        log::info!("[EdgeTTS] 配置消息已发送");

        // 发送 SSML 消息
        let ssml_request_id = Uuid::new_v4().to_string();
        let ssml_message = format!(
            "X-RequestId:{}\r\nX-Timestamp:{}Z\r\nContent-Type:application/ssml+xml\r\nPath:ssml\r\n\r\n{}",
            ssml_request_id, timestamp, ssml
        );
        ws_stream
            .send(tungstenite::Message::Text(ssml_message.into()))
            .await
            .map_err(|e| {
                log::error!("[EdgeTTS] 发送SSML消息失败: {}", e);
                ProviderError::NetworkError(format!("发送SSML消息失败: {}", e))
            })?;
        log::info!("[EdgeTTS] SSML消息已发送，等待音频数据...");

        // 接收音频数据
        let mut audio_data = Vec::new();
        let mut message_count = 0;

        while let Some(message) = ws_stream.next().await {
            message_count += 1;
            match message {
                Ok(tungstenite::Message::Binary(data)) => {
                    log::debug!("[EdgeTTS] 收到二进制消息 #{}, 大小={}字节", message_count, data.len());
                    // Edge TTS 二进制消息格式（与 Python edge-tts 库一致）：
                    // [header_length: 2字节 BE uint16][header_data: (header_length-2)字节][\r\n: 2字节分隔符][音频数据]
                    // header_length 包含自身2字节，因此：
                    // data[:header_length] = header部分（含header_length字段）
                    // data[header_length+2:] = 音频数据（跳过header部分 + \r\n分隔符）
                    if data.len() < 4 {
                        log::warn!("[EdgeTTS] 二进制消息太短({}字节)，忽略", data.len());
                        continue;
                    }
                    let header_length = u16::from_be_bytes([data[0], data[1]]) as usize;
                    let audio_start = header_length + 2; // header_length 已包含自身2字节 + 2字节\r\n分隔符
                    if audio_start < data.len() {
                        let audio_chunk = &data[audio_start..];
                        audio_data.extend_from_slice(audio_chunk);
                        log::debug!("[EdgeTTS] 提取音频数据, header_length={}, 音频数据大小={}字节, 累计={}字节", header_length, audio_chunk.len(), audio_data.len());
                    } else if audio_start == data.len() {
                        log::debug!("[EdgeTTS] 二进制消息 header_length={} 无音频数据(仅header)", header_length);
                    } else {
                        log::warn!("[EdgeTTS] 二进制消息 header_length={} 但消息太短({}字节)，audio_start={}", header_length, data.len(), audio_start);
                    }
                }
                Ok(tungstenite::Message::Text(text)) => {
                    let text_str: &str = text.as_ref();
                    log::debug!("[EdgeTTS] 收到文本消息 #{}, 内容长度={}", message_count, text_str.len());
                    // 打印前300个字符用于调试（安全处理多字节字符边界）
                    let preview = if text_str.len() > 300 {
                        let safe_len = text_str.char_indices()
                            .take(300)
                            .last()
                            .map(|(idx, _)| idx)
                            .unwrap_or(300);
                        format!("{}...(共{}字符)", &text_str[..safe_len], text_str.chars().count())
                    } else {
                        text_str.to_string()
                    };
                    log::debug!("[EdgeTTS] 文本消息内容: {}", preview);
                    // 检查是否是结束标记
                    if text_str.contains("Path:turn.end") || text_str.contains("Path:Turn.end") {
                        log::info!("[EdgeTTS] 收到结束标记 (turn.end)");
                        break;
                    }
                    // 检查是否是错误响应
                    if text_str.contains("Path:turn.start") || text_str.contains("Path:Turn.start") {
                        log::debug!("[EdgeTTS] 收到开始标记 (turn.start)，继续接收音频");
                        continue;
                    }
                    // 检查是否包含错误信息
                    if text_str.contains("X-RequestId") && text_str.contains("Path:response") {
                        log::warn!("[EdgeTTS] 收到response消息，可能是错误响应");
                    }
                }
                Ok(msg_type) => {
                    log::warn!("[EdgeTTS] 收到未知消息类型: {:?}", msg_type);
                }
                Err(e) => {
                    log::error!("[EdgeTTS] WebSocket错误: {}", e);
                    return Err(ProviderError::NetworkError(format!(
                        "WebSocket错误: {}",
                        e
                    )));
                }
            }
        }
        
        log::info!("[EdgeTTS] 消息接收完成: 收到{}条消息, 音频数据大小={}字节", message_count, audio_data.len());

        if audio_data.is_empty() {
            log::error!("[EdgeTTS] 未收到任何音频数据");
            return Err(ProviderError::GenerationFailed(
                "未收到任何音频数据".to_string(),
            ));
        }

        log::info!("[EdgeTTS] 语音合成完成: size={}KB", audio_data.len() / 1024);
        Ok(audio_data)
    }

    /// 从 asset_ref 的 style 字段推断语音角色
    fn infer_voice_from_style(&self, style: Option<&str>) -> ChineseVoice {
        match style {
            Some(s) => {
                let lower = s.to_lowercase();
                if lower.contains("xiaoxiao") || lower.contains("温柔") || lower.contains("gentle") {
                    ChineseVoice::XiaoxiaoNeural
                } else if lower.contains("xiaoyi") || lower.contains("活泼") || lower.contains("lively") {
                    ChineseVoice::XiaoyiNeural
                } else if lower.contains("yunjian") || lower.contains("沉稳") || lower.contains("steady") {
                    ChineseVoice::YunjianNeural
                } else if lower.contains("yunxi") || lower.contains("阳光") || lower.contains("cheerful") {
                    ChineseVoice::YunxiNeural
                } else if lower.contains("yunxia") || lower.contains("少年") {
                    ChineseVoice::YunxiaNeural
                } else if lower.contains("yunyang") || lower.contains("新闻") || lower.contains("narrator") {
                    ChineseVoice::YunyangNeural
                } else if lower.contains("xiaobei") || lower.contains("东北") {
                    ChineseVoice::XiaobeiNeural
                } else if lower.contains("xiaoni") || lower.contains("陕西") {
                    ChineseVoice::XiaoniNeural
                } else if lower.contains("male") || lower.contains("男") {
                    ChineseVoice::YunjianNeural
                } else if lower.contains("female") || lower.contains("女") {
                    ChineseVoice::XiaoxiaoNeural
                } else {
                    self.default_voice.clone()
                }
            }
            None => self.default_voice.clone(),
        }
    }

    fn generate_cache_key(asset_ref: &AssetRef) -> String {
        let mut hasher = Sha256::new();
        hasher.update(asset_ref.id.as_bytes());
        hasher.update(asset_ref.prompt.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }
}

#[async_trait]
impl IAssetProvider for EdgeTTSProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        let text = &asset_ref.prompt;
        let voice = self.infer_voice_from_style(asset_ref.style.as_deref());
        let params = TtsQualityParams::from_model_advanced_params(&self.config);

        let voice_id = if !params.voice.is_empty() { &params.voice } else { voice.as_str() };
        let audio_data = self.synthesize(text, voice_id, &params).await?;

        let cache_key = asset_ref
            .cache_key
            .clone()
            .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

        let dest_dir = self.asset_base_path.join("cacheAssets").join(&cache_key);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| ProviderError::GenerationFailed(format!("创建资源目录失败: {}", e)))?;

        let dest_path = dest_dir.join(format!("{}.mp3", asset_ref.id));
        std::fs::write(&dest_path, &audio_data)
            .map_err(|e| ProviderError::GenerationFailed(format!("写入音频文件失败: {}", e)))?;

        Ok(LocalAsset {
            id: asset_ref.id.clone(),
            asset_type: AssetType::Voice,
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
        self.check_connectivity_with_prompt("测试").await
    }

    async fn check_connectivity_with_prompt(&self, prompt: &str) -> Result<ConnectivityCheck, ProviderError> {
        let start = SystemTime::now();

        // 实际合成音频验证连通性
        let params = TtsQualityParams::from_model_advanced_params(&self.config);
        let voice_id = if !params.voice.is_empty() {
            &params.voice
        } else {
            self.default_voice.as_str()
        };
        let request_body = format!(
            "voice={}, text={}, rate={}, pitch={}, volume={}, output_format={}",
            voice_id, prompt, params.rate, params.pitch, params.volume, params.output_format
        );
        let result = self.synthesize(prompt, voice_id, &params).await;

        let latency = SystemTime::now()
            .duration_since(start)
            .unwrap_or_default()
            .as_millis() as u64;

        match result {
            Ok(audio_data) => {
                // 保存测试音频到 gen/cache/ 目录
                let media_url = self.save_test_audio(&audio_data, "edge_tts").ok();
                Ok(ConnectivityCheck {
                    provider_id: self.config.id.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    status: ConnectivityStatus::Ok,
                    latency: Some(latency),
                    error_message: None,
                    quota_info: None,
                    response_preview: Some(format!("合成成功，音频大小: {}KB", audio_data.len() / 1024)),
                    test_prompt: Some(prompt.to_string()),
                    media_url,
                    media_type: Some("audio".to_string()),
                    polling_task_id: None,
                    polling_status: None,
                    polling_elapsed_secs: None,
                    media_items: None,
                    request_endpoint: Some("wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1".to_string()),
                    request_model: Some(voice_id.to_string()),
                    request_headers: Some("WebSocket (无需请求头)".to_string()),
                    request_body: Some(truncate_str(&request_body, 2000).to_string()),
                    response_status: Some(200),
                })
            }
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
                polling_task_id: None,
                polling_status: None,
                polling_elapsed_secs: None,
                media_items: None,
                request_endpoint: Some("wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1".to_string()),
                request_model: Some(self.default_voice.as_str().to_string()),
                request_headers: Some("WebSocket (无需请求头)".to_string()),
                request_body: Some(truncate_str(&request_body, 2000).to_string()),
                response_status: None,
            }),
        }
    }

    fn supported_modalities(&self) -> Vec<AIModality> {
        vec![AIModality::Voice]
    }

    fn provider_id(&self) -> &str {
        &self.config.id
    }
}

impl EdgeTTSProvider {
    /// 保存测试音频到 gen/cache/ 目录，返回本地文件路径
    fn save_test_audio(&self, audio_data: &[u8], provider_name: &str) -> Result<String, ProviderError> {
        let cache_dir = self.asset_base_path.join("cache");
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| ProviderError::GenerationFailed(format!("创建缓存目录失败: {}", e)))?;

        let filename = format!("{}_test_{}.mp3", provider_name,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs());
        let dest_path = cache_dir.join(&filename);
        std::fs::write(&dest_path, audio_data)
            .map_err(|e| ProviderError::GenerationFailed(format!("写入测试音频失败: {}", e)))?;

        Ok(dest_path.to_string_lossy().to_string())
    }
}
