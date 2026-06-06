use async_trait::async_trait;
use super::{truncate_str, save_raw_response, IAssetProvider, ProviderError};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_ENDPOINT: &str = "https://api.vidu.ai/v1/videos/generations";
const DEFAULT_MODEL: &str = "vidu-1.5";
const POLL_INTERVAL_SECS: u64 = 10;
const MAX_POLL_DURATION_SECS: u64 = 600;

#[derive(Debug, Serialize)]
struct VideoGenerationRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aspect_ratio: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VideoGenerationResponse {
    #[allow(dead_code)]
    task_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    status: String,
    #[serde(default)]
    #[allow(dead_code)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VideoTaskStatusResponse {
    #[allow(dead_code)]
    task_id: String,
    #[serde(default)]
    #[allow(dead_code)]
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

pub struct ViduProvider {
    config: AIProviderConfig,
    client: Client,
    api_key: String,
    default_model: String,
    endpoint: String,
    asset_base_path: PathBuf,
}

impl ViduProvider {
    pub fn new(config: &AIProviderConfig, asset_base_path: PathBuf) -> Result<Self, ProviderError> {
        let api_key = config.auth_config.api_key
            .as_ref()
            .map(|k| k.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("Vidu API Key 未配置".to_string()))?;

        let default_model = config.models
            .iter()
            .find(|m| m.is_default && m.modality == AIModality::Video)
            .map(|m| m.id.clone())
            .or_else(|| config.models.iter().find(|m| m.modality == AIModality::Video).map(|m| m.id.clone()))
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
            .map_err(|e| ProviderError::NetworkError(format!("创建HTTP客户端失败: {}", e)))?;

        Ok(Self {
            config: config.clone(),
            client,
            api_key,
            default_model,
            endpoint,
            asset_base_path,
        })
    }

    pub async fn generate_video(
        &self,
        prompt: &str,
        negative_prompt: Option<&str>,
        duration: Option<&str>,
        aspect_ratio: Option<&str>,
    ) -> Result<Vec<u8>, ProviderError> {
        let request = VideoGenerationRequest {
            model: self.default_model.clone(),
            prompt: prompt.to_string(),
            negative_prompt: negative_prompt.map(|s| s.to_string()),
            duration: duration.map(|s| s.to_string()),
            aspect_ratio: aspect_ratio.map(|s| s.to_string()),
        };

        let task_id = self.submit_video_task(&request).await?;
        let video_url = self.poll_until_complete(&task_id).await?;
        self.download_video(&video_url).await
    }

    async fn submit_video_task(&self, request: &VideoGenerationRequest) -> Result<String, ProviderError> {
        log::info!("[Vidu] 提交视频任务: endpoint={}, model={}, prompt_len={}", 
            self.endpoint, request.model, request.prompt.len());

        let response = self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[Vidu] 请求发送失败: {} (is_timeout={}, is_connect={}, is_request={})", 
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
        log::info!("[Vidu] 响应状态: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

        let body = response.text().await
            .map_err(|e| {
                log::error!("[Vidu] 读取响应体失败: {}", e);
                ProviderError::NetworkError(format!("读取响应失败: {}", e))
            })?;

        let truncated_response = if body.len() > 1000 {
            format!("{}...(共{}字符)", truncate_str(&body, 1000), body.len())
        } else {
            body.clone()
        };
        log::info!("[Vidu] 响应体: {}", truncated_response);
        super::save_raw_response("vidu", "video_gen", &body);

        if status.is_success() {
            let gen_response: VideoGenerationResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[Vidu] 解析响应JSON失败: {} (原始响应前200字: {})", e, truncate_str(&body, 200));
                    ProviderError::GenerationFailed(format!("解析响应失败: {}", e))
                })?;
            log::info!("[Vidu] 任务已提交: task_id={}", gen_response.task_id);
            Ok(gen_response.task_id)
        } else {
            self.handle_error_status(status.as_u16(), &body)
        }
    }

