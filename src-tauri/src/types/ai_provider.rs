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
