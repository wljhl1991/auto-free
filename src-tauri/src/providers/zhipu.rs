use async_trait::async_trait;
use super::{truncate_str, save_raw_response, IAssetProvider, ProviderError};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_RETRIES: u32 = 3;
const DEFAULT_TEXT_ENDPOINT: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";
const DEFAULT_IMAGE_ENDPOINT: &str = "https://open.bigmodel.cn/api/paas/v4/images/generations";
const DEFAULT_VIDEO_ENDPOINT: &str = "https://open.bigmodel.cn/api/paas/v4/videos/generations";
const DEFAULT_TEXT_MODEL: &str = "glm-4-flash";
const DEFAULT_IMAGE_MODEL: &str = "cogview-3-flash";
const DEFAULT_VIDEO_MODEL: &str = "cogvideox-flash";
const POLL_INTERVAL_SECS: u64 = 5;
const MAX_POLL_DURATION_SECS: u64 = 300;

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

#[derive(Debug, Serialize)]
struct ImageGenerationRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImageGenerationResponse {
    data: Vec<ImageData>,
}

#[derive(Debug, Deserialize)]
struct ImageData {
    url: String,
}

#[derive(Debug, Serialize)]
struct VideoGenerationRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct VideoGenerationResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct VideoTaskStatusResponse {
    id: String,
    task_status: String,
    #[serde(default)]
    data: Option<Vec<VideoResultData>>,
}

#[derive(Debug, Deserialize)]
struct VideoResultData {
    url: String,
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
    default_text_model: String,
    default_image_model: String,
    default_video_model: String,
    api_key: String,
    asset_base_path: PathBuf,
}

impl ZhipuProvider {
    pub fn new(config: &AIProviderConfig, asset_base_path: &std::path::Path) -> Result<Self, ProviderError> {
        let api_key = config.auth_config.api_key
            .as_ref()
            .map(|k| k.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("智谱 API Key 未配置".to_string()))?;

        let default_text_model = config.models
            .iter()
            .find(|m| m.is_default && m.modality == AIModality::Text)
            .map(|m| m.id.clone())
            .or_else(|| config.models.iter().find(|m| m.modality == AIModality::Text).map(|m| m.id.clone()))
            .unwrap_or_else(|| DEFAULT_TEXT_MODEL.to_string());

        let default_image_model = config.models
            .iter()
            .find(|m| m.is_default && m.modality == AIModality::Image)
            .map(|m| m.id.clone())
            .or_else(|| config.models.iter().find(|m| m.modality == AIModality::Image).map(|m| m.id.clone()))
            .unwrap_or_else(|| DEFAULT_IMAGE_MODEL.to_string());

        let default_video_model = config.models
            .iter()
            .find(|m| m.is_default && m.modality == AIModality::Video)
            .map(|m| m.id.clone())
            .or_else(|| config.models.iter().find(|m| m.modality == AIModality::Video).map(|m| m.id.clone()))
            .unwrap_or_else(|| DEFAULT_VIDEO_MODEL.to_string());

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| ProviderError::NetworkError(format!("创建HTTP客户端失败: {}", e)))?;

