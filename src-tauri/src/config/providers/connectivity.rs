use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck, ConnectivityStatus, QuotaInfo};

pub struct ConnectivityChecker;

impl ConnectivityChecker {
    /// 检测单个服务商连通性（框架，返回占位结果）
    pub async fn check_provider(provider: &AIProviderConfig) -> ConnectivityCheck {
        // 占位实现：实际检测逻辑在后续节点实现
        ConnectivityCheck {
            provider_id: provider.id.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            status: ConnectivityStatus::Ok,
            latency: None,
            error_message: None,
            quota_info: None,
        }
    }

    /// 批量检测所有服务商连通性（框架，返回占位结果）
    pub async fn check_all(providers: &[AIProviderConfig]) -> Vec<ConnectivityCheck> {
        let mut results = Vec::with_capacity(providers.len());
        for provider in providers {
            results.push(Self::check_provider(provider).await);
        }
        results
    }
}
