use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::asset::AIModality;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    ApiKey,
    Oauth,
    Account,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Unconfigured,
    Configured,
    Connected,
    AuthFailed,
    QuotaExceeded,
    NetworkError,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityLevel {
    Fast,
    Standard,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectivityStatus {
    Ok,
    Unconfigured,
    AuthFailed,
    NetworkError,
    QuotaExceeded,
    UnknownError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyField {
    pub value: String,
    pub label: String,
    pub placeholder: String,
    pub help_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialField {
    pub value: String,
    pub label: String,
    pub placeholder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountCredentials {
    pub username: Option<CredentialField>,
    pub password: Option<CredentialField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthConfig {
    pub client_id: String,
    pub redirect_uri: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraParamField {
    pub value: String,
    pub label: String,
    pub placeholder: String,
    pub required: bool,
    pub secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub api_key: Option<ApiKeyField>,
    pub account: Option<AccountCredentials>,
    pub oauth: Option<OAuthConfig>,
    pub extra_params: Option<HashMap<String, ExtraParamField>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIModelConfig {
    pub id: String,
    pub name: String,
    pub modality: AIModality,
    pub is_default: bool,
    pub endpoint: String,
    pub max_tokens: Option<u32>,
    pub supported_sizes: Option<Vec<String>>,
    pub max_duration: Option<u32>,
    pub cost_per_call: Option<f64>,
    pub free_quota: Option<String>,
    pub quality: QualityLevel,
    /// 高级参数（如 num_inference_steps, guidance_scale, seed 等）
    /// key 为参数名，value 为参数值（字符串形式，运行时按需解析）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced_params: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIProviderConfig {
    pub id: String,
    /// 原始内置服务商类型 ID（如 "siliconflow"、"deepseek"），用于 Provider 工厂路由
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<String>,
    pub name: String,
    pub vendor: String,
    pub description: String,
    pub official_url: String,
    pub register_url: String,
    pub docs_url: String,
    pub modality: Vec<AIModality>,
    pub auth_type: AuthType,
    pub auth_config: AuthConfig,
    pub models: Vec<AIModelConfig>,
    pub status: ProviderStatus,
    pub last_checked: Option<u64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetProvider {
    pub provider_id: String,
    pub modality: AIModality,
    pub model_id: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinFallback {
    pub image: bool,
    pub video: bool,
    pub music: bool,
    pub voice: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub vendor_count: u32,
    pub providers: Vec<PresetProvider>,
    pub builtin_fallback: BuiltinFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    pub auto_retry_on_fail: bool,
    pub fallback_to_alternative: bool,
    pub max_concurrent_generations: u32,
    pub default_quality: QualityLevel,
    pub language: String,
    /// 每个模态的首选 provider ID，用户可指定文本用 DeepSeek、图片用硅基流动等
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub preferred_providers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub active_preset_id: String,
    pub providers: Vec<AIProviderConfig>,
    pub presets: Vec<ConfigPreset>,
    pub global_settings: GlobalSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaInfo {
    pub remaining: Option<f64>,
    pub total: Option<f64>,
    pub unit: String,
    pub reset_at: Option<u64>,
}

/// 单个媒体项（音乐/图片/视频等）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    /// 媒体 ID（如音乐ID或封面图片URL / 本地文件路径
    pub id: String,
    /// 媒体类型: image, audio, video
    pub media_type: String,
    /// 媒体标题
    pub title: Option<String>,
    /// 本地文件路径或远程 URL（可与 image_url/audio_url 的值）
    pub media_url: Option<String>,
    /// 封面图片URL（仅音乐项有封面）
    pub image_url: Option<String>,
    /// 音频URL（远程地址）
    pub audio_url: Option<String>,
    /// 本地缓存文件路径（本地音频/图片）
    pub local_path: Option<String>,
    /// 本地文件的 base64 data URL（前端可直接播放）
    pub data_url: Option<String>,
    /// 状态: queued, submitted, streaming, complete, failed
    pub status: Option<String>,
    /// 标签/风格（如 "epic orchestral"）
    pub tags: Option<String>,
    /// 生成进度 0-100
    pub progress: Option<u32>,
    /// 错误信息
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityCheck {
    pub provider_id: String,
    pub timestamp: u64,
    pub status: ConnectivityStatus,
    pub latency: Option<u64>,
    pub error_message: Option<String>,
    pub quota_info: Option<QuotaInfo>,
    pub response_preview: Option<String>,
    pub test_prompt: Option<String>,
    pub media_url: Option<String>,
    pub media_type: Option<String>,
    /// 轮询中的任务 ID（妙音 AI 等异步生成场景）
    #[serde(default)]
    pub polling_task_id: Option<String>,
    /// 轮询状态: polling | done | failed
    #[serde(default)]
    pub polling_status: Option<String>,
    /// 已完成的轮询秒数
    #[serde(default)]
    pub polling_elapsed_secs: Option<u64>,
    /// 多媒体项列表（妙音 AI 返回的多首音乐 + 封面）
    #[serde(default)]
    pub media_items: Option<Vec<MediaItem>>,
    /// 请求详情
    pub request_endpoint: Option<String>,
    pub request_model: Option<String>,
    pub request_headers: Option<String>,
    pub request_body: Option<String>,
    pub response_status: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct GenerationTask {
    pub id: String,
    pub asset_ref_id: String,
    pub model_provider: String,
    pub model_endpoint: String,
    pub priority: u32,
    pub dependencies: Vec<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub status: TaskStatus,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}
