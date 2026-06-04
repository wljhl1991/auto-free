use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::asset::AIModality;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIProviderConfig {
    pub id: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
