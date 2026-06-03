use async_trait::async_trait;
use super::{IAssetProvider, ProviderError};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_RETRIES: u32 = 3;
const DEFAULT_ENDPOINT: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";
const DEFAULT_MODEL: &str = "glm-4-flash";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    error: Option<ApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: Option<String>,
}

pub struct ZhipuProvider {
    config: AIProviderConfig,
    client: Client,
    default_model: String,
    api_key: String,
    endpoint: String,
    asset_base_path: PathBuf,
}

impl ZhipuProvider {
    pub fn new(config: &AIProviderConfig, asset_base_path: &std::path::Path) -> Result<Self, ProviderError> {
        let api_key = config.auth_config.api_key
            .as_ref()
            .map(|k| k.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("Zhipu API Key not configured".to_string()))?;

        let default_model = config.models
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
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config: config.clone(),
            client,
            default_model,
            api_key,
            endpoint,
            asset_base_path: asset_base_path.to_path_buf(),
        })
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>, model: Option<&str>) -> Result<String, ProviderError> {
        let model = model.unwrap_or(&self.default_model);
        let max_tokens = self.config.models
            .iter()
            .find(|m| m.id == model)
            .and_then(|m| m.max_tokens);

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            max_tokens,
            temperature: Some(0.7),
            stream: Some(false),
        };

        let mut last_error = None;
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
            }

            match self.send_request(&request).await {
                Ok(text) => return Ok(text),
                Err(e) => {
                    let should_retry = matches!(
                        &e,
                        ProviderError::NetworkError(_) | ProviderError::Timeout(_)
                    ) || matches!(&e, ProviderError::GenerationFailed(msg) if msg.contains("5"));
                    if should_retry && attempt < MAX_RETRIES {
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| ProviderError::NetworkError("Unknown error after retries".to_string())))
    }

    pub async fn chat_stream(&self, messages: Vec<ChatMessage>, model: Option<&str>) -> Result<reqwest::Response, ProviderError> {
        let model = model.unwrap_or(&self.default_model);
        let max_tokens = self.config.models
            .iter()
            .find(|m| m.id == model)
            .and_then(|m| m.max_tokens);

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            max_tokens,
            temperature: Some(0.7),
            stream: Some(true),
        };

        let mut last_error = None;
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
            }

            match self.send_stream_request(&request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    let should_retry = matches!(
                        &e,
                        ProviderError::NetworkError(_) | ProviderError::Timeout(_)
                    ) || matches!(&e, ProviderError::GenerationFailed(msg) if msg.contains("5"));
                    if should_retry && attempt < MAX_RETRIES {
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| ProviderError::NetworkError("Unknown error after retries".to_string())))
    }

    async fn send_request(&self, request: &ChatRequest) -> Result<String, ProviderError> {
        let response = self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
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

        if status.is_success() {
            let chat_response: ChatResponse = serde_json::from_str(&body)
                .map_err(|e| ProviderError::GenerationFailed(format!("Failed to parse response: {}", e)))?;
            chat_response.choices.first()
                .map(|c| c.message.content.clone())
                .ok_or_else(|| ProviderError::GenerationFailed("No choices in response".to_string()))
        } else {
            self.handle_error_status(status.as_u16(), &body)
        }
    }

    async fn send_stream_request(&self, request: &ChatRequest) -> Result<reqwest::Response, ProviderError> {
        let response = self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
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
        if status.is_success() {
            Ok(response)
        } else {
            let body = response.text().await
                .map_err(|e| ProviderError::NetworkError(format!("Failed to read error response: {}", e)))?;
            Err(self.handle_error_status(status.as_u16(), &body).unwrap_err())
        }
    }

    fn handle_error_status(&self, status_code: u16, body: &str) -> Result<String, ProviderError> {
        let error_msg = serde_json::from_str::<ApiError>(body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or_else(|| body.to_string());

        match status_code {
            401 | 403 => Err(ProviderError::AuthFailed(format!("Authentication failed: {}", error_msg))),
            429 => Err(ProviderError::QuotaExceeded(format!("Rate limited: {}", error_msg))),
            s if s >= 500 => Err(ProviderError::GenerationFailed(format!("Server error ({}): {}", status_code, error_msg))),
            _ => Err(ProviderError::GenerationFailed(format!("API error ({}): {}", status_code, error_msg))),
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
impl IAssetProvider for ZhipuProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: asset_ref.prompt.clone(),
            },
        ];

        let text = self.chat(messages, None).await?;

        let cache_key = asset_ref.cache_key.clone()
            .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

        let dest_dir = self.asset_base_path.join(&cache_key);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| ProviderError::GenerationFailed(format!("Failed to create asset dir: {}", e)))?;

        let dest_path = dest_dir.join(format!("{}.txt", asset_ref.id));
        std::fs::write(&dest_path, &text)
            .map_err(|e| ProviderError::GenerationFailed(format!("Failed to write text file: {}", e)))?;

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

        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            },
        ];

        let result = self.chat(messages, None).await;
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
