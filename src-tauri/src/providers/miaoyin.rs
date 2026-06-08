use async_trait::async_trait;
use super::{truncate_str, save_raw_response, IAssetProvider, ProviderError};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_BASE_URL: &str = "https://ai.growingth.com/api";
const DEFAULT_MODEL: &str = "moka-v9";
const POLL_INTERVAL_SECS: u64 = 5;
const MAX_POLL_DURATION_SECS: u64 = 300; // 5 minutes

#[derive(Debug, Serialize)]
struct MusicGenerationRequest {
    prompt: String,
    desc: Option<String>,
    make_instrumental: bool,
}

#[derive(Debug, Deserialize)]
struct MusicGenerationResponse {
    status: String,
    message: String,
    #[serde(default)]
    data: Option<Vec<TaskData>>,
}

#[derive(Debug, Deserialize)]
struct TaskData {
    #[serde(alias = "taskId")]
    task_id: Option<String>,
    #[serde(alias = "clipId")]
    clip_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    status: Option<String>,
    message: Option<String>,
}

pub struct MiaoYinProvider {
    config: AIProviderConfig,
    client: Client,
    api_token: String,
    default_model: String,
    base_url: String,
    asset_base_path: PathBuf,
}

impl MiaoYinProvider {
    pub fn new(config: &AIProviderConfig, asset_base_path: PathBuf) -> Result<Self, ProviderError> {
        let api_token = config.auth_config.api_key
            .as_ref()
            .map(|k| k.value.clone())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProviderError::InvalidConfig("妙音AI API Token 未配置".to_string()))?;

