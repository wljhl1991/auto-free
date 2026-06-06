use async_trait::async_trait;
use super::{truncate_str, save_raw_response, IAssetProvider, ProviderError};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_ENDPOINT: &str = "https://api.klingai.com/v1/videos/generations";
const DEFAULT_MODEL: &str = "kling-3.0";
const POLL_INTERVAL_SECS: u64 = 10;
const MAX_POLL_DURATION_SECS: u64 = 600; // 10 minutes

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
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct VideoGenerationResponse {
    code: i32,
    message: String,
    data: Option<VideoData>,
}

#[derive(Debug, Deserialize)]
struct VideoData {
    task_id: String,
    task_status: String,
    task_result: Option<TaskResult>,
}

#[derive(Debug, Deserialize)]
struct TaskResult {
    videos: Vec<VideoInfo>,
}

#[derive(Debug, Deserialize, Clone)]
struct VideoInfo {
    url: String,
    duration: String,
}

#[derive(Debug, Deserialize)]
struct TaskStatusResponse {
    code: i32,
    message: String,
    data: VideoData,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    error: Option<ApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: Option<String>,
}

pub struct KlingProvider {
    config: AIProviderConfig,
    client: Client,
    access_key: String,
    secret_key: String,
    default_model: String,
    endpoint: String,
    asset_base_path: PathBuf,
}

