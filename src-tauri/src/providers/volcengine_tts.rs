use async_trait::async_trait;
use super::{truncate_str, IAssetProvider, ProviderError};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_ENDPOINT: &str = "https://openspeech.bytedance.com/api/v1/tts";
const DEFAULT_MODEL: &str = "volcengine-tts";

#[derive(Debug, Serialize)]
struct TTSRequest {
    app: TTSApp,
    user: TTSUser,
    audio: TTSAudio,
    request: TTSRequestInfo,
}

#[derive(Debug, Serialize)]
struct TTSApp {
    appid: String,
    token: String,
}

#[derive(Debug, Serialize)]
struct TTSUser {
    uid: String,
}

#[derive(Debug, Serialize)]
struct TTSAudio {
    voice_type: String,
    encoding: String,
    speed_ratio: f32,
}

#[derive(Debug, Serialize)]
struct TTSRequestInfo {
    reqid: String,
    text: String,
    text_type: String,
    operation: String,
}

#[derive(Debug, Deserialize)]
struct TTSResponse {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<String>, // base64 encoded audio
}

#[derive(Debug, Deserialize)]
struct ApiError {
    error: Option<ApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: Option<String>,
}

pub struct VolcengineTTSProvider {
    config: AIProviderConfig,
    client: Client,
    appid: String,
    access_token: String,
    default_voice: String,
    endpoint: String,
    asset_base_path: PathBuf,
}

impl VolcengineTTSProvider {
    pub fn new(config: &AIProviderConfig, asset_base_path: &std::path::Path) -> Result<Self, ProviderError> {
        let appid = config.auth_config.extra_params
            .as_ref()
            .and_then(|p| p.get("appid"))
            .map(|f| f.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("火山引擎 TTS appid 未配置".to_string()))?;

        let access_token = config.auth_config.extra_params
            .as_ref()
            .and_then(|p| p.get("access_token"))
            .map(|f| f.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("火山引擎 TTS access_token 未配置".to_string()))?;

        let default_voice = config.models
            .iter()
            .find(|m| m.is_default)
            .map(|m| m.id.clone())
            .or_else(|| config.models.first().map(|m| m.id.clone()))
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());

