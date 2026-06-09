use async_trait::async_trait;
use super::{truncate_str, save_raw_response, IAssetProvider, ProviderError};
use crate::types::game_script::AssetRef;
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus, MediaItem};
use base64::Engine;
use reqwest::Client;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 妙音 AI 查询返回的单条音乐数据（直接从 data 数组项解析）
#[derive(Debug, Clone, Deserialize)]
struct MiaoYinTaskInfo {
    #[serde(default)]
    id: Option<String>,
    #[serde(alias = "taskId", default)]
    task_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(alias = "image_url", default)]
    image_url: Option<String>,
    #[serde(alias = "audio_url", default)]
    audio_url: Option<String>,
    #[serde(alias = "stream_url", default)]
    stream_url: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    model_name: Option<String>,
    #[serde(alias = "errorMsg", default)]
    error_msg: Option<String>,
    #[serde(alias = "duration", default)]
    duration: Option<serde_json::Value>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

const DEFAULT_BASE_URL: &str = "https://ai.growingth.com/api";
const DEFAULT_MODEL: &str = "moka-v9";
const MAX_POLL_DURATION_SECS: u64 = 300; // 5 minutes

/// 妙音 AI 生成请求 - 参数需包装在 data 字段中
#[derive(Debug, Serialize)]
struct MusicGenerationRequestInner {
    prompt: String,
    desc: Option<String>,
    make_instrumental: bool,
}

#[derive(Debug, Serialize)]
struct MusicGenerationRequest {
    data: MusicGenerationRequestInner,
}

/// 妙音 AI 生成响应 - data 字段可能是数组或对象
#[derive(Debug, Deserialize)]
struct MusicGenerationResponse {
    status: String,
    message: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

/// 从响应 data 中提取任务信息（可能是数组第一项或直接是对象）
#[derive(Debug, Deserialize)]
struct TaskData {
    #[serde(alias = "taskId")]
    task_id: Option<String>,
    #[serde(alias = "clipId")]
    clip_id: Option<String>,
}

/// 妙音 AI 查询请求 - 参数需包装在 data 字段中
#[derive(Debug, Serialize)]
struct QueryRequestInner {
    ids: String,
}

#[derive(Debug, Serialize)]
struct QueryRequest {
    data: QueryRequestInner,
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
        // 妙音 AI API 要求将参数包装在 data 字段中
        let request = MusicGenerationRequest {
            data: MusicGenerationRequestInner {
                prompt: prompt.to_string(),
                desc: None,
                make_instrumental,
            },
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

        // data 字段可能是数组、对象或 null，需要灵活处理
        let task_id = Self::extract_task_id(gen_response.data)?;

        log::info!("[MiaoYin] 任务已提交: task_id={}", task_id);

        // 轮询等待生成完成
        let audio_url = self.poll_until_complete(&task_id).await?;

        // 下载音频
        self.download_audio(&audio_url).await
    }

    /// 从响应的 data 字段中提取 task_id - 支持数组或对象格式
    fn extract_task_id(data: Option<serde_json::Value>) -> Result<String, ProviderError> {
        let data_value = match data {
            Some(d) => d,
            None => {
                log::error!("[MiaoYin] 响应中 data 字段为 null");
                return Err(ProviderError::GenerationFailed("响应中未找到任务数据".to_string()));
            }
        };

        // 先尝试按数组解析
        if let Some(arr) = data_value.as_array() {
            if let Some(first) = arr.first() {
                return Self::extract_task_id_from_json(first);
            }
            log::error!("[MiaoYin] data 数组为空");
            return Err(ProviderError::GenerationFailed("响应中 data 数组为空".to_string()));
        }

        // 再尝试按对象解析
        if data_value.is_object() {
            return Self::extract_task_id_from_json(&data_value);
        }

        log::error!("[MiaoYin] data 字段格式无法解析: {:?}", data_value);
        Err(ProviderError::GenerationFailed("响应中 data 格式不正确".to_string()))
    }