impl KlingProvider {
    pub fn new(config: &AIProviderConfig, asset_base_path: PathBuf) -> Result<Self, ProviderError> {
        let access_key = config.auth_config.extra_params
            .as_ref()
            .and_then(|p| p.get("access_key"))
            .map(|f| f.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("Kling Access Key 未配置".to_string()))?;

        let secret_key = config.auth_config.extra_params
            .as_ref()
            .and_then(|p| p.get("secret_key"))
            .map(|f| f.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("Kling Secret Key 未配置".to_string()))?;

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
            access_key,
            secret_key,
            default_model,
            endpoint,
            asset_base_path,
        })
    }

    /// 生成视频
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
            seed: None,
        };

        let task_id = self.submit_video_task(&request).await?;
        let video_info = self.poll_until_complete(&task_id).await?;
        self.download_video(&video_info.url).await
    }

    /// 提交视频生成任务
    async fn submit_video_task(&self, request: &VideoGenerationRequest) -> Result<String, ProviderError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let path = self.extract_path(&self.endpoint);
        let signature = self.generate_signature("POST", &path, timestamp);

        log::info!("[Kling] 提交视频任务: endpoint={}, model={}, prompt_len={}, timestamp={}", 
            self.endpoint, request.model, request.prompt.len(), timestamp);

        let response = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("X-Access-Key", &self.access_key)
            .header("X-Signature", &signature)
            .header("X-Timestamp", timestamp.to_string())
            .json(request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[Kling] 请求发送失败: {} (is_timeout={}, is_connect={}, is_request={})", 
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
        log::info!("[Kling] 响应状态: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

        let body = response.text().await
            .map_err(|e| {
                log::error!("[Kling] 读取响应体失败: {}", e);
                ProviderError::NetworkError(format!("读取响应失败: {}", e))
            })?;

        let truncated_response = if body.len() > 1000 {
            format!("{}...(共{}字符)", truncate_str(&body, 1000), body.len())
        } else {
            body.clone()
        };
        log::info!("[Kling] 响应体: {}", truncated_response);
        save_raw_response("kling", "gen", &body);

        if status.is_success() {
            let gen_response: VideoGenerationResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[Kling] 解析响应JSON失败: {} (原始响应前200字: {})", e, truncate_str(&body, 200));
                    ProviderError::GenerationFailed(format!("解析响应失败: {}", e))
                })?;

            if gen_response.code != 0 {
                log::error!("[Kling] API返回错误码: code={}, message={}", gen_response.code, gen_response.message);
                return Err(ProviderError::GenerationFailed(format!(
                    "API错误 (code={}): {}", gen_response.code, gen_response.message
                )));
            }

            let task_id = gen_response.data
                .map(|d| d.task_id)
                .ok_or_else(|| {
                    log::error!("[Kling] 响应中没有task_id");
                    ProviderError::GenerationFailed("响应中没有task_id".to_string())
                })?;
            log::info!("[Kling] 任务已提交: task_id={}", task_id);
            Ok(task_id)
        } else {
            self.handle_error_status(status.as_u16(), &body)
        }
    }

    /// 异步轮询等待生成完成
    async fn poll_until_complete(&self, task_id: &str) -> Result<VideoInfo, ProviderError> {
        let poll_url = format!("{}/{}", self.endpoint.trim_end_matches('/'), task_id);
        let path = self.extract_path(&poll_url);
        let start = SystemTime::now();

        loop {
            let elapsed = SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_secs();

            if elapsed >= MAX_POLL_DURATION_SECS {
                log::error!("[Kling] 轮询超时: task_id={}, elapsed={}s", task_id, elapsed);
                return Err(ProviderError::Timeout(format!(
                    "视频生成超时 ({}秒)，任务ID: {}", elapsed, task_id
                )));
            }

            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            let signature = self.generate_signature("GET", &path, timestamp);

            log::info!("[Kling] 轮询状态: task_id={}, elapsed={}s", task_id, elapsed);

            let response = self.client
                .get(&poll_url)
                .header("X-Access-Key", &self.access_key)
                .header("X-Signature", &signature)
                .header("X-Timestamp", timestamp.to_string())
                .send()
                .await
                .map_err(|e| {
                    log::error!("[Kling] 轮询请求失败: {}", e);
                    ProviderError::NetworkError(format!("轮询请求失败: {}", e))
                })?;

            let status = response.status();
            let body = response.text().await
                .map_err(|e| {
                    log::error!("[Kling] 读取轮询响应失败: {}", e);
                    ProviderError::NetworkError(format!("读取轮询响应失败: {}", e))
                })?;

            let truncated = if body.len() > 500 {
                format!("{}...(共{}字符)", truncate_str(&body, 500), body.len())
            } else {
                body.clone()
            };
            log::info!("[Kling] 轮询响应: status={}, body={}", status.as_u16(), truncated);
            super::save_raw_response("kling", "query", &body);

            if !status.is_success() {
                return self.handle_error_status(status.as_u16(), &body);
            }

            let poll_response: TaskStatusResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[Kling] 解析轮询响应失败: {}", e);
                    ProviderError::GenerationFailed(format!("解析轮询响应失败: {}", e))
                })?;

            if poll_response.code != 0 {
                log::error!("[Kling] 轮询API错误: code={}, message={}", poll_response.code, poll_response.message);
                return Err(ProviderError::GenerationFailed(format!(
                    "轮询API错误 (code={}): {}", poll_response.code, poll_response.message
                )));
            }

            match poll_response.data.task_status.as_str() {
                "succeed" => {
                    log::info!("[Kling] 视频生成完成: task_id={}", task_id);
                    let result = poll_response.data.task_result
                        .ok_or_else(|| {
                            log::error!("[Kling] 任务成功但无结果数据");
                            ProviderError::GenerationFailed("任务成功但无结果数据".to_string())
                        })?;
                    let video = result.videos.first()
                        .ok_or_else(|| {
                            log::error!("[Kling] 结果中没有视频数据");
                            ProviderError::GenerationFailed("结果中没有视频数据".to_string())
                        })?;
                    log::info!("[Kling] 视频URL: {}, duration={}", &video.url[..video.url.len().min(200)], video.duration);
                    return Ok(video.clone());
                }
                "failed" => {
                    log::error!("[Kling] 视频生成失败: task_id={}", task_id);
                    return Err(ProviderError::GenerationFailed(format!(
                        "视频生成失败，任务ID: {}", task_id
                    )));
                }
                // "submitted" | "processing" => continue polling
                _ => {
                    tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                }
            }
        }
    }

    /// 下载视频文件
    async fn download_video(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        log::info!("[Kling] 下载视频: url={}", &url[..url.len().min(200)]);

        let response = self.client
            .get(url)
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| {
                log::error!("[Kling] 下载视频失败: {}", e);
                ProviderError::NetworkError(format!("下载视频失败: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            log::error!("[Kling] 下载视频响应错误: status={}", status);
            return Err(ProviderError::NetworkError(format!("下载失败，状态码: {}", status)));
        }

        let data = response.bytes().await
            .map(|b| b.to_vec())
            .map_err(|e| {
                log::error!("[Kling] 读取视频数据失败: {}", e);
                ProviderError::NetworkError(format!("读取视频数据失败: {}", e))
            })?;

        log::info!("[Kling] 视频下载完成: size={}MB", data.len() / (1024 * 1024));
        Ok(data)
    }

    /// 构造视频 Prompt
    /// CG 动画：{场景描述}, cinematic, {镜头运动}, {氛围}, 4s-6s
    /// 动态背景：{场景描述}, slow pan, ambient, looping, 4s
    pub fn build_video_prompt(&self, asset_ref: &AssetRef) -> (String, Option<String>) {
        let style_prefix = asset_ref.style.as_deref().unwrap_or("").trim();
        let base_prompt = asset_ref.prompt.trim();

        let is_cg = asset_ref.id.contains("cg")
            || asset_ref.id.contains("animation")
            || asset_ref.id.contains("cutscene")
            || asset_ref.prompt.to_lowercase().contains("cinematic")
            || asset_ref.prompt.to_lowercase().contains("animation");

        let prompt = if is_cg {
            // CG 动画风格
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
            // 动态背景风格
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

    /// 从 AssetRef 推断视频参数
    fn infer_video_params(&self, asset_ref: &AssetRef) -> (String, String) {
        let is_cg = asset_ref.id.contains("cg")
            || asset_ref.id.contains("animation")
            || asset_ref.id.contains("cutscene");

        let duration = if is_cg { "5".to_string() } else { "5".to_string() };

        let is_portrait = asset_ref.id.contains("portrait")
            || asset_ref.id.contains("vertical")
            || asset_ref.prompt.to_lowercase().contains("portrait")
            || asset_ref.prompt.to_lowercase().contains("vertical");

        let aspect_ratio = if is_portrait {
            "9:16".to_string()
        } else {
            "16:9".to_string()
        };

        (duration, aspect_ratio)
    }

    /// 生成签名（可灵 API 需要 Access Key + Secret Key 签名）
    /// 签名内容：method + path + timestamp
    /// 密钥：secret_key
    fn generate_signature(&self, method: &str, path: &str, timestamp: i64) -> String {
        let message = format!("{}{}{}", method, path, timestamp);
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        let result = mac.finalize();
        let code_bytes = result.into_bytes();
        hex::encode(code_bytes)
    }

    /// 从完整 URL 中提取路径部分
    fn extract_path(&self, url: &str) -> String {
        // 简单提取：找到第三个 / 之后的部分
        // https://api.klingai.com/v1/videos/generations -> /v1/videos/generations
        let mut slash_count = 0;
        for (i, c) in url.char_indices() {
            if c == '/' {
                slash_count += 1;
                if slash_count == 3 {
                    return url[i..].to_string();
                }
            }
        }
        "/v1/videos/generations".to_string()
    }

    fn handle_error_status<T>(&self, status_code: u16, body: &str) -> Result<T, ProviderError> {
        let error_msg = serde_json::from_str::<ApiError>(body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or_else(|| body.to_string());

        log::error!("[Kling] API错误: status={}, message={}", status_code, error_msg);

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
impl IAssetProvider for KlingProvider {
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
        // 发送一个极简请求来验证认证
        let request = VideoGenerationRequest {
            model: self.default_model.clone(),
            prompt: "a single frame".to_string(),
            negative_prompt: None,
            duration: Some("5".to_string()),
            aspect_ratio: Some("16:9".to_string()),
            seed: Some(42),
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
