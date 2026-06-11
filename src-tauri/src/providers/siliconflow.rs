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
const DEFAULT_ENDPOINT: &str = "https://api.siliconflow.cn/v1/images/generations";
const DEFAULT_IMAGE_MODEL: &str = "Kwai-Kolors/Kolors";
const DEFAULT_TEXT_MODEL: &str = "deepseek-ai/DeepSeek-V3";

#[derive(Debug, Serialize)]
struct ImageGenerationRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_inference_steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guidance_scale: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ImageGenerationResponse {
    images: Vec<ImageData>,
}

#[derive(Debug, Deserialize)]
struct ImageData {
    url: String,
}

/// 文本生成请求体（OpenAI 兼容格式）
#[derive(Debug, Serialize)]
struct TextGenerationRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// 文本生成响应体（OpenAI 兼容格式）
#[derive(Debug, Deserialize)]
struct TextGenerationResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: Option<u32>,
    #[serde(default)]
    completion_tokens: Option<u32>,
    #[serde(default)]
    total_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    error: Option<ApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: Option<String>,
}

pub struct SiliconFlowProvider {
    config: AIProviderConfig,
    client: Client,
    api_key: String,
    default_image_model: String,
    #[allow(dead_code)]
    default_text_model: String, // reserved for future text generation support
    endpoint: String,
    asset_base_path: PathBuf,
}