    async fn poll_until_complete(&self, task_id: &str) -> Result<String, ProviderError> {
        let poll_url = format!("{}/{}", self.endpoint.trim_end_matches('/'), task_id);
        let start = SystemTime::now();

        loop {
            let elapsed = SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_secs();

            if elapsed >= MAX_POLL_DURATION_SECS {
                log::error!("[Vidu] 轮询超时: task_id={}, elapsed={}s", task_id, elapsed);
                return Err(ProviderError::Timeout(format!(
                    "视频生成超时 ({}秒)，任务ID: {}", elapsed, task_id
                )));
            }

            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;

            log::info!("[Vidu] 轮询状态: task_id={}, elapsed={}s", task_id, elapsed);

            let response = self.client
                .get(&poll_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send()
                .await
                .map_err(|e| {
                    log::error!("[Vidu] 轮询请求失败: {}", e);
                    ProviderError::NetworkError(format!("轮询请求失败: {}", e))
                })?;

            let status = response.status();
            let body = response.text().await
                .map_err(|e| {
                    log::error!("[Vidu] 读取轮询响应失败: {}", e);
                    ProviderError::NetworkError(format!("读取轮询响应失败: {}", e))
                })?;

            let truncated = if body.len() > 500 {
                format!("{}...(共{}字符)", truncate_str(&body, 500), body.len())
            } else {
                body.clone()
            };
            log::info!("[Vidu] 轮询响应: status={}, body={}", status.as_u16(), truncated);
            save_raw_response("vidu", "video_gen_query", &body);

            if !status.is_success() {
                return self.handle_error_status(status.as_u16(), &body);
            }

            let task_status: VideoTaskStatusResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[Vidu] 解析轮询响应失败: {}", e);
                    ProviderError::GenerationFailed(format!("解析轮询响应失败: {}", e))
                })?;

            match task_status.status.as_str() {
                "completed" | "succeed" => {
                    log::info!("[Vidu] 视频生成完成: task_id={}", task_id);
                    return task_status.url
                        .ok_or_else(|| {
                            log::error!("[Vidu] 视频生成完成但未返回视频URL");
                            ProviderError::GenerationFailed("视频生成完成但未返回视频URL".to_string())
                        });
                }
                "failed" => {
                    log::error!("[Vidu] 视频生成失败: task_id={}, error={:?}", task_id, task_status.error);
                    return Err(ProviderError::GenerationFailed(
                        task_status.error
                            .unwrap_or_else(|| "视频生成失败，未知错误".to_string())
                    ));
                }
                "pending" | "processing" | "submitted" => {
                    continue;
                }
                _ => {
                    continue;
                }
            }
        }
    }

    async fn download_video(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        log::info!("[Vidu] 下载视频: url={}", &url[..url.len().min(200)]);

        let response = self.client
            .get(url)
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| {
                log::error!("[Vidu] 下载视频失败: {}", e);
                ProviderError::NetworkError(format!("下载视频失败: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            log::error!("[Vidu] 下载视频响应错误: status={}", status);
            return Err(ProviderError::NetworkError(format!("下载失败，状态码: {}", status)));
        }

        let data = response.bytes().await
            .map(|b| b.to_vec())
            .map_err(|e| {
                log::error!("[Vidu] 读取视频数据失败: {}", e);
                ProviderError::NetworkError(format!("读取视频数据失败: {}", e))
            })?;

        log::info!("[Vidu] 视频下载完成: size={}MB", data.len() / (1024 * 1024));
        Ok(data)
    }

    pub fn build_video_prompt(&self, asset_ref: &AssetRef) -> (String, Option<String>) {
        let style_prefix = asset_ref.style.as_deref().unwrap_or("").trim();
        let base_prompt = asset_ref.prompt.trim();

        let is_cg = asset_ref.id.contains("cg")
            || asset_ref.id.contains("animation")
            || asset_ref.id.contains("cutscene")
            || asset_ref.prompt.to_lowercase().contains("cinematic")
            || asset_ref.prompt.to_lowercase().contains("animation");

        let prompt = if is_cg {
            let mut parts = Vec::new();
            if !style_prefix.is_empty() {
                parts.push(style_prefix.to_string());
            }
            parts.push(base_prompt.to_string());
            parts.push("cinematic".to_string());
            parts.push("dramatic lighting".to_string());
            parts.push("high quality".to_string());
            parts.join(", ")
        } else {
            let mut parts = Vec::new();
            if !style_prefix.is_empty() {
                parts.push(style_prefix.to_string());
            }
            parts.push(base_prompt.to_string());
            parts.push("slow pan".to_string());
            parts.push("ambient".to_string());
            parts.push("looping".to_string());
            parts.push("high quality".to_string());
            parts.join(", ")
        };

        let negative_prompt = asset_ref.negative_prompt.clone();

        (prompt, negative_prompt)
    }

    fn infer_video_params(&self, asset_ref: &AssetRef) -> (String, String) {
        let is_portrait = asset_ref.id.contains("portrait")
            || asset_ref.id.contains("vertical")
            || asset_ref.prompt.to_lowercase().contains("portrait")
            || asset_ref.prompt.to_lowercase().contains("vertical");

        let aspect_ratio = if is_portrait {
            "9:16".to_string()
        } else {
            "16:9".to_string()
        };

        ("5".to_string(), aspect_ratio)
    }

    fn handle_error_status<T>(&self, status_code: u16, body: &str) -> Result<T, ProviderError> {
        let error_msg = serde_json::from_str::<ApiError>(body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or_else(|| body.to_string());

        log::error!("[Vidu] API错误: status={}, message={}", status_code, error_msg);

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
impl IAssetProvider for ViduProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        let (prompt, negative_prompt) = self.build_video_prompt(asset_ref);
        let (duration, aspect_ratio) = self.infer_video_params(asset_ref);

        let video_data = self.generate_video(
            &prompt,
            negative_prompt.as_deref(),
            Some(&duration),
            Some(&aspect_ratio),
        )
        .await?;

        let cache_key = asset_ref.cache_key.clone()
            .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

        let dest_dir = self.asset_base_path.join(&cache_key);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| ProviderError::GenerationFailed(format!("创建资源目录失败: {}", e)))?;

        let dest_path = dest_dir.join(format!("{}.mp4", asset_ref.id));
        std::fs::write(&dest_path, &video_data)
            .map_err(|e| ProviderError::GenerationFailed(format!("写入视频文件失败: {}", e)))?;

        Ok(LocalAsset {
            id: asset_ref.id.clone(),
            asset_type: AssetType::Video,
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

        // 视频生成耗时较长，仅验证 API 密钥有效性
        let request = VideoGenerationRequest {
            model: self.default_model.clone(),
            prompt: "a single frame".to_string(),
            negative_prompt: None,
            duration: Some("5".to_string()),
            aspect_ratio: Some("16:9".to_string()),
        };

        let result = self.submit_video_task(&request).await;
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
                response_preview: Some("视频生成服务可用（完整生成需要较长时间，未执行测试生成）".to_string()),
                test_prompt: None,
                media_url: None,
                media_type: None,
                request_endpoint: None,
                request_model: None,
                request_headers: None,
                request_body: None,
                response_status: None,
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
                media_url: None,
                media_type: None,
                request_endpoint: None,
                request_model: None,
                request_headers: None,
                request_body: None,
                response_status: None,
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
                media_url: None,
                media_type: None,
                request_endpoint: None,
                request_model: None,
                request_headers: None,
                request_body: None,
                response_status: None,
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
                media_url: None,
                media_type: None,
                request_endpoint: None,
                request_model: None,
                request_headers: None,
                request_body: None,
                response_status: None,
            }),
        }
    }

    fn supported_modalities(&self) -> Vec<AIModality> {
        vec![AIModality::Video]
    }

    fn provider_id(&self) -> &str {
        &self.config.id
    }
}
