use async_trait::async_trait;
use super::{IAssetProvider, ProviderError};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite};
use uuid::Uuid;

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

const EDGE_TTS_WS_URL: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken=6A5AA1D4EAFF4E9FB37E23D68491D6F4&ConnectionId=";

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
        voice: Option<&ChineseVoice>,
        rate: Option<&str>,
        pitch: Option<&str>,
    ) -> Result<Vec<u8>, ProviderError> {
        let voice = voice.unwrap_or(&self.default_voice);
        let rate = rate.unwrap_or("+0%");
        let pitch = pitch.unwrap_or("+0Hz");
        let ssml = self.build_ssml(text, voice, rate, pitch);
        self.connect_and_synthesize(&ssml, voice).await
    }

    /// 根据 NPC 性别和性格选择语音
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
    fn build_ssml(&self, text: &str, voice: &ChineseVoice, rate: &str, pitch: &str) -> String {
        format!(
            "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='zh-CN'>\
             <voice name='{}'>\
             <prosody pitch='{}' rate='{}'>\
             {}\
             </prosody>\
             </voice>\
             </speak>",
            voice.as_str(),
            pitch,
            rate,
            text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        )
    }

    /// 通过 WebSocket 连接 Edge TTS 并获取音频
    async fn connect_and_synthesize(
        &self,
        ssml: &str,
        voice: &ChineseVoice,
    ) -> Result<Vec<u8>, ProviderError> {
        let request_id = Uuid::new_v4().to_string();
        let connection_id = Uuid::new_v4().to_string();

        let ws_url = format!("{}{}", EDGE_TTS_WS_URL, connection_id);
        log::info!("[EdgeTTS] 连接WebSocket: voice={}, text_len={}", voice.as_str(), ssml.len());

        // 连接 WebSocket
        let (mut ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| {
                log::error!("[EdgeTTS] WebSocket连接失败: {}", e);
                ProviderError::NetworkError(format!("WebSocket连接失败: {}", e))
            })?;
        log::info!("[EdgeTTS] WebSocket连接成功");

        // 发送配置消息
        let config_message = format!(
            "X-RequestId:{}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n\
             {{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"true\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}",
            request_id
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
            "X-RequestId:{}\r\nContent-Type:application/ssml+xml\r\nPath:ssml\r\n\r\n{}",
            ssml_request_id, ssml
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

        while let Some(message) = ws_stream.next().await {
            match message {
                Ok(tungstenite::Message::Binary(data)) => {
                    // 二进制消息：以 header + \r\n\r\n + 音频数据 格式
                    if let Some(separator_pos) = find_separator(&data) {
                        let audio_chunk = &data[separator_pos + 4..];
                        if !audio_chunk.is_empty() {
                            audio_data.extend_from_slice(audio_chunk);
                        }
                    }
                }
                Ok(tungstenite::Message::Text(text)) => {
                    let text_str: &str = text.as_ref();
                    // 检查是否是结束标记
                    if text_str.contains("Path:turn.end") {
                        log::info!("[EdgeTTS] 收到结束标记");
                        break;
                    }
                    // 检查是否是错误响应
                    if text_str.contains("Path:turn.start") {
                        log::debug!("[EdgeTTS] 收到开始标记，继续接收音频");
                        continue;
                    }
                }
                Ok(_) => continue,
                Err(e) => {
                    log::error!("[EdgeTTS] WebSocket错误: {}", e);
                    return Err(ProviderError::NetworkError(format!(
                        "WebSocket错误: {}", e
                    )));
                }
            }
        }

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

/// 在二进制数据中查找 \r\n\r\n 分隔符的位置
fn find_separator(data: &[u8]) -> Option<usize> {
    let pattern = b"\r\n\r\n";
    if data.len() < pattern.len() {
        return None;
    }
    for i in 0..=data.len() - pattern.len() {
        if &data[i..i + pattern.len()] == pattern {
            return Some(i);
        }
    }
    None
}

#[async_trait]
impl IAssetProvider for EdgeTTSProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        let text = &asset_ref.prompt;
        let voice = self.infer_voice_from_style(asset_ref.style.as_deref());

        let audio_data = self.synthesize(text, Some(&voice), None, None).await?;

        let cache_key = asset_ref
            .cache_key
            .clone()
            .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

        let dest_dir = self.asset_base_path.join(&cache_key);
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
        let start = SystemTime::now();

        // 尝试合成一个短文本验证连通性
        let result = self.synthesize("测试", None, None, None).await;

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
                    response_preview: None,
                    test_prompt: Some("测试".to_string()),
                    media_url,
                    media_type: Some("audio".to_string()),
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
                test_prompt: Some("测试".to_string()),
                media_url: None,
                media_type: None,
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
