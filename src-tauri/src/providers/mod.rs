pub mod builtin;
pub mod deepseek;
pub mod edge_tts;
pub mod hailuo;
pub mod jimeng;
pub mod kling;
pub mod netease_music;
pub mod qwen;
pub mod siliconflow;
pub mod skymusic;
pub mod vidu;
pub mod volcengine_tts;
pub mod xfyun_spark;
pub mod zhipu;

use async_trait::async_trait;
use crate::types::asset::{LocalAsset, AIModality};
use crate::types::game_script::AssetRef;
use crate::types::ai_provider::{AIProviderConfig, ConnectivityCheck};

/// 统一资源获取接口 — 无论来源，调用方式相同
#[async_trait]
pub trait IAssetProvider: Send + Sync {
    /// 获取资源
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError>;
    /// 检测连通性
    async fn check_connectivity(&self) -> Result<ConnectivityCheck, ProviderError>;
    /// 获取支持的模态
    fn supported_modalities(&self) -> Vec<AIModality>;
    /// 获取 Provider ID
    fn provider_id(&self) -> &str;
}

/// Provider 错误类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ProviderError {
    NetworkError(String),
    AuthFailed(String),
    QuotaExceeded(String),
    GenerationFailed(String),
    InvalidConfig(String),
    NotFound(String),
    Timeout(String),
}

/// Provider 工厂 — 根据配置创建 Provider 实例
pub struct ProviderFactory;

impl ProviderFactory {
    /// 根据服务商配置创建对应的 Provider
    pub fn create(config: &AIProviderConfig, asset_base_path: &std::path::Path) -> Result<Box<dyn IAssetProvider>, ProviderError> {
        // 优先用 id 匹配（英文标识符），fallback 到 vendor（中文名）
        let key = config.id.as_str();
        match key {
            "builtin" => {
                let builtin_assets_path = asset_base_path.join("builtin-assets");
                let game_assets_path = asset_base_path.join("games");
                Ok(Box::new(builtin::BuiltinAssetProvider::new(builtin_assets_path, game_assets_path)))
            }
            "deepseek" => {
                Ok(Box::new(deepseek::DeepSeekProvider::new(config, asset_base_path)?))
            }
            "edge-tts" => {
                Ok(Box::new(edge_tts::EdgeTTSProvider::new(config, asset_base_path)?))
            }
            "siliconflow" => {
                Ok(Box::new(siliconflow::SiliconFlowProvider::new(config, asset_base_path.to_path_buf())?))
            }
            "kling" => {
                Ok(Box::new(kling::KlingProvider::new(config, asset_base_path.to_path_buf())?))
            }
            "skymusic" => {
                Ok(Box::new(skymusic::SkyMusicProvider::new(config, asset_base_path.to_path_buf())?))
            }
            "xfyun-spark-lite" => {
                Ok(Box::new(xfyun_spark::XfyunSparkProvider::new(config, asset_base_path.to_path_buf())?))
            }
            "qwen" => {
                Ok(Box::new(qwen::QwenProvider::new(config, asset_base_path)?))
            }
            "zhipu" => {
                Ok(Box::new(zhipu::ZhipuProvider::new(config, asset_base_path)?))
            }
            "hailuo" => {
                Ok(Box::new(hailuo::HailuoProvider::new(config, asset_base_path.to_path_buf())?))
            }
            "jimeng" => {
                Ok(Box::new(jimeng::JimengProvider::new(config, asset_base_path.to_path_buf())?))
            }
            "vidu" => {
                Ok(Box::new(vidu::ViduProvider::new(config, asset_base_path.to_path_buf())?))
            }
            "volcengine-tts" => {
                Ok(Box::new(volcengine_tts::VolcengineTTSProvider::new(config, asset_base_path)?))
            }
            "netease-music" => {
                Ok(Box::new(netease_music::NeteaseMusicProvider::new(config, asset_base_path.to_path_buf())?))
            }
            _ => Err(ProviderError::InvalidConfig(format!(
                "Unknown provider: id={}, vendor={}",
                config.id, config.vendor
            ))),
        }
    }
}