        let default_model = config.models
            .iter()
            .find(|m| m.is_default && m.modality == AIModality::Music)
            .map(|m| m.id.clone())
            .or_else(|| config.models.iter().find(|m| m.modality == AIModality::Music).map(|m| m.id.clone()))
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());

        let base_url = config.models
            .iter()
            .find(|m| m.is_default)
            .and_then(|m| {
                let ep = m.endpoint.trim().to_string();
                if ep.is_empty() { None } else { Some(ep) }
            })
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::NetworkError(format!("创建HTTP客户端失败: {}", e)))?;

        Ok(Self {
            config: config.clone(),
            client,
            api_token,
            default_model,
            base_url,
            asset_base_path,
        })
    }

    /// 生成音乐
    pub async fn generate_music(
        &self,
        prompt: &str,
        make_instrumental: bool,
    ) -> Result<Vec<u8>, ProviderError> {
        let request = MusicGenerationRequest {
            prompt: prompt.to_string(),
            desc: None,
            make_instrumental,
        };

        let url = format!("{}/song/proxy?name=generate", self.base_url);
        log::info!("[MiaoYin] 请求: url={}, model={}, prompt_len={}, instrumental={}", 
            url, self.default_model, prompt.len(), make_instrumental);

        let response = self.client
            .post(&url)
            .header("api-token", &self.api_token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[MiaoYin] 请求发送失败: {} (is_timeout={}, is_connect={}, is_request={})", 
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
        log::info!("[MiaoYin] 响应状态: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

        let body = response.text().await
            .map_err(|e| {
                log::error!("[MiaoYin] 读取响应体失败: {}", e);
                ProviderError::NetworkError(format!("读取响应失败: {}", e))
            })?;

        let truncated_response = if body.len() > 1000 {
            format!("{}...(共{}字符)", truncate_str(&body, 1000), body.len())
        } else {
            body.clone()
        };
        log::info!("[MiaoYin] 响应体: {}", truncated_response);
        save_raw_response("miaoyin", "music_gen", &body);

        if !status.is_success() {
            return Err(self.handle_error_status(status.as_u16(), &body));
        }

        let gen_response: MusicGenerationResponse = serde_json::from_str(&body)
            .map_err(|e| {
                log::error!("[MiaoYin] 解析响应JSON失败: {} (原始响应前200字: {})", e, truncate_str(&body, 200));
                ProviderError::GenerationFailed(format!("解析响应失败: {}", e))
            })?;

        if gen_response.status != "Success" {
            return Err(ProviderError::GenerationFailed(
                gen_response.message
            ));
        }

        let task_data = gen_response.data
            .and_then(|d| d.into_iter().next())
            .ok_or_else(|| {
                log::error!("[MiaoYin] 响应中未找到任务数据");
                ProviderError::GenerationFailed("响应中未找到任务数据".to_string())
            })?;

        let task_id = task_data.task_id.or(task_data.clip_id)
            .ok_or_else(|| {
                log::error!("[MiaoYin] 响应中未找到 taskId 或 clipId");
                ProviderError::GenerationFailed("响应中未找到 taskId 或 clipId".to_string())
            })?;

        log::info!("[MiaoYin] 任务已提交: task_id={}", task_id);

        // 轮询等待生成完成
        let audio_url = self.poll_until_complete(&task_id).await?;

        // 下载音频
        self.download_audio(&audio_url).await
    }

    /// 异步轮询等待生成完成
    async fn poll_until_complete(&self, task_id: &str) -> Result<String, ProviderError> {
        let url = format!("{}/song/proxy?name=getMusic", self.base_url);
        let start = SystemTime::now();

        loop {
            let elapsed = SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_secs();

            if elapsed >= MAX_POLL_DURATION_SECS {
                log::error!("[MiaoYin] 轮询超时: task_id={}, elapsed={}s", task_id, elapsed);
                return Err(ProviderError::Timeout(format!(
                    "音乐生成超时 ({}秒)，任务ID: {}", MAX_POLL_DURATION_SECS, task_id
                )));
            }

            // 查询间隔不低于10秒
            tokio::time::sleep(Duration::from_secs(10)).await;

            log::info!("[MiaoYin] 轮询状态: task_id={}, elapsed={}s", task_id, elapsed);

            #[derive(Debug, Serialize)]
            struct QueryRequest {
                ids: String,
            }

            let request = QueryRequest {
                ids: task_id.to_string(),
            };

            let response = self.client
                .post(&url)
                .header("api-token", &self.api_token)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| {
                    log::error!("[MiaoYin] 轮询请求失败: {}", e);
                    ProviderError::NetworkError(format!("轮询请求失败: {}", e))
                })?;

            let status = response.status();
            let body = response.text().await
                .map_err(|e| {
                    log::error!("[MiaoYin] 读取轮询响应失败: {}", e);
                    ProviderError::NetworkError(format!("读取轮询响应失败: {}", e))
                })?;

            let truncated = if body.len() > 500 {
                format!("{}...(共{}字符)", truncate_str(&body, 500), body.len())
            } else {
                body.clone()
            };
            log::info!("[MiaoYin] 轮询响应: status={}, body={}", status.as_u16(), truncated);
            save_raw_response("miaoyin", "music_gen_query", &body);

            if !status.is_success() {
                return Err(self.handle_error_status(status.as_u16(), &body));
            }

            // 解析轮询响应，查找音频URL
            // 响应格式: { status: "Success", data: [{ audio_url: "...", status: "complete", ... }] }
            #[derive(Debug, Deserialize)]
            struct QueryResponse {
                status: String,
                message: Option<String>,
                #[serde(default)]
                data: Option<Vec<QueryTaskData>>,
            }

            #[derive(Debug, Deserialize)]
            struct QueryTaskData {
                id: String,
                #[serde(alias = "audio_url")]
                audio_url: Option<String>,
                #[serde(alias = "video_url")]
                video_url: Option<String>,
                status: Option<String>,
                #[serde(default)]
                error: Option<String>,
            }

            let query_response: QueryResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[MiaoYin] 解析轮询响应失败: {}", e);
                    ProviderError::GenerationFailed(format!("解析轮询响应失败: {}", e))
                })?;

            if query_response.status == "Success" {
                if let Some(data) = query_response.data {
                    for task in data {
                        if task.id == task_id {
                            if task.status.as_deref() == Some("complete") {
                                if let Some(audio_url) = task.audio_url {
                                    if !audio_url.is_empty() {
                                        log::info!("[MiaoYin] 音乐生成完成: task_id={}", task_id);
                                        return Ok(audio_url);
                                    }
                                }
                                // 如果没有 audio_url 但有 video_url，尝试使用 video_url
                                if let Some(video_url) = task.video_url {
                                    if !video_url.is_empty() {
                                        log::info!("[MiaoYin] 音乐生成完成(使用video_url): task_id={}", task_id);
                                        return Ok(video_url);
                                    }
                                }
                                return Err(ProviderError::GenerationFailed("音频生成完成但未返回音频URL".to_string()));
                            } else if task.status.as_deref() == Some("failed") {
                                return Err(ProviderError::GenerationFailed(
                                    task.error.unwrap_or_else(|| "音乐生成失败".to_string())
                                ));
                            }
                            // pending 或其他状态，继续轮询
                            log::info!("[MiaoYin] 任务进行中: task_id={}, status={:?}", task_id, task.status);
                            break;
                        }
                    }
                }
            } else if let Some(msg) = query_response.message {
                if msg.contains("failed") || msg.contains("error") {
                    return Err(ProviderError::GenerationFailed(msg));
                }
            }
        }
    }

    /// 下载音频文件
    async fn download_audio(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        log::info!("[MiaoYin] 下载音频: url={}", &url[..url.len().min(200)]);

        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| {
                log::error!("[MiaoYin] 下载音频失败: {}", e);
                ProviderError::NetworkError(format!("下载音频失败: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            log::error!("[MiaoYin] 下载音频响应错误: status={}", status);
            return Err(ProviderError::NetworkError(format!(
                "音频下载失败，状态码: {}", status
            )));
        }

        let data = response.bytes().await
            .map(|b| b.to_vec())
            .map_err(|e| {
                log::error!("[MiaoYin] 读取音频数据失败: {}", e);
                ProviderError::NetworkError(format!("读取音频数据失败: {}", e))
            })?;

        log::info!("[MiaoYin] 音频下载完成: size={}KB", data.len() / 1024);
        Ok(data)
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
        let error_msg = serde_json::from_str::<ApiErrorResponse>(body)
            .ok()
            .and_then(|e| e.message)
            .unwrap_or_else(|| body.to_string());

        log::error!("[MiaoYin] API错误: status={}, message={}", status_code, error_msg);

        match status_code {
            401 | 403 => ProviderError::AuthFailed(format!("认证失败: {}", error_msg)),
            429 => ProviderError::QuotaExceeded(format!("请求频率超限: {}", error_msg)),
            s if s >= 500 => ProviderError::GenerationFailed(format!("服务器错误 ({}): {}", status_code, error_msg)),
            _ => ProviderError::GenerationFailed(format!("API错误 ({}): {}", status_code, error_msg)),
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
impl IAssetProvider for MiaoYinProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        let prompt = self.build_music_prompt(asset_ref);
        let style = self.infer_style(&prompt);
        
        // 构建完整提示词
        let full_prompt = if style.is_empty() {
            prompt
        } else {
            format!("{}, {}", prompt, style)
        };

        // 妙音AI默认生成带人声的歌曲，BGM场景需要纯音乐
        let audio_data = self.generate_music(&full_prompt, true).await?;

        let cache_key = asset_ref.cache_key.clone()
            .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

        let dest_dir = self.asset_base_path.join("cacheAssets").join(&cache_key);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| ProviderError::GenerationFailed(format!("创建资源目录失败: {}", e)))?;

        let dest_path = dest_dir.join(format!("{}.mp3", asset_ref.id));
        std::fs::write(&dest_path, &audio_data)
            .map_err(|e| ProviderError::GenerationFailed(format!("写入音频文件失败: {}", e)))?;

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
        let url = format!("{}/song/proxy?name=generate", self.base_url);
        let request = MusicGenerationRequest {
            prompt: "test".to_string(),
            desc: Some("connectivity test".to_string()),
            make_instrumental: true,
        };

        let result = self.client
            .post(&url)
            .header("api-token", &self.api_token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await;

        let latency = SystemTime::now()
            .duration_since(start)
            .unwrap_or_default()
            .as_millis() as u64;

        match result {
            Ok(response) => {
                let status = response.status();
                if status.is_success() || status.as_u16() == 400 {
                    // 400 可能是因为参数不完整，但说明 API 可达
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
                        response_preview: Some("妙音AI服务可用".to_string()),
                        test_prompt: None,
                        media_url: None,
                        media_type: None,
                        request_endpoint: None,
                        request_model: None,
                        request_headers: None,
                        request_body: None,
                        response_status: None,
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
                        error_message: Some("认证失败".to_string()),
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
                        error_message: Some("请求频率超限".to_string()),
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
                        error_message: Some(format!("未知状态: {}", status)),
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
                    error_message: Some(format!("网络错误: {}", e)),
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