        Ok(Self {
            config: config.clone(),
            client,
            default_text_model,
            default_image_model,
            default_video_model,
            api_key,
            asset_base_path: asset_base_path.to_path_buf(),
        })
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>, model: Option<&str>) -> Result<String, ProviderError> {
        let model = model.unwrap_or(&self.default_text_model);
        let max_tokens = self.config.models
            .iter()
            .find(|m| m.id == model)
            .and_then(|m| m.max_tokens);

        let endpoint = self.config.models
            .iter()
            .find(|m| m.id == model)
            .and_then(|m| {
                let ep = m.endpoint.trim().to_string();
                if ep.is_empty() { None } else { Some(ep) }
            })
            .unwrap_or_else(|| DEFAULT_TEXT_ENDPOINT.to_string());

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

            match self.send_chat_request(&request, &endpoint).await {
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
        Err(last_error.unwrap_or_else(|| ProviderError::NetworkError("重试后仍然失败".to_string())))
    }

    async fn send_chat_request(&self, request: &ChatRequest, endpoint: &str) -> Result<String, ProviderError> {
        log::info!("[Zhipu] 文本请求: endpoint={}, model={}, messages_count={}", 
            endpoint, request.model, request.messages.len());

        let request_body = serde_json::to_string(request).unwrap_or_default();
        let truncated_body = if request_body.len() > 500 {
            format!("{}...(共{}字符)", truncate_str(&request_body, 500), request_body.len())
        } else {
            request_body.clone()
        };
        log::debug!("[Zhipu] 文本请求体: {}", truncated_body);

        let response = self.client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[Zhipu] 文本请求发送失败: {} (is_timeout={}, is_connect={}, is_request={})", 
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
        log::info!("[Zhipu] 文本响应状态: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

        let body_bytes = response.bytes().await
            .map_err(|e| {
                log::error!("[Zhipu] 读取文本响应体失败: {}", e);
                ProviderError::NetworkError(format!("读取响应失败: {} (可能是网络中断或响应编码异常)", e))
            })?;
        let body = String::from_utf8_lossy(&body_bytes).to_string();

        let truncated_response = if body.len() > 1000 {
            format!("{}...(共{}字符)", truncate_str(&body, 1000), body.len())
        } else {
            body.clone()
        };
        log::info!("[Zhipu] 文本响应体: {}", truncated_response);
        save_raw_response("zhipu", "chat", &body);

        if status.is_success() {
            let chat_response: ChatResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[Zhipu] 解析文本响应JSON失败: {} (原始响应前200字: {})", e, truncate_str(&body, 200));
                    ProviderError::GenerationFailed(format!("解析响应失败: {} (响应可能不是有效JSON)", e))
                })?;
            chat_response.choices.first()
                .map(|c| c.message.content.clone())
                .ok_or_else(|| {
                    log::error!("[Zhipu] 文本响应中没有choices");
                    ProviderError::GenerationFailed("响应中没有choices".to_string())
                })
        } else {
            self.handle_error_status(status.as_u16(), &body)
        }
    }

    pub async fn generate_image(
        &self,
        prompt: &str,
        size: Option<&str>,
        model: Option<&str>,
    ) -> Result<Vec<u8>, ProviderError> {
        let model = model.unwrap_or(&self.default_image_model);
        let endpoint = self.config.models
            .iter()
            .find(|m| m.id == model)
            .and_then(|m| {
                let ep = m.endpoint.trim().to_string();
                if ep.is_empty() { None } else { Some(ep) }
            })
            .unwrap_or_else(|| DEFAULT_IMAGE_ENDPOINT.to_string());

        let request = ImageGenerationRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            size: size.map(|s| s.to_string()),
        };

        let mut last_error = None;
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
            }

            match self.send_image_request(&request, &endpoint).await {
                Ok(image_url) => {
                    return self.download_image(&image_url).await;
                }
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
        Err(last_error.unwrap_or_else(|| ProviderError::NetworkError("重试后仍然失败".to_string())))
    }

    async fn send_image_request(&self, request: &ImageGenerationRequest, endpoint: &str) -> Result<String, ProviderError> {
        log::info!("[Zhipu] 图片生成请求: endpoint={}, model={}, prompt={}", 
            endpoint, request.model, truncate_str(&request.prompt, 500));

        let request_body = serde_json::to_string(request).unwrap_or_default();
        log::debug!("[Zhipu] 图片请求体: {}", request_body);

        let response = self.client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[Zhipu] 图片请求发送失败: {}", e);
                if e.is_timeout() {
                    ProviderError::Timeout(format!("请求超时: {}", e))
                } else if e.is_connect() {
                    ProviderError::NetworkError(format!("连接失败: {} (请检查网络或API地址是否正确)", e))
                } else {
                    ProviderError::NetworkError(format!("网络错误: {}", e))
                }
            })?;

        let status = response.status();
        log::info!("[Zhipu] 图片响应状态: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

        let body = response.text().await
            .map_err(|e| {
                log::error!("[Zhipu] 读取图片响应体失败: {}", e);
                ProviderError::NetworkError(format!("读取响应失败: {}", e))
            })?;

        let truncated_response = if body.len() > 1000 {
            format!("{}...(共{}字符)", truncate_str(&body, 1000), body.len())
        } else {
            body.clone()
        };
        log::info!("[Zhipu] 图片响应体: {}", truncated_response);
        save_raw_response("zhipu", "image", &body);

        if status.is_success() {
            let gen_response: ImageGenerationResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[Zhipu] 解析图片响应JSON失败: {}", e);
                    ProviderError::GenerationFailed(format!("解析响应失败: {}", e))
                })?;
            gen_response.data.first()
                .map(|d| d.url.clone())
                .ok_or_else(|| {
                    log::error!("[Zhipu] 图片响应中没有data");
                    ProviderError::GenerationFailed("响应中没有图片数据".to_string())
                })
        } else {
            self.handle_error_status(status.as_u16(), &body)
        }
    }

    async fn download_image(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        log::info!("[Zhipu] 下载图片: url={}", &url[..url.len().min(200)]);

        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| {
                log::error!("[Zhipu] 下载图片失败: {}", e);
                ProviderError::NetworkError(format!("下载图片失败: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            log::error!("[Zhipu] 下载图片响应错误: status={}", status);
            return Err(ProviderError::NetworkError(format!("下载失败，状态码: {}", status)));
        }

        let data = response.bytes().await
            .map(|b| b.to_vec())
            .map_err(|e| {
                log::error!("[Zhipu] 读取图片数据失败: {}", e);
                ProviderError::NetworkError(format!("读取图片数据失败: {}", e))
            })?;

        log::info!("[Zhipu] 图片下载完成: size={}KB", data.len() / 1024);
        Ok(data)
    }

    pub async fn generate_video(
        &self,
        prompt: &str,
        duration: Option<u64>,
        model: Option<&str>,
    ) -> Result<Vec<u8>, ProviderError> {
        let model = model.unwrap_or(&self.default_video_model);
        let endpoint = self.config.models
            .iter()
            .find(|m| m.id == model)
            .and_then(|m| {
                let ep = m.endpoint.trim().to_string();
                if ep.is_empty() { None } else { Some(ep) }
            })
            .unwrap_or_else(|| DEFAULT_VIDEO_ENDPOINT.to_string());

        let request = VideoGenerationRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            duration,
        };

        let mut last_error = None;
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
            }

            match self.submit_video_task(&request, &endpoint).await {
                Ok(task_id) => {
                    log::info!("[Zhipu] 视频任务已提交: task_id={}", task_id);
                    let video_url = self.poll_video_task(&task_id, &endpoint).await?;
                    return self.download_video(&video_url).await;
                }
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
        Err(last_error.unwrap_or_else(|| ProviderError::NetworkError("重试后仍然失败".to_string())))
    }

    async fn submit_video_task(&self, request: &VideoGenerationRequest, endpoint: &str) -> Result<String, ProviderError> {
        log::info!("[Zhipu] 提交视频任务: endpoint={}, model={}, prompt={}", 
            endpoint, request.model, truncate_str(&request.prompt, 500));

        let request_body = serde_json::to_string(request).unwrap_or_default();
        log::debug!("[Zhipu] 视频请求体: {}", request_body);

        let response = self.client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[Zhipu] 视频请求发送失败: {}", e);
                if e.is_timeout() {
                    ProviderError::Timeout(format!("请求超时: {}", e))
                } else if e.is_connect() {
                    ProviderError::NetworkError(format!("连接失败: {} (请检查网络或API地址是否正确)", e))
                } else {
                    ProviderError::NetworkError(format!("网络错误: {}", e))
                }
            })?;

        let status = response.status();
        log::info!("[Zhipu] 视频提交响应状态: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

        let body = response.text().await
            .map_err(|e| {
                log::error!("[Zhipu] 读取视频响应体失败: {}", e);
                ProviderError::NetworkError(format!("读取响应失败: {}", e))
            })?;

        let truncated_response = if body.len() > 1000 {
            format!("{}...(共{}字符)", truncate_str(&body, 1000), body.len())
        } else {
            body.clone()
        };
        log::info!("[Zhipu] 视频提交响应体: {}", truncated_response);
        save_raw_response("zhipu", "video_submit", &body);

        if status.is_success() {
            let gen_response: VideoGenerationResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[Zhipu] 解析视频响应JSON失败: {}", e);
                    ProviderError::GenerationFailed(format!("解析响应失败: {}", e))
                })?;
            Ok(gen_response.id)
        } else {
            self.handle_error_status(status.as_u16(), &body)
        }
    }

    async fn poll_video_task(&self, task_id: &str, endpoint: &str) -> Result<String, ProviderError> {
        let poll_url = format!("{}/{}", endpoint.trim_end_matches('/'), task_id);
        let start = SystemTime::now();

        loop {
            let elapsed = SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_secs();

            if elapsed >= MAX_POLL_DURATION_SECS {
                log::error!("[Zhipu] 视频轮询超时: task_id={}, elapsed={}s", task_id, elapsed);
                return Err(ProviderError::Timeout(format!(
                    "视频生成超时 ({}秒)，任务ID: {}", elapsed, task_id
                )));
            }

            log::info!("[Zhipu] 轮询视频任务: task_id={}, elapsed={}s", task_id, elapsed);

            let response = self.client
                .get(&poll_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send()
                .await
                .map_err(|e| {
                    log::error!("[Zhipu] 轮询请求失败: {}", e);
                    ProviderError::NetworkError(format!("轮询请求失败: {}", e))
                })?;

            let status = response.status();
            let body = response.text().await
                .map_err(|e| {
                    log::error!("[Zhipu] 读取轮询响应失败: {}", e);
                    ProviderError::NetworkError(format!("读取轮询响应失败: {}", e))
                })?;

            let truncated = if body.len() > 1000 {
                format!("{}...(共{}字符)", truncate_str(&body, 1000), body.len())
            } else {
                body.clone()
            };
            log::info!("[Zhipu] 轮询响应: status={}, body={}", status.as_u16(), truncated);
            save_raw_response("zhipu", "video_poll", &body);

            if !status.is_success() {
                return self.handle_error_status(status.as_u16(), &body);
            }

            let poll_response: VideoTaskStatusResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[Zhipu] 解析轮询响应JSON失败: {}", e);
                    ProviderError::GenerationFailed(format!("解析轮询响应失败: {}", e))
                })?;

            match poll_response.task_status.as_str() {
                "PROCESSING" | "PROCESSING-UPLOADING" | "PENDING" => {
                    log::info!("[Zhipu] 任务处理中: task_status={}", poll_response.task_status);
                    tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                }
                "SUCCESS" => {
                    log::info!("[Zhipu] 视频生成成功: task_id={}", task_id);
                    let video_url = poll_response.data
                        .and_then(|d| d.first().map(|v| v.url.clone()))
                        .ok_or_else(|| {
                            log::error!("[Zhipu] 响应中没有视频URL");
                            ProviderError::GenerationFailed("响应中没有视频URL".to_string())
                        })?;
                    return Ok(video_url);
                }
                "FAIL" => {
                    log::error!("[Zhipu] 视频生成失败: task_id={}", task_id);
                    return Err(ProviderError::GenerationFailed(format!("视频生成失败，任务ID: {}", task_id)));
                }
                status => {
                    log::warn!("[Zhipu] 未知任务状态: {}", status);
                    tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                }
            }
        }
    }

    async fn download_video(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        log::info!("[Zhipu] 下载视频: url={}", &url[..url.len().min(200)]);

        let response = self.client
            .get(url)
            .timeout(Duration::from_secs(300))
            .send()
            .await
            .map_err(|e| {
                log::error!("[Zhipu] 下载视频失败: {}", e);
                ProviderError::NetworkError(format!("下载视频失败: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            log::error!("[Zhipu] 下载视频响应错误: status={}", status);
            return Err(ProviderError::NetworkError(format!("下载失败，状态码: {}", status)));
        }

        let data = response.bytes().await
            .map(|b| b.to_vec())
            .map_err(|e| {
                log::error!("[Zhipu] 读取视频数据失败: {}", e);
                ProviderError::NetworkError(format!("读取视频数据失败: {}", e))
            })?;

        log::info!("[Zhipu] 视频下载完成: size={}MB", data.len() / (1024 * 1024));
        Ok(data)
    }

    pub fn build_image_prompt(&self, asset_ref: &AssetRef) -> (String, String) {
        let size = self.infer_image_size(asset_ref);

        let style_prefix = asset_ref.style.as_deref().unwrap_or("").trim();
        let base_prompt = asset_ref.prompt.trim();

        let prompt = if asset_ref.id.contains("avatar") || asset_ref.id.contains("portrait") {
            let mut parts = Vec::new();
            if !style_prefix.is_empty() {
                parts.push(style_prefix.to_string());
            }
            parts.push(base_prompt.to_string());
            parts.push("portrait".to_string());
            parts.push("game character".to_string());
            parts.push("detailed face".to_string());
            parts.join(", ")
        } else {
            let mut parts = Vec::new();
            if !style_prefix.is_empty() {
                parts.push(style_prefix.to_string());
            }
            parts.push(base_prompt.to_string());
            parts.push("game background".to_string());
            parts.push("high quality".to_string());
            parts.push("detailed".to_string());
            parts.join(", ")
        };

        (prompt, size)
    }

    fn infer_image_size(&self, asset_ref: &AssetRef) -> String {
        let is_avatar = asset_ref.id.contains("avatar")
            || asset_ref.id.contains("portrait")
            || asset_ref.id.contains("head");

        if is_avatar {
            "1024x1024".to_string()
        } else {
            "1024x1024".to_string()
        }
    }

    pub fn build_video_prompt(&self, asset_ref: &AssetRef) -> (String, Option<u64>) {
        let style_prefix = asset_ref.style.as_deref().unwrap_or("").trim();
        let base_prompt = asset_ref.prompt.trim();

        let prompt = {
            let mut parts = Vec::new();
            if !style_prefix.is_empty() {
                parts.push(style_prefix.to_string());
            }
            parts.push(base_prompt.to_string());
            parts.push("cinematic".to_string());
            parts.push("dramatic lighting".to_string());
            parts.push("high quality".to_string());
            parts.join(", ")
        };

        let duration = if asset_ref.id.contains("cutscene") || asset_ref.id.contains("animation") {
            Some(6)
        } else {
            Some(5)
        };

        (prompt, duration)
    }

    fn handle_error_status<T>(&self, status_code: u16, body: &str) -> Result<T, ProviderError> {
        let error_msg = serde_json::from_str::<ApiError>(body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or_else(|| body.to_string());

        log::error!("[Zhipu] API错误: status={}, message={}", status_code, error_msg);

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

    async fn download_and_save_test_media(&self, url: &str, media_type: &str) -> Result<String, ProviderError> {
        let data = if media_type == "image" {
            self.download_image(url).await?
        } else {
            self.download_video(url).await?
        };

        let cache_dir = self.asset_base_path.join("cache");
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| ProviderError::GenerationFailed(format!("创建缓存目录失败: {}", e)))?;

        let ext = if media_type == "image" { "png" } else { "mp4" };
        let filename = format!("zhipu_test_{}_{}.{}", media_type,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(), ext);
        let dest_path = cache_dir.join(&filename);
        std::fs::write(&dest_path, &data)
            .map_err(|e| ProviderError::GenerationFailed(format!("写入测试文件失败: {}", e)))?;

        Ok(dest_path.to_string_lossy().to_string())
    }
}

#[async_trait]
impl IAssetProvider for ZhipuProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        match asset_ref.asset_type {
            crate::types::game_script::AssetType::Image => {
                let (prompt, size) = self.build_image_prompt(asset_ref);
                let image_data = self.generate_image(&prompt, Some(&size), None).await?;

                let cache_key = asset_ref.cache_key.clone()
                    .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

                let dest_dir = self.asset_base_path.join("cacheAssets").join(&cache_key);
                std::fs::create_dir_all(&dest_dir)
                    .map_err(|e| ProviderError::GenerationFailed(format!("创建资源目录失败: {}", e)))?;

                let dest_path = dest_dir.join(format!("{}.png", asset_ref.id));
                std::fs::write(&dest_path, &image_data)
                    .map_err(|e| ProviderError::GenerationFailed(format!("写入图片文件失败: {}", e)))?;

                Ok(LocalAsset {
                    id: asset_ref.id.clone(),
                    asset_type: AssetType::Image,
                    local_path: dest_path.to_string_lossy().to_string(),
                    source: AssetSource::AiGenerated,
                    cache_key,
                    created_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                })
            }
            crate::types::game_script::AssetType::Video => {
                let (prompt, duration) = self.build_video_prompt(asset_ref);
                let video_data = self.generate_video(&prompt, duration, None).await?;

                let cache_key = asset_ref.cache_key.clone()
                    .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

                let dest_dir = self.asset_base_path.join("cacheAssets").join(&cache_key);
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
            _ => {
                let messages = vec![
                    ChatMessage {
                        role: "user".to_string(),
                        content: asset_ref.prompt.clone(),
                    },
                ];

                let text = self.chat(messages, None).await?;

                let cache_key = asset_ref.cache_key.clone()
                    .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

                let dest_dir = self.asset_base_path.join("cacheAssets").join(&cache_key);
                std::fs::create_dir_all(&dest_dir)
                    .map_err(|e| ProviderError::GenerationFailed(format!("创建资源目录失败: {}", e)))?;

                let dest_path = dest_dir.join(format!("{}.txt", asset_ref.id));
                std::fs::write(&dest_path, &text)
                    .map_err(|e| ProviderError::GenerationFailed(format!("写入文本文件失败: {}", e)))?;

                Ok(LocalAsset {
                    id: asset_ref.id.clone(),
                    asset_type: AssetType::Audio, // Use Audio for text content (same as DeepSeek)
                    local_path: dest_path.to_string_lossy().to_string(),
                    source: AssetSource::AiGenerated,
                    cache_key,
                    created_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                })
            }
        }
    }

    async fn check_connectivity(&self) -> Result<ConnectivityCheck, ProviderError> {
        self.check_connectivity_with_prompt("hi").await
    }

    async fn check_connectivity_with_prompt(&self, prompt: &str) -> Result<ConnectivityCheck, ProviderError> {
        let start = SystemTime::now();

        let selected_model = self.config.models.iter().find(|m| m.is_default)
            .or_else(|| self.config.models.first());

        let modality = selected_model.map(|m| m.modality.clone()).unwrap_or(AIModality::Text);
        let model_id = selected_model.map(|m| m.id.clone()).unwrap_or_else(|| self.default_text_model.clone());

        let test_prompt = if !prompt.trim().is_empty() && prompt.len() <= 2000 {
            prompt.trim()
        } else {
            "hi"
        };

        match modality {
            AIModality::Text => {
                let endpoint = self.config.models
                    .iter()
                    .find(|m| m.id == model_id)
                    .and_then(|m| {
                        let ep = m.endpoint.trim().to_string();
                        if ep.is_empty() { None } else { Some(ep) }
                    })
                    .unwrap_or_else(|| DEFAULT_TEXT_ENDPOINT.to_string());

                let messages = vec![
                    ChatMessage {
                        role: "user".to_string(),
                        content: test_prompt.to_string(),
                    },
                ];

                let request_body = serde_json::to_string(&ChatRequest {
                    model: model_id.clone(),
                    messages: messages.clone(),
                    max_tokens: None,
                    temperature: Some(0.7),
                    stream: Some(false),
                }).unwrap_or_default();
                let request_headers = r#"{"Authorization":"Bearer ***","Content-Type":"application/json"}"#.to_string();

                let result = self.chat(messages, Some(&model_id)).await;
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
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(endpoint),
                        request_model: Some(model_id),
                        request_headers: Some(request_headers),
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
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(endpoint),
                        request_model: Some(model_id),
                        request_headers: Some(request_headers),
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
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(endpoint),
                        request_model: Some(model_id),
                        request_headers: Some(request_headers),
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
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(endpoint),
                        request_model: Some(model_id),
                        request_headers: Some(request_headers),
                        request_body: Some(truncate_str(&request_body, 2000).to_string()),
                        response_status: None,
                    }),
                }
            }
            AIModality::Image => {
                let endpoint = self.config.models
                    .iter()
                    .find(|m| m.id == model_id)
                    .and_then(|m| {
                        let ep = m.endpoint.trim().to_string();
                        if ep.is_empty() { None } else { Some(ep) }
                    })
                    .unwrap_or_else(|| DEFAULT_IMAGE_ENDPOINT.to_string());

                let request = ImageGenerationRequest {
                    model: model_id.clone(),
                    prompt: test_prompt.to_string(),
                    size: Some("1024x1024".to_string()),
                };

                let request_body = serde_json::to_string(&request).unwrap_or_default();
                let request_headers = r#"{"Authorization":"Bearer ***","Content-Type":"application/json"}"#.to_string();

                let result = self.send_image_request(&request, &endpoint).await;
                let latency = SystemTime::now()
                    .duration_since(start)
                    .unwrap_or_default()
                    .as_millis() as u64;

                match result {
                    Ok(image_url) => {
                        let media_url = self.download_and_save_test_media(&image_url, "image").await.ok();
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
                            test_prompt: Some(test_prompt.to_string()),
                            media_url,
                            media_type: Some("image".to_string()),
                            polling_task_id: None,
                            polling_status: None,
                            polling_elapsed_secs: None,
                            media_items: None,
                            request_endpoint: Some(endpoint),
                            request_model: Some(model_id),
                            request_headers: Some(request_headers),
                            request_body: Some(truncate_str(&request_body, 2000).to_string()),
                            response_status: Some(200),
                        })
                    }
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
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(endpoint),
                        request_model: Some(model_id),
                        request_headers: Some(request_headers),
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
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(endpoint),
                        request_model: Some(model_id),
                        request_headers: Some(request_headers),
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
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(endpoint),
                        request_model: Some(model_id),
                        request_headers: Some(request_headers),
                        request_body: Some(truncate_str(&request_body, 2000).to_string()),
                        response_status: None,
                    }),
                }
            }
            AIModality::Video => {
                let endpoint = self.config.models
                    .iter()
                    .find(|m| m.id == model_id)
                    .and_then(|m| {
                        let ep = m.endpoint.trim().to_string();
                        if ep.is_empty() { None } else { Some(ep) }
                    })
                    .unwrap_or_else(|| DEFAULT_VIDEO_ENDPOINT.to_string());

                let request = VideoGenerationRequest {
                    model: model_id.clone(),
                    prompt: test_prompt.to_string(),
                    duration: Some(5),
                };

                let request_body = serde_json::to_string(&request).unwrap_or_default();
                let request_headers = r#"{"Authorization":"Bearer ***","Content-Type":"application/json"}"#.to_string();

                let result = self.submit_video_task(&request, &endpoint).await;
                let latency = SystemTime::now()
                    .duration_since(start)
                    .unwrap_or_default()
                    .as_millis() as u64;

                match result {
                    Ok(task_id) => {
                        // 对于连通性检查，只验证任务提交成功，不等待完整生成
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
                            response_preview: Some(format!("任务已提交: task_id={}", task_id)),
                            test_prompt: Some(test_prompt.to_string()),
                            media_url: None,
                            media_type: Some("video".to_string()),
                            polling_task_id: None,
                            polling_status: None,
                            polling_elapsed_secs: None,
                            media_items: None,
                            request_endpoint: Some(endpoint),
                            request_model: Some(model_id),
                            request_headers: Some(request_headers),
                            request_body: Some(truncate_str(&request_body, 2000).to_string()),
                            response_status: Some(200),
                        })
                    }
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
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(endpoint),
                        request_model: Some(model_id),
                        request_headers: Some(request_headers),
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
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(endpoint),
                        request_model: Some(model_id),
                        request_headers: Some(request_headers),
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
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(endpoint),
                        request_model: Some(model_id),
                        request_headers: Some(request_headers),
                        request_body: Some(truncate_str(&request_body, 2000).to_string()),
                        response_status: None,
                    }),
                }
            }
            _ => {
                self.check_connectivity_with_prompt(test_prompt).await
            }
        }
    }

    fn supported_modalities(&self) -> Vec<AIModality> {
        vec![AIModality::Text, AIModality::Image, AIModality::Video]
    }

    fn provider_id(&self) -> &str {
        &self.config.id
    }
}