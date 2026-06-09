use crate::types::ai_provider::{
    AIProviderConfig, ConnectivityCheck, ConnectivityStatus, ProviderStatus,
};
use crate::providers::ProviderFactory;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct ConnectivityChecker;

impl ConnectivityChecker {
    /// 检测单个服务商连通性：真正调用 Provider 的 check_connectivity
    pub async fn check_provider(provider: &AIProviderConfig, test_prompt: Option<&str>) -> ConnectivityCheck {
        use crate::types::asset::AIModality;

        let start = SystemTime::now();

        // Edge TTS 无需 API Key，跳过凭证检查，直接调用 check_connectivity
        let skip_credential_check = provider.id == "edge-tts";

        // 检查 API Key 是否已填写（Edge TTS 等无需凭证的服务跳过）
        let has_api_key = skip_credential_check || provider.auth_config.api_key
            .as_ref()
            .map(|k| !k.value.is_empty() && k.value != "FREE" && !k.value.contains("***"))
            .unwrap_or(false);

        let has_extra_creds = provider.auth_config.extra_params
            .as_ref()
            .map(|params| {
                params.values().all(|p| !p.value.is_empty() && !p.value.contains("***"))
            })
            .unwrap_or(true);

        if !has_api_key || !has_extra_creds {
            return ConnectivityCheck {
                provider_id: provider.id.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                status: ConnectivityStatus::Unconfigured,
                latency: None,
                error_message: Some("未配置 API Key".to_string()),
                quota_info: None,
                response_preview: None,
                test_prompt: test_prompt.map(|s| s.to_string()),
                media_url: None,
                media_type: None,
                polling_task_id: None,
                polling_status: None,
                polling_elapsed_secs: None,
                media_items: None,
                request_endpoint: None,
                request_model: None,
                request_headers: None,
                request_body: None,
                response_status: None,
            };
        }

        // 根据模态选择超时时间：music/video/image 需要生成时间，给更长超时
        let is_generation_modality = provider.modality.iter().any(|m| {
            matches!(m, AIModality::Music | AIModality::Image | AIModality::Video)
        });
        let timeout_secs = if is_generation_modality { 360u64 } else { 30u64 };

        // 尝试创建 Provider 并检测连通性
        let base_path = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("gen");

        match ProviderFactory::create(provider, &base_path) {
            Ok(provider_instance) => {
                let prompt = test_prompt.unwrap_or("hi");
                match tokio::time::timeout(Duration::from_secs(timeout_secs), provider_instance.check_connectivity_with_prompt(prompt)).await {
                    Ok(Ok(check)) => check,
                    Ok(Err(e)) => {
                        let latency = SystemTime::now()
                            .duration_since(start)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        ConnectivityCheck {
                            provider_id: provider.id.clone(),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            status: ConnectivityStatus::NetworkError,
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
                            request_endpoint: None,
                            request_model: None,
                            request_headers: None,
                            request_body: None,
                            response_status: None,
                        }
                    }
                    Err(_) => {
                        ConnectivityCheck {
                            provider_id: provider.id.clone(),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            status: ConnectivityStatus::NetworkError,
                            latency: Some(timeout_secs * 1000),
                            error_message: Some(format!("连接超时（{}秒）", timeout_secs)),
                            quota_info: None,
                            response_preview: None,
                            test_prompt: Some(prompt.to_string()),
                            media_url: None,
                            media_type: None,
                            polling_task_id: None,
                            polling_status: Some("failed".to_string()),
                            polling_elapsed_secs: Some(timeout_secs),
                            media_items: None,
                            request_endpoint: None,
                            request_model: None,
                            request_headers: None,
                            request_body: None,
                            response_status: None,
                        }
                    }
                }
            }
            Err(e) => {
                ConnectivityCheck {
                    provider_id: provider.id.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    status: ConnectivityStatus::Unconfigured,
                    latency: None,
                    error_message: Some(format!("Provider 创建失败: {:?}", e)),
                    quota_info: None,
                    response_preview: None,
                    test_prompt: test_prompt.map(|s| s.to_string()),
                    media_url: None,
                    media_type: None,
                    polling_task_id: None,
                    polling_status: None,
                    polling_elapsed_secs: None,
                    media_items: None,
                    request_endpoint: None,
                    request_model: None,
                    request_headers: None,
                    request_body: None,
                    response_status: None,
                }
            }
        }
    }

    /// 批量检测所有服务商连通性
    pub async fn check_all(providers: &[AIProviderConfig]) -> Vec<ConnectivityCheck> {
        let mut handles = Vec::with_capacity(providers.len());
        for provider in providers {
            let provider = provider.clone();
            handles.push(tokio::spawn(async move {
                Self::check_provider(&provider, None).await
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(check) => results.push(check),
                Err(e) => results.push(ConnectivityCheck {
                    provider_id: String::new(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    status: ConnectivityStatus::NetworkError,
                    latency: None,
                    error_message: Some(format!("检测任务失败: {}", e)),
                    quota_info: None,
                    response_preview: None,
                    test_prompt: None,
                    media_url: None,
                    media_type: None,
                    polling_task_id: None,
                    polling_status: None,
                    polling_elapsed_secs: None,
                    media_items: None,
                    request_endpoint: None,
                    request_model: None,
                    request_headers: None,
                    request_body: None,
                    response_status: None,
                }),
            }
        }
        results
    }

    /// 将 ConnectivityCheck 结果映射为 ProviderStatus
    pub fn check_to_status(check: &ConnectivityCheck) -> ProviderStatus {
        match check.status {
            ConnectivityStatus::Ok => ProviderStatus::Connected,
            ConnectivityStatus::Unconfigured => ProviderStatus::Unconfigured,
            ConnectivityStatus::AuthFailed => ProviderStatus::AuthFailed,
            ConnectivityStatus::QuotaExceeded => ProviderStatus::QuotaExceeded,
            ConnectivityStatus::NetworkError => ProviderStatus::NetworkError,
            ConnectivityStatus::UnknownError => ProviderStatus::Error,
        }
    }
}