    /// 从单个 JSON 对象中提取 task_id
    fn extract_task_id_from_json(value: &serde_json::Value) -> Result<String, ProviderError> {
        // 尝试解析为 TaskData
        let task: TaskData = serde_json::from_value(value.clone())
            .map_err(|e| {
                log::error!("[MiaoYin] 解析任务数据失败: {}", e);
                ProviderError::GenerationFailed("解析任务数据失败".to_string())
            })?;

        task.task_id.or(task.clip_id)
            .ok_or_else(|| {
                log::error!("[MiaoYin] 响应中未找到 taskId 或 clipId");
                ProviderError::GenerationFailed("响应中未找到 taskId 或 clipId".to_string())
            })
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

            // 妙音 AI API 要求将参数包装在 data 字段中
            let request = QueryRequest {
                data: QueryRequestInner {
                    ids: task_id.to_string(),
                },
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

            // 解析轮询响应 - data 可能是数组或对象，需要灵活处理
            #[derive(Debug, Deserialize)]
            struct QueryResponse {
                status: String,
                message: Option<String>,
                #[serde(default)]
                data: Option<serde_json::Value>,
            }

            #[derive(Debug, Deserialize)]
            struct QueryTaskData {
                #[serde(alias = "id")]
                #[serde(default)]
                id: Option<String>,
                #[serde(alias = "audio_url")]
                audio_url: Option<String>,
                #[serde(alias = "video_url")]
                video_url: Option<String>,
                status: Option<String>,
                #[serde(default)]
                error: Option<String>,
            }

            // 清理URL中可能包含的反引号
            fn clean_url(url: &str) -> String {
                url.trim_matches('`').to_string()
            }

            let query_response: QueryResponse = serde_json::from_str(&body)
                .map_err(|e| {
                    log::error!("[MiaoYin] 解析轮询响应失败: {}", e);
                    ProviderError::GenerationFailed(format!("解析轮询响应失败: {}", e))
                })?;

            if query_response.status == "Success" {
                // data 可能是数组或对象，灵活处理
                let tasks: Vec<QueryTaskData> = match query_response.data {
                    Some(serde_json::Value::Array(arr)) => {
                        // 数组格式：data: [ {...}, {...} ]
                        arr.into_iter()
                            .filter_map(|v| serde_json::from_value::<QueryTaskData>(v).ok())
                            .collect()
                    }
                    Some(serde_json::Value::Object(_)) => {
                        // 对象格式：data: { "id": "...", ... }
                        if let Ok(task) = serde_json::from_value::<QueryTaskData>(query_response.data.unwrap()) {
                            vec![task]
                        } else {
                            Vec::new()
                        }
                    }
                    _ => {
                        log::warn!("[MiaoYin] 轮询响应中 data 格式不可识别");
                        Vec::new()
                    }
                };

                for task in tasks {
                    // 任务ID匹配：id 字段可能缺失（整个 data 就是任务本身）
                    let id_matches = task.id.as_deref() == Some(task_id) || task.id.is_none();
                    if id_matches {
                        if task.status.as_deref() == Some("complete") {
                            if let Some(audio_url) = task.audio_url {
                                let cleaned_url = clean_url(&audio_url);
                                if !cleaned_url.is_empty() {
                                    log::info!("[MiaoYin] 音乐生成完成: task_id={}, url={}", task_id, cleaned_url);
                                    return Ok(cleaned_url);
                                }
                            }
                            // 如果没有 audio_url 但有 video_url，尝试使用 video_url
                            if let Some(video_url) = task.video_url {
                                let cleaned_url = clean_url(&video_url);
                                if !cleaned_url.is_empty() {
                                    log::info!("[MiaoYin] 音乐生成完成(使用video_url): task_id={}, url={}", task_id, cleaned_url);
                                    return Ok(cleaned_url);
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

    /// 生成音乐并轮询结果，返回所有音乐任务的详情（封面 + 音频）
    pub async fn generate_music_with_details(
        &self,
        prompt: &str,
        make_instrumental: bool,
    ) -> Result<(String, Vec<MediaItem>, u64), ProviderError> {
        let poll_start = SystemTime::now();

        // 1. 发送生成请求（请求体包装在 data 字段）
        let request = MusicGenerationRequest {
            data: MusicGenerationRequestInner {
                prompt: prompt.to_string(),
                desc: None,
                make_instrumental,
            },
        };
        let url = format!("{}/song/proxy?name=generate", self.base_url);

        let response = self.client
            .post(&url)
            .header("api-token", &self.api_token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(format!("发送生成请求失败: {}", e)))?;

        let body = response.text().await
            .map_err(|e| ProviderError::NetworkError(format!("读取生成响应失败: {}", e)))?;
        save_raw_response("miaoyin", "music_gen", &body);

        let gen_resp: MusicGenerationResponse = serde_json::from_str(&body)
            .map_err(|e| ProviderError::GenerationFailed(format!("解析生成响应失败: {}", e)))?;

        if gen_resp.status != "Success" {
            return Err(ProviderError::GenerationFailed(gen_resp.message));
        }

        // 2. 提取 task_id（data 是数组时取第一项的 taskId）
        let task_id = Self::extract_task_id(gen_resp.data)?;
        log::info!("[MiaoYin] 任务已提交: task_id={}", task_id);

        // 3. 轮询：等待所有音乐项完成
        let poll_url = format!("{}/song/proxy?name=getMusic", self.base_url);
        let mut final_items: Vec<MediaItem> = Vec::new();
        let mut last_seen_ids: Vec<String> = Vec::new();

        loop {
            let elapsed = SystemTime::now()
                .duration_since(poll_start)
                .unwrap_or_default()
                .as_secs();

            if elapsed >= MAX_POLL_DURATION_SECS {
                log::warn!("[MiaoYin] 轮询超时: task_id={}, elapsed={}s, 使用当前已获取的结果", task_id, elapsed);
                break;
            }

            tokio::time::sleep(Duration::from_secs(10)).await;

            let poll_request = QueryRequest {
                data: QueryRequestInner {
                    ids: task_id.to_string(),
                },
            };

            let poll_response = self.client
                .post(&poll_url)
                .header("api-token", &self.api_token)
                .header("Content-Type", "application/json")
                .json(&poll_request)
                .send()
                .await;

            let poll_body = match poll_response {
                Ok(r) => r.text().await.unwrap_or_default(),
                Err(e) => {
                    log::warn!("[MiaoYin] 轮询请求失败: {}, 继续等待", e);
                    continue;
                }
            };

            save_raw_response("miaoyin", "music_gen_query", &poll_body);

            // 解析响应：data 可能是数组或对象
            let parsed: Result<MusicGenerationResponse, _> = serde_json::from_str(&poll_body);
            let data_value = match parsed {
                Ok(r) => r.data.unwrap_or(serde_json::Value::Array(vec![])),
                Err(_) => continue,
            };

            // 将 data 统一转换为 Vec<MiaoYinTaskInfo>
            let task_items: Vec<MiaoYinTaskInfo> = match data_value {
                serde_json::Value::Array(arr) => {
                    arr.into_iter()
                        .filter_map(|v| serde_json::from_value::<MiaoYinTaskInfo>(v).ok())
                        .collect()
                }
                serde_json::Value::Object(obj) => {
                    if let Ok(item) = serde_json::from_value::<MiaoYinTaskInfo>(serde_json::Value::Object(obj)) {
                        vec![item]
                    } else {
                        Vec::new()
                    }
                }
                _ => Vec::new(),
            };

            // 更新任务状态跟踪
            let current_ids: Vec<String> = task_items.iter().filter_map(|t| t.id.clone()).collect();
            if !current_ids.is_empty() {
                last_seen_ids = current_ids;
            }

            // 检查所有任务的状态
            let all_known = task_items.iter().all(|t| match t.status.as_deref() {
                Some("complete") | Some("failed") | Some("streaming") => true,
                _ => false,
            });

            // 如果已经确定最终结果（所有项都有结束状态，或空数组但之前有数据），则退出
            if !task_items.is_empty() && all_known {
                final_items = Self::task_items_to_media_items(&task_items, &task_id);
                break;
            }

            // 持续更新中间状态（最后一次轮询结果也会被使用）
            final_items = Self::task_items_to_media_items(&task_items, &task_id);
        }

        let elapsed_secs = SystemTime::now()
            .duration_since(poll_start)
            .unwrap_or_default()
            .as_secs();

        // 4. 对每个 complete 的音乐项，下载音频到本地并生成 data URL
        let mut enriched_items: Vec<MediaItem> = Vec::new();
        for item in final_items {
            if item.status.as_deref() != Some("complete") {
                // 未完成的项目直接原样保留
                enriched_items.push(item);
                continue;
            }

            // 尝试下载音频（如果有 audio_url）
            let mut enriched = item.clone();
            if let Some(audio_url) = &item.audio_url {
                match self.download_audio(audio_url).await {
                    Ok(audio_data) => {
                        // 保存到 cache 目录并生成 data URL
                        let local_path = self.save_test_audio(&audio_data, &format!("miaoyin_{}", item.id)).ok();
                        if let Some(path) = &local_path {
                            let data_url = Self::read_file_as_data_url(path);
                            enriched.local_path = Some(path.clone());
                            enriched.data_url = data_url;
                            enriched.media_url = Some(audio_url.clone());
                        }
                    }
                    Err(e) => {
                        log::warn!("[MiaoYin] 下载音频失败 id={}: {}", item.id, e);
                        enriched.error = Some(format!("下载失败: {}", e));
                    }
                }
            }
            enriched_items.push(enriched);
        }

        Ok((task_id, enriched_items, elapsed_secs))
    }

    /// 将妙音 AI 的任务数据转换为 MediaItem
    fn task_items_to_media_items(items: &[MiaoYinTaskInfo], task_id: &str) -> Vec<MediaItem> {
        items.iter().map(|t| {
            let item_id = t.id.clone().unwrap_or_else(|| format!("{}_{}", task_id, "unknown"));
            MediaItem {
                id: item_id.clone(),
                media_type: "audio".to_string(),
                title: t.title.clone().filter(|s| !s.is_empty()),
                media_url: t.audio_url.clone(),
                image_url: t.image_url.clone(),
                audio_url: t.audio_url.clone(),
                local_path: None,
                data_url: None,
                status: t.status.clone(),
                tags: t.tags.clone().filter(|s| !s.is_empty()),
                progress: None,
                error: t.error_msg.clone(),
            }
        }).collect()
    }

    /// 读取本地文件为 base64 data URL（用于前端直接播放音频）
    fn read_file_as_data_url(path: &str) -> Option<String> {
        use std::io::Read;
        let mut file = std::fs::File::open(path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&buffer);
        Some(format!("data:audio/mpeg;base64,{}", b64))
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
        self.check_connectivity_with_prompt("一首轻松的背景音乐").await
    }

    async fn check_connectivity_with_prompt(&self, prompt: &str) -> Result<ConnectivityCheck, ProviderError> {
        let start = SystemTime::now();
        let request_url = format!("{}/song/proxy?name=generate", self.base_url);
        // 妙音 AI API 要求将参数包装在 data 字段中
        let request = MusicGenerationRequest {
            data: MusicGenerationRequestInner {
                prompt: prompt.to_string(),
                desc: Some("connectivity test".to_string()),
                make_instrumental: true,
            },
        };
        let request_body = serde_json::to_string(&request).unwrap_or_default();

        // 调用新的生成+轮询方法，获取详细结果
        let result = self.generate_music_with_details(prompt, true).await;
        let latency = SystemTime::now()
            .duration_since(start)
            .unwrap_or_default()
            .as_millis() as u64;

        match result {
            Ok((task_id, media_items, elapsed_secs)) => {
                // 统计结果
                let complete_count = media_items.iter().filter(|m| m.status.as_deref() == Some("complete")).count();
                let failed_count = media_items.iter().filter(|m| m.status.as_deref() == Some("failed")).count();
                let total_kb: u64 = media_items.iter()
                    .filter_map(|m| m.data_url.as_ref().map(|d| d.len() as u64 / 1024))
                    .sum();

                let status = if complete_count > 0 {
                    ConnectivityStatus::Ok
                } else if failed_count > 0 {
                    ConnectivityStatus::UnknownError
                } else {
                    ConnectivityStatus::NetworkError
                };

                let msg = if complete_count > 0 {
                    format!("成功生成 {} 首音乐，总音频大小: {}KB，用时 {}s", complete_count, total_kb, elapsed_secs)
                } else if failed_count > 0 {
                    format!("生成失败: {} 首音乐失败", failed_count)
                } else {
                    format!("轮询超时（{}s），未能获取结果", elapsed_secs)
                };

                Ok(ConnectivityCheck {
                    provider_id: self.config.id.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    status,
                    latency: Some(latency),
                    error_message: if complete_count > 0 { None } else { Some(msg.clone()) },
                    quota_info: None,
                    response_preview: Some(msg),
                    test_prompt: Some(prompt.to_string()),
                    media_url: media_items.first().and_then(|m| m.local_path.clone()),
                    media_type: Some("audio".to_string()),
                    polling_task_id: Some(task_id),
                    polling_status: Some(if complete_count > 0 { "done".to_string() } else { "failed".to_string() }),
                    polling_elapsed_secs: Some(elapsed_secs),
                    media_items: Some(media_items),
                    request_endpoint: Some(request_url),
                    request_model: Some(self.default_model.clone()),
                    request_headers: Some(r#"{"api-token": "***"}"#.to_string()),
                    request_body: Some(truncate_str(&request_body, 2000).to_string()),
                    response_status: Some(200),
                })
            }
            Err(e) => {
                let status = match &e {
                    ProviderError::AuthFailed(_) => ConnectivityStatus::AuthFailed,
                    ProviderError::QuotaExceeded(_) => ConnectivityStatus::QuotaExceeded,
                    ProviderError::NetworkError(_) => ConnectivityStatus::NetworkError,
                    _ => ConnectivityStatus::UnknownError,
                };

                Ok(ConnectivityCheck {
                    provider_id: self.config.id.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    status,
                    latency: Some(latency),
                    error_message: Some(format!("{:?}", e)),
                    quota_info: None,
                    response_preview: None,
                    test_prompt: Some(prompt.to_string()),
                    media_url: None,
                    media_type: None,
                    polling_task_id: None,
                    polling_status: Some("failed".to_string()),
                    polling_elapsed_secs: None,
                    media_items: None,
                    request_endpoint: Some(request_url),
                    request_model: Some(self.default_model.clone()),
                    request_headers: Some(r#"{"api-token": "***"}"#.to_string()),
                    request_body: Some(truncate_str(&request_body, 2000).to_string()),
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
