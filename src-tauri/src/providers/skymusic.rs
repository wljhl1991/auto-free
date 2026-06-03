use async_trait::async_trait;
use super::{IAssetProvider, ProviderError};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_ENDPOINT: &str = "https://api.tiangong.cn/v1/music/generations";
const DEFAULT_MODEL: &str = "skymusic-v1";
const POLL_INTERVAL_SECS: u64 = 5;
const MAX_POLL_DURATION_SECS: u64 = 300; // 5 minutes

#[derive(Debug, Serialize)]
struct MusicGenerationRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MusicGenerationResponse {
    task_id: String,
    status: String,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MusicTaskStatusResponse {
    task_id: String,
    status: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    error: Option<ApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: Option<String>,
}

pub struct SkyMusicProvider {
    config: AIProviderConfig,
    client: Client,
    api_key: String,
    default_model: String,
    endpoint: String,
    asset_base_path: PathBuf,
}

impl SkyMusicProvider {
    pub fn new(config: &AIProviderConfig, asset_base_path: PathBuf) -> Result<Self, ProviderError> {
        let api_key = config.auth_config.api_key
            .as_ref()
            .map(|k| k.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("SkyMusic API Key not configured".to_string()))?;

        let default_model = config.models
            .iter()
            .find(|m| m.is_default && m.modality == AIModality::Music)
            .map(|m| m.id.clone())
            .or_else(|| config.models.iter().find(|m| m.modality == AIModality::Music).map(|m| m.id.clone()))
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
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config: config.clone(),
            client,
            api_key,
            default_model,
            endpoint,
            asset_base_path,
        })
    }

    /// 生成音乐
    pub async fn generate_music(
        &self,
        prompt: &str,
        duration: Option<u32>,
        style: Option<&str>,
    ) -> Result<Vec<u8>, ProviderError> {
        let request = MusicGenerationRequest {
            model: self.default_model.clone(),
            prompt: prompt.to_string(),
            duration,
            style: style.map(|s| s.to_string()),
        };

        let response = self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProviderError::Timeout(format!("Request timeout: {}", e))
                } else {
                    ProviderError::NetworkError(format!("Network error: {}", e))
                }
            })?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| ProviderError::NetworkError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(self.handle_error_status(status.as_u16(), &body));
        }

        let gen_response: MusicGenerationResponse = serde_json::from_str(&body)
            .map_err(|e| ProviderError::GenerationFailed(format!("Failed to parse response: {}", e)))?;

        let task_id = gen_response.task_id;

        // 轮询等待生成完成
        let audio_url = self.poll_until_complete(&task_id).await?;

        // 下载音频
        self.download_audio(&audio_url).await
    }

    /// 异步轮询等待生成完成
    async fn poll_until_complete(&self, task_id: &str) -> Result<String, ProviderError> {
        let poll_url = format!("{}/{}", self.endpoint.trim_end_matches('/'), task_id);
        let start = SystemTime::now();

        loop {
            let elapsed = SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_secs();

            if elapsed >= MAX_POLL_DURATION_SECS {
                return Err(ProviderError::Timeout(format!(
                    "Music generation timed out after {} seconds for task {}",
                    MAX_POLL_DURATION_SECS, task_id
                )));
            }

            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;

            let response = self.client
                .get(&poll_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send()
                .await
                .map_err(|e| ProviderError::NetworkError(format!("Poll request failed: {}", e)))?;

            let status = response.status();
            let body = response.text().await
                .map_err(|e| ProviderError::NetworkError(format!("Failed to read poll response: {}", e)))?;

            if !status.is_success() {
                return Err(self.handle_error_status(status.as_u16(), &body));
            }

            let task_status: MusicTaskStatusResponse = serde_json::from_str(&body)
                .map_err(|e| ProviderError::GenerationFailed(format!("Failed to parse poll response: {}", e)))?;

            match task_status.status.as_str() {
                "completed" => {
                    return task_status.url
                        .ok_or_else(|| ProviderError::GenerationFailed(
                            "Music generation completed but no audio URL returned".to_string()
                        ));
                }
                "failed" => {
                    return Err(ProviderError::GenerationFailed(
                        task_status.error
                            .unwrap_or_else(|| "Music generation failed with unknown error".to_string())
                    ));
                }
                "pending" | "processing" => {
                    // 继续轮询
                    continue;
                }
                _ => {
                    // 未知状态，继续轮询
                    continue;
                }
            }
        }
    }

    /// 下载音频文件
    async fn download_audio(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(format!("Failed to download audio: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ProviderError::NetworkError(format!(
                "Audio download failed with status: {}", status
            )));
        }

        response.bytes().await
            .map(|b| b.to_vec())
            .map_err(|e| ProviderError::NetworkError(format!("Failed to read audio data: {}", e)))
    }

    /// 构造 BGM Prompt
    /// {gameType} background music, {mood}, {tempo}, {instruments}, loopable, no vocals
    pub fn build_music_prompt(&self, asset_ref: &AssetRef) -> String {
        let mut parts = Vec::new();

        // 从 prompt 提取关键词
        let base_prompt = asset_ref.prompt.trim();
        if !base_prompt.is_empty() {
            parts.push(base_prompt.to_string());
        }

        // 添加风格描述
        if let Some(style) = asset_ref.style.as_deref() {
            let style = style.trim();
            if !style.is_empty() {
                parts.push(style.to_string());
            }
        }

        // 根据游戏类型添加风格描述
        let game_type_style = self.infer_style_from_id(&asset_ref.id);
        if !game_type_style.is_empty() {
            parts.push(game_type_style);
        }

        // 添加 BGM 通用后缀
        parts.push("background music".to_string());
        parts.push("loopable".to_string());
        parts.push("no vocals".to_string());

        parts.join(", ")
    }

    /// 从 prompt 推断音乐风格
    fn infer_style(&self, prompt: &str) -> String {
        let lower = prompt.to_lowercase();

        if lower.contains("horror") || lower.contains("恐怖") || lower.contains("scary") || lower.contains("creepy") {
            "dark ambient".to_string()
        } else if lower.contains("romance") || lower.contains("浪漫") || lower.contains("love") || lower.contains("tender") {
            "romantic".to_string()
        } else if lower.contains("battle") || lower.contains("战斗") || lower.contains("fight") || lower.contains("epic") {
            "epic orchestral".to_string()
        } else if lower.contains("mystery") || lower.contains("悬疑") || lower.contains("suspense") {
            "mysterious ambient".to_string()
        } else if lower.contains("happy") || lower.contains("欢快") || lower.contains("cheerful") || lower.contains("joyful") {
            "upbeat".to_string()
        } else if lower.contains("sad") || lower.contains("悲伤") || lower.contains("melancholy") {
            "melancholic".to_string()
        } else if lower.contains("fantasy") || lower.contains("奇幻") || lower.contains("magical") {
            "fantasy orchestral".to_string()
        } else if lower.contains("rpg") || lower.contains("冒险") || lower.contains("adventure") {
            "adventure".to_string()
        } else if lower.contains("visual novel") || lower.contains("视觉小说") || lower.contains("visualnovel") {
            "cinematic ambient".to_string()
        } else if lower.contains("simulation") || lower.contains("模拟") || lower.contains("sim") {
            "relaxing".to_string()
        } else {
            "ambient".to_string()
        }
    }

    /// 从 asset_ref.id 推断游戏类型风格
    fn infer_style_from_id(&self, id: &str) -> String {
        let lower = id.to_lowercase();
        if lower.contains("horror") || lower.contains("恐怖") {
            "dark atmospheric".to_string()
        } else if lower.contains("rpg") || lower.contains("冒险") {
            "adventure orchestral".to_string()
        } else if lower.contains("mystery") || lower.contains("悬疑") {
            "suspenseful".to_string()
        } else if lower.contains("romance") || lower.contains("浪漫") {
            "romantic piano".to_string()
        } else {
            String::new()
        }
    }

    fn handle_error_status(&self, status_code: u16, body: &str) -> ProviderError {
        let error_msg = serde_json::from_str::<ApiError>(body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or_else(|| body.to_string());

        match status_code {
            401 | 403 => ProviderError::AuthFailed(format!("Authentication failed: {}", error_msg)),
            429 => ProviderError::QuotaExceeded(format!("Rate limited: {}", error_msg)),
            s if s >= 500 => ProviderError::GenerationFailed(format!("Server error ({}): {}", status_code, error_msg)),
            _ => ProviderError::GenerationFailed(format!("API error ({}): {}", status_code, error_msg)),
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
impl IAssetProvider for SkyMusicProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        let prompt = self.build_music_prompt(asset_ref);
        let style = self.infer_style(&prompt);
        let duration = asset_ref.style.as_deref()
            .and_then(|s| s.split(',').find(|p| p.trim().starts_with("duration:")))
            .and_then(|p| p.trim().strip_prefix("duration:"))
            .and_then(|v| v.trim().parse::<u32>().ok())
            .or(Some(30)); // 默认 30 秒

        let audio_data = self.generate_music(&prompt, duration, Some(&style)).await?;

        let cache_key = asset_ref.cache_key.clone()
            .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

        let dest_dir = self.asset_base_path.join(&cache_key);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| ProviderError::GenerationFailed(format!("Failed to create asset dir: {}", e)))?;

        let dest_path = dest_dir.join(format!("{}.mp3", asset_ref.id));
        std::fs::write(&dest_path, &audio_data)
            .map_err(|e| ProviderError::GenerationFailed(format!("Failed to write audio file: {}", e)))?;

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
        let start = SystemTime::now();

        // 尝试发起一个轻量级请求验证连通性
        // 天工音乐 API 没有专门的余额查询接口，使用模型列表或简单请求验证
        let result = self.client
            .get(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await;

        let latency = SystemTime::now()
            .duration_since(start)
            .unwrap_or_default()
            .as_millis() as u64;

        match result {
            Ok(response) => {
                let status = response.status();
                if status.is_success() || status.as_u16() == 404 {
                    // 404 也说明 API 可达，只是路径不对（GET list 可能不支持）
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
                    })
                } else if status.as_u16() == 401 || status.as_u16() == 403 {
                    Ok(ConnectivityCheck {
                        provider_id: self.config.id.clone(),
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        status: ConnectivityStatus::AuthFailed,
                        latency: Some(latency),
                        error_message: Some("Authentication failed".to_string()),
                        quota_info: None,
                    })
                } else if status.as_u16() == 429 {
                    Ok(ConnectivityCheck {
                        provider_id: self.config.id.clone(),
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        status: ConnectivityStatus::QuotaExceeded,
                        latency: Some(latency),
                        error_message: Some("Rate limited".to_string()),
                        quota_info: None,
                    })
                } else {
                    Ok(ConnectivityCheck {
                        provider_id: self.config.id.clone(),
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        status: ConnectivityStatus::UnknownError,
                        latency: Some(latency),
                        error_message: Some(format!("Unexpected status: {}", status)),
                        quota_info: None,
                    })
                }
            }
            Err(e) => {
                Ok(ConnectivityCheck {
                    provider_id: self.config.id.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    status: ConnectivityStatus::NetworkError,
                    latency: Some(latency),
                    error_message: Some(format!("Network error: {}", e)),
                    quota_info: None,
                })
            }
        }
    }

    fn supported_modalities(&self) -> Vec<AIModality> {
        vec![AIModality::Music]
    }

    fn provider_id(&self) -> &str {
        &self.config.id
    }
}