impl SiliconFlowProvider {
    pub fn new(config: &AIProviderConfig, asset_base_path: PathBuf) -> Result<Self, ProviderError> {
        let api_key = config.auth_config.api_key
            .as_ref()
            .map(|k| k.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("SiliconFlow API Key 未配置".to_string()))?;

        let default_image_model = config.models
            .iter()
            .find(|m| m.is_default && m.modality == AIModality::Image)
            .map(|m| m.id.clone())
            .or_else(|| config.models.iter().find(|m| m.modality == AIModality::Image).map(|m| m.id.clone()))
            .unwrap_or_else(|| DEFAULT_IMAGE_MODEL.to_string());

        let default_text_model = config.models
            .iter()
            .find(|m| m.is_default && m.modality == AIModality::Text)
            .map(|m| m.id.clone())
            .or_else(|| config.models.iter().find(|m| m.modality == AIModality::Text).map(|m| m.id.clone()))
            .unwrap_or_else(|| DEFAULT_TEXT_MODEL.to_string());

        let endpoint = config.models
            .iter()
            .find(|m| m.is_default && m.modality == AIModality::Image)
            .and_then(|m| {
                let ep = m.endpoint.trim().to_string();
                if ep.is_empty() { None } else { Some(ep) }
            })
            .or_else(|| {
                config.models.iter().find(|m| m.modality == AIModality::Image).and_then(|m| {
                    let ep = m.endpoint.trim().to_string();
                    if ep.is_empty() { None } else { Some(ep) }
                })
            })
            .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string());

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| ProviderError::NetworkError(format!("创建HTTP客户端失败: {}", e)))?;

        Ok(Self {
            config: config.clone(),
            client,
            api_key,
            default_image_model,
            default_text_model,
            endpoint,
            asset_base_path,
        })
    }

    /// 从模型配置的 advanced_params 读取高级参数
    fn get_advanced_params(&self, model_id: &str) -> (Option<u32>, Option<f64>, Option<u64>) {
        let model = self.config.models.iter().find(|m| m.id == model_id);
        let params = model.and_then(|m| m.advanced_params.as_ref());

        let num_inference_steps = params
            .and_then(|p| p.get("num_inference_steps"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let guidance_scale = params
            .and_then(|p| p.get("guidance_scale"))
            .and_then(|v| v.as_f64());

        let seed = params
            .and_then(|p| p.get("seed"))
            .and_then(|v| v.as_u64());

        (num_inference_steps, guidance_scale, seed)
    }

    /// 生成图片
    pub async fn generate_image(
        &self,
        prompt: &str,
        negative_prompt: Option<&str>,
        size: Option<&str>,
        model: Option<&str>,
    ) -> Result<Vec<u8>, ProviderError> {
        let model = model.unwrap_or(&self.default_image_model);

        // 从当前模型的 advanced_params 读取高级参数
        let (num_inference_steps, guidance_scale, seed) = self.get_advanced_params(model);

        let request = ImageGenerationRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            negative_prompt: negative_prompt.map(|s| s.to_string()),
            image_size: size.map(|s| s.to_string()),
            num_inference_steps,
            guidance_scale,
            seed,
        };

        let mut last_error = None;
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
            }

            match self.send_image_request(&request).await {
                Ok(image_url) => {
                    return self.download_image(&image_url).await;
                }
                Err(ProviderError::QuotaExceeded(msg)) => {
                    // 429 速率限制：等待后重试
                    log::warn!("[SiliconFlow] 请求频率超限 (attempt {}/{}): {}", attempt + 1, MAX_RETRIES + 1, msg);
                    if attempt < MAX_RETRIES {
                        // 429 重试等待时间：指数退避，基础 10 秒
                        let backoff = Duration::from_secs(10 * (1 << attempt.min(3)));
                        log::info!("[SiliconFlow] 等待 {:?} 后重试...", backoff);
                        tokio::time::sleep(backoff).await;
                        last_error = Some(ProviderError::QuotaExceeded(msg));
                        continue;
                    }
                    return Err(ProviderError::QuotaExceeded(msg));
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

    async fn send_image_request(&self, request: &ImageGenerationRequest) -> Result<String, ProviderError> {
        log::info!("[SiliconFlow] 图片生成请求: endpoint={}, model={}, prompt={}, negative_prompt={}, image_size={}, num_inference_steps={}, guidance_scale={}, seed={}",
            self.endpoint,
            request.model,
            truncate_str(&request.prompt, 2000),
            request.negative_prompt.as_deref().unwrap_or("None"),
            request.image_size.as_deref().unwrap_or("None"),
            request.num_inference_steps.map(|s| s.to_string()).as_deref().unwrap_or("None"),
            request.guidance_scale.map(|s| s.to_string()).as_deref().unwrap_or("None"),
            request.seed.map(|s| s.to_string()).as_deref().unwrap_or("None"),
        );

        let response = self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[SiliconFlow] 请求发送失败: {} (is_timeout={}, is_connect={}, is_request={})", 
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
        // 尝试读取 Retry-After 头（429 响应时可能包含）
        let retry_after = if status.as_u16() == 429 {
            response.headers().get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs)
        } else {
            None
        };

        log::info!("[SiliconFlow] 响应状态: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

        let body = response.text().await
            .map_err(|e| {
                log::error!("[SiliconFlow] 读取响应体失败: {}", e);
                ProviderError::NetworkError(format!("读取响应失败: {} (可能是网络中断或响应编码异常)", e))
            })?;

        let truncated_response = if body.len() > 1000 {
            format!("{}...(共{}字符)", truncate_str(&body, 1000), body.len())
        } else {
            body.clone()
        };
        log::info!("[SiliconFlow] 响应体: {}", truncated_response);
        save_raw_response("siliconflow", "image_gen", &body);

        if status.is_success() {
            let gen_response: ImageGenerationResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[SiliconFlow] 解析响应JSON失败: {} (原始响应前200字: {})", e, truncate_str(&body, 200));
                    ProviderError::GenerationFailed(format!("解析响应失败: {} (响应可能不是有效JSON)", e))
                })?;
            gen_response.images.first()
                .map(|img| img.url.clone())
                .ok_or_else(|| {
                    log::error!("[SiliconFlow] 响应中没有图片数据");
                    ProviderError::GenerationFailed("响应中没有图片数据".to_string())
                })
        } else {
            // 对 429 响应，将 Retry-After 信息附加到错误消息中
            if status.as_u16() == 429 {
                let base_msg = serde_json::from_str::<ApiError>(&body)
                    .ok()
                    .and_then(|e| e.error)
                    .and_then(|e| e.message)
                    .unwrap_or_else(|| body.to_string());
                let msg = match retry_after {
                    Some(d) => format!("{} (Retry-After: {}s)", base_msg, d.as_secs()),
                    None => base_msg,
                };
                log::error!("[SiliconFlow] API速率限制: status=429, message={}", msg);
                return Err(ProviderError::QuotaExceeded(msg));
            }
            self.handle_error_status(status.as_u16(), &body)
        }
    }

    /// 发送文本生成请求（使用 /chat/completions 端点）
    async fn send_text_request(&self, request: &TextGenerationRequest) -> Result<String, ProviderError> {
        let text_endpoint = "https://api.siliconflow.cn/v1/chat/completions";
        
        log::info!("[SiliconFlow] 文本生成请求: endpoint={}, model={}, messages_count={}",
            text_endpoint, request.model, request.messages.len());
        
        // 记录请求体
        let request_body = serde_json::to_string(request).unwrap_or_default();
        let truncated_body = if request_body.len() > 2000 {
            format!("{}...(共{}字符)", truncate_str(&request_body, 2000), request_body.len())
        } else {
            request_body.clone()
        };
        log::info!("[SiliconFlow] 文本请求体: {}", truncated_body);

        let response = self.client
            .post(text_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[SiliconFlow] 文本请求发送失败: {} (is_timeout={}, is_connect={}, is_request={})", 
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
        log::info!("[SiliconFlow] 文本响应状态: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

        let body = response.text().await
            .map_err(|e| {
                log::error!("[SiliconFlow] 读取文本响应体失败: {}", e);
                ProviderError::NetworkError(format!("读取响应失败: {}", e))
            })?;

        let truncated_response = if body.len() > 1000 {
            format!("{}...(共{}字符)", truncate_str(&body, 1000), body.len())
        } else {
            body.clone()
        };
        log::info!("[SiliconFlow] 文本响应体: {}", truncated_response);
        save_raw_response("siliconflow", "text_gen", &body);

        if status.is_success() {
            let gen_response: TextGenerationResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[SiliconFlow] 解析文本响应JSON失败: {} (原始响应前200字: {})", e, truncate_str(&body, 200));
                    ProviderError::GenerationFailed(format!("解析响应失败: {}", e))
                })?;
            gen_response.choices.first()
                .and_then(|c| c.message.content.clone())
                .ok_or_else(|| {
                    log::error!("[SiliconFlow] 文本响应中没有内容");
                    ProviderError::GenerationFailed("响应中没有文本内容".to_string())
                })
        } else {
            self.handle_error_status(status.as_u16(), &body)
        }
    }

    /// 下载图片
    async fn download_image(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        log::info!("[SiliconFlow] 下载图片: url={}", &url[..url.len().min(200)]);

        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| {
                log::error!("[SiliconFlow] 下载图片失败: {}", e);
                ProviderError::NetworkError(format!("下载图片失败: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            log::error!("[SiliconFlow] 下载图片响应错误: status={}", status);
            return Err(ProviderError::NetworkError(format!("下载失败，状态码: {}", status)));
        }

        let data = response.bytes().await
            .map(|b| b.to_vec())
            .map_err(|e| {
                log::error!("[SiliconFlow] 读取图片数据失败: {}", e);
                ProviderError::NetworkError(format!("读取图片数据失败: {}", e))
            })?;

        log::info!("[SiliconFlow] 图片下载完成: size={}KB", data.len() / 1024);
        Ok(data)
    }

    /// 根据 AssetRef 构造 Prompt
    /// 返回 (prompt, negative_prompt, image_size)
    pub fn build_image_prompt(&self, asset_ref: &AssetRef) -> (String, Option<String>, String) {
        let size = self.infer_image_size(asset_ref);

        let style_prefix = asset_ref.style.as_deref().unwrap_or("").trim();
        let base_prompt = asset_ref.prompt.trim();

        let prompt = match asset_ref.asset_type {
            crate::types::game_script::AssetType::Image => {
                // 根据用途推断是背景图还是头像
                let is_avatar = asset_ref.id.contains("avatar")
                    || asset_ref.id.contains("portrait")
                    || asset_ref.id.contains("head")
                    || asset_ref.prompt.to_lowercase().contains("portrait")
                    || asset_ref.prompt.to_lowercase().contains("avatar");

                if is_avatar {
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
                }
            }
            crate::types::game_script::AssetType::Video => {
                let mut parts = Vec::new();
                if !style_prefix.is_empty() {
                    parts.push(style_prefix.to_string());
                }
                parts.push(base_prompt.to_string());
                parts.push("cinematic".to_string());
                parts.push("dramatic lighting".to_string());
                parts.push("high quality".to_string());
                parts.join(", ")
            }
            _ => {
                let mut parts = Vec::new();
                if !style_prefix.is_empty() {
                    parts.push(style_prefix.to_string());
                }
                parts.push(base_prompt.to_string());
                parts.push("high quality".to_string());
                parts.join(", ")
            }
        };

        let negative_prompt = asset_ref.negative_prompt.clone();

        (prompt, negative_prompt, size)
    }

    /// 从 AssetRef 推断图片尺寸
    fn infer_image_size(&self, asset_ref: &AssetRef) -> String {
        let is_avatar = asset_ref.id.contains("avatar")
            || asset_ref.id.contains("portrait")
            || asset_ref.id.contains("head")
            || asset_ref.prompt.to_lowercase().contains("portrait")
            || asset_ref.prompt.to_lowercase().contains("avatar");

        if is_avatar {
            "1024x1024".to_string() // 1:1
        } else {
            "1024x576".to_string() // 16:9
        }
    }

    fn handle_error_status(&self, status_code: u16, body: &str) -> Result<String, ProviderError> {
        let error_msg = serde_json::from_str::<ApiError>(body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or_else(|| body.to_string());

        log::error!("[SiliconFlow] API错误: status={}, message={}", status_code, error_msg);

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
impl IAssetProvider for SiliconFlowProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        let (prompt, negative_prompt, image_size) = self.build_image_prompt(asset_ref);

        let image_data = self.generate_image(
            &prompt,
            negative_prompt.as_deref(),
            Some(&image_size),
            None,
        )
        .await?;

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

    async fn check_connectivity(&self) -> Result<ConnectivityCheck, ProviderError> {
        self.check_connectivity_with_prompt("a beautiful sunset over mountains, digital art").await
    }

    async fn check_connectivity_with_prompt(&self, prompt: &str) -> Result<ConnectivityCheck, ProviderError> {
        let start = SystemTime::now();

        // 非空且不超长时使用用户输入的提示词，否则使用默认提示词
        let test_prompt = if !prompt.trim().is_empty() && prompt.len() <= 2000 {
            prompt.trim()
        } else {
            "a beautiful sunset over mountains, digital art"
        };

        // 找到当前选中的模型（is_default=true）
        let selected_model = self.config.models.iter().find(|m| m.is_default)
            .or_else(|| self.config.models.first());

        // 根据模型的 modality 决定调用哪种 API
        let modality = selected_model.map(|m| m.modality.clone()).unwrap_or(AIModality::Image);
        let text_endpoint = "https://api.siliconflow.cn/v1/chat/completions";

        if modality == AIModality::Text {
            // 文本模型：调用 /chat/completions 端点
            let model_id = selected_model.map(|m| m.id.clone())
                .unwrap_or_else(|| self.default_text_model.clone());
            
            let request = TextGenerationRequest {
                model: model_id.clone(),
                messages: vec![
                    ChatMessage {
                        role: "user".to_string(),
                        content: test_prompt.to_string(),
                    }
                ],
                max_tokens: Some(100),
                temperature: Some(0.7),
            };

            let request_body = serde_json::to_string(&request).unwrap_or_default();
            let request_headers = r#"{"Authorization":"Bearer ***","Content-Type":"application/json"}"#.to_string();

            let result = self.send_text_request(&request).await;
            let latency = SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_millis() as u64;

            match result {
                Ok(text_response) => {
                    // 截断过长的响应
                    let response_preview = if text_response.len() > 500 {
                        format!("{}...(共{}字符)", &text_response[..500], text_response.len())
                    } else {
                        text_response
                    };
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
                        response_preview: Some(response_preview),
                        test_prompt: Some(test_prompt.to_string()),
                        media_url: None,
                        media_type: None,
                        polling_task_id: None,
                        polling_status: None,
                        polling_elapsed_secs: None,
                        media_items: None,
                        request_endpoint: Some(text_endpoint.to_string()),
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
                        request_endpoint: Some(text_endpoint.to_string()),
                        request_model: None,
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
                        request_endpoint: Some(text_endpoint.to_string()),
                        request_model: None,
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
                        request_endpoint: Some(text_endpoint.to_string()),
                        request_model: None,
                        request_headers: Some(request_headers),
                        request_body: Some(truncate_str(&request_body, 2000).to_string()),
                        response_status: None,
                }),
            }
        } else {
            // 图片模型：调用 /images/generations 端点（原有逻辑）
            let (num_inference_steps, guidance_scale, seed) = self.get_advanced_params(&self.default_image_model);

            let request = ImageGenerationRequest {
                model: self.default_image_model.clone(),
                prompt: test_prompt.to_string(),
                negative_prompt: None,
                image_size: Some("512x512".to_string()),
                num_inference_steps,
                guidance_scale,
                seed,
            };

            let request_body = serde_json::to_string(&request).unwrap_or_default();
            let request_headers = r#"{"Authorization":"Bearer ***","Content-Type":"application/json"}"#.to_string();

            let result = self.send_image_request(&request).await;
            let latency = SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_millis() as u64;

            match result {
                Ok(image_url) => {
                    // 下载并保存测试图片
                    let media_url = self.download_and_save_test_image(&image_url, "siliconflow").await.ok();
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
                    request_endpoint: Some(self.endpoint.clone()),
                    request_model: Some(self.default_image_model.clone()),
                    request_headers: Some(request_headers.clone()),
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
                request_endpoint: Some(self.endpoint.clone()),
                request_model: Some(self.default_image_model.clone()),
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
                    test_prompt: Some(test_prompt.to_string()),
                    media_url: None,
                media_type: None,
                polling_task_id: None,
                polling_status: None,
                polling_elapsed_secs: None,
                media_items: None,
                request_endpoint: Some(self.endpoint.clone()),
                request_model: Some(self.default_image_model.clone()),
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
                    test_prompt: Some(test_prompt.to_string()),
                    media_url: None,
                media_type: None,
                polling_task_id: None,
                polling_status: None,
                polling_elapsed_secs: None,
                media_items: None,
                request_endpoint: Some(self.endpoint.clone()),
                request_model: Some(self.default_image_model.clone()),
                request_headers: Some(request_headers),
                request_body: Some(truncate_str(&request_body, 2000).to_string()),
                response_status: None,
                }),
            }
        }
    }

    fn supported_modalities(&self) -> Vec<AIModality> {
        vec![AIModality::Text, AIModality::Image]
    }

    fn provider_id(&self) -> &str {
        &self.config.id
    }
}

impl SiliconFlowProvider {
    /// 下载测试图片并保存到 gen/cache/ 目录，返回本地文件路径
    async fn download_and_save_test_image(&self, url: &str, provider_name: &str) -> Result<String, ProviderError> {
        let image_data = self.download_image(url).await?;

        let cache_dir = self.asset_base_path.join("cache");
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| ProviderError::GenerationFailed(format!("创建缓存目录失败: {}", e)))?;

        let filename = format!("{}_test_{}.png", provider_name,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs());
        let dest_path = cache_dir.join(&filename);
        std::fs::write(&dest_path, &image_data)
            .map_err(|e| ProviderError::GenerationFailed(format!("写入测试图片失败: {}", e)))?;

        Ok(dest_path.to_string_lossy().to_string())
    }
}