        let endpoint = config.models
            .iter()
            .find(|m| m.is_default)
            .and_then(|m| {
                let ep = m.endpoint.trim().to_string();
                if ep.is_empty() { None } else { Some(ep) }
            })
            .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string());

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| ProviderError::NetworkError(format!("创建HTTP客户端失败: {}", e)))?;

        Ok(Self {
            config: config.clone(),
            client,
            appid,
            access_token,
            default_voice,
            endpoint,
            asset_base_path: asset_base_path.to_path_buf(),
        })
    }

    pub async fn synthesize(
        &self,
        text: &str,
        voice_type: Option<&str>,
        speed: Option<f32>,
    ) -> Result<Vec<u8>, ProviderError> {
        let voice_type = voice_type.unwrap_or(&self.default_voice);
        let speed = speed.unwrap_or(1.0);

        log::info!("[VolcengineTTS] 请求: endpoint={}, voice_type={}, text_len={}, speed={}", 
            self.endpoint, voice_type, text.len(), speed);

        let request = TTSRequest {
            app: TTSApp {
                appid: self.appid.clone(),
                token: self.access_token.clone(),
            },
            user: TTSUser {
                uid: "autofree_user".to_string(),
            },
            audio: TTSAudio {
                voice_type: voice_type.to_string(),
                encoding: "mp3".to_string(),
                speed_ratio: speed,
            },
            request: TTSRequestInfo {
                reqid: uuid::Uuid::new_v4().to_string(),
                text: text.to_string(),
                text_type: "plain".to_string(),
                operation: "query".to_string(),
            },
        };

        let response = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[VolcengineTTS] 请求发送失败: {} (is_timeout={}, is_connect={}, is_request={})", 
                    e, e.is_timeout(), e.is_connect(), e.is_request());
                if e.is_timeout() {
                    ProviderError::Timeout(format!("请求超时: {}", e))
                } else if e.is_connect() {
                    ProviderError::NetworkError(format!("连接失败: {} (请检查网络或API地址是否正确)", e))
                } else {
                    ProviderError::NetworkError(format!("网络错误: {}", e))
                }
            })?;

        let status = response.status();
        log::info!("[VolcengineTTS] 响应状态: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

        let body = response.text().await
            .map_err(|e| {
                log::error!("[VolcengineTTS] 读取响应体失败: {}", e);
                ProviderError::NetworkError(format!("读取响应失败: {}", e))
            })?;

        let truncated_response = if body.len() > 1000 {
            format!("{}...(共{}字符)", truncate_str(&body, 1000), body.len())
        } else {
            body.clone()
        };
        log::info!("[VolcengineTTS] 响应体: {}", truncated_response);

        if status.is_success() {
            let tts_response: TTSResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[VolcengineTTS] 解析响应JSON失败: {} (原始响应前200字: {})", e, truncate_str(&body, 200));
                    ProviderError::GenerationFailed(format!("解析响应失败: {}", e))
                })?;

            if tts_response.code != 0 {
                log::error!("[VolcengineTTS] TTS API错误: code={}, message={}", tts_response.code, tts_response.message);
                return Err(ProviderError::GenerationFailed(format!(
                    "TTS API错误 (code={}): {}", tts_response.code, tts_response.message
                )));
            }

            let audio_base64 = tts_response.data
                .ok_or_else(|| {
                    log::error!("[VolcengineTTS] 响应中没有音频数据");
                    ProviderError::GenerationFailed("响应中没有音频数据".to_string())
                })?;

            let audio_data = base64::decode(&audio_base64)
                .map_err(|e| {
                    log::error!("[VolcengineTTS] 解码音频数据失败: {}", e);
                    ProviderError::GenerationFailed(format!("解码音频数据失败: {}", e))
                })?;

            log::info!("[VolcengineTTS] 语音合成完成: size={}KB", audio_data.len() / 1024);
            Ok(audio_data)
        } else {
            self.handle_error_status(status.as_u16(), &body)
        }
    }

    fn infer_voice_from_style(&self, style: Option<&str>) -> String {
        match style {
            Some(s) => {
                let lower = s.to_lowercase();
                if lower.contains("female") || lower.contains("女") {
                    "zh_female_tianmei".to_string()
                } else if lower.contains("male") || lower.contains("男") {
                    "zh_male_chunhou".to_string()
                } else {
                    self.default_voice.clone()
                }
            }
            None => self.default_voice.clone(),
        }
    }

    fn handle_error_status(&self, status_code: u16, body: &str) -> Result<Vec<u8>, ProviderError> {
        let error_msg = serde_json::from_str::<ApiError>(body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or_else(|| body.to_string());

        log::error!("[VolcengineTTS] API错误: status={}, message={}", status_code, error_msg);

        match status_code {
            401 | 403 => Err(ProviderError::AuthFailed(format!("认证失败: {}", error_msg))),
            429 => Err(ProviderError::QuotaExceeded(format!("请求频率超限: {}", error_msg))),
            s if s >= 500 => Err(ProviderError::GenerationFailed(format!("服务器错误 ({}): {}", status_code, error_msg))),
            _ => Err(ProviderError::GenerationFailed(format!("API错误 ({}): {}", status_code, error_msg))),
        }
    }

    fn generate_cache_key(asset_ref: &AssetRef) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(asset_ref.id.as_bytes());
        hasher.update(asset_ref.prompt.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }
}

#[async_trait]
impl IAssetProvider for VolcengineTTSProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        let text = &asset_ref.prompt;
        let voice_type = self.infer_voice_from_style(asset_ref.style.as_deref());

        let audio_data = self.synthesize(text, Some(&voice_type), None).await?;

        let cache_key = asset_ref.cache_key.clone()
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

        let result = self.synthesize("测试", None, None).await;

        let latency = SystemTime::now()
            .duration_since(start)
            .unwrap_or_default()
            .as_millis() as u64;

        match result {
            Ok(_) => Ok(ConnectivityCheck {
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
                test_prompt: None,
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
                test_prompt: None,
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
                test_prompt: None,
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
                test_prompt: None,
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
