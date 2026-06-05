use crate::types::ai_provider::{
    AIProviderConfig, ConnectivityCheck, ConnectivityStatus, ProviderStatus,
};
use crate::providers::ProviderFactory;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct ConnectivityChecker;

impl ConnectivityChecker {
    /// 检测单个服务商连通性：真正调用 Provider 的 check_connectivity
    pub async fn check_provider(provider: &AIProviderConfig, test_prompt: Option<&str>) -> ConnectivityCheck {
        let start = SystemTime::now();

        // Edge TTS 无需 API Key，始终 Connected
        if provider.id == "edge-tts" {
            return ConnectivityCheck {
                provider_id: provider.id.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                status: ConnectivityStatus::Ok,
                latency: Some(0),
                error_message: None,
                quota_info: None,
                response_preview: None,
                test_prompt: test_prompt.map(|s| s.to_string()),
            };
        }

        // 检查 API Key 是否已填写
        let has_api_key = provider.auth_config.api_key
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
            };
        }

        // 尝试创建 Provider 并检测连通性
        let base_path = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("gen");

        match ProviderFactory::create(provider, &base_path) {
            Ok(provider_instance) => {
                let prompt = test_prompt.unwrap_or("hi");
                match tokio::time::timeout(Duration::from_secs(30), provider_instance.check_connectivity_with_prompt(prompt)).await {
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
                            latency: Some(30000),
                            error_message: Some("连接超时（30秒）".to_string()),
                            quota_info: None,
                            response_preview: None,
                            test_prompt: Some(prompt.to_string()),
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
