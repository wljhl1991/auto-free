pub mod builtin;
pub mod deepseek;
pub mod edge_tts;
pub mod hailuo;
pub mod jimeng;
pub mod kling;
pub mod miaoyin;
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
    /// 使用自定义提示词检测连通性（默认实现调用 check_connectivity）
    async fn check_connectivity_with_prompt(&self, prompt: &str) -> Result<ConnectivityCheck, ProviderError> {
        let _ = prompt;
        self.check_connectivity().await
    }
    /// 获取支持的模态
    #[allow(dead_code)]
    fn supported_modalities(&self) -> Vec<AIModality>;
    /// 获取 Provider ID
    #[allow(dead_code)]
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

/// 安全的 UTF-8 字符串截断（不会在多字节字符中间切断）
/// max_bytes 是字节长度上限，函数会找到不超过此上限的最后一个字符边界
pub fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // 找到不超过 max_bytes 的最后一个字符边界
    let mut boundary = max_bytes;
    while boundary > 0 && !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &s[..boundary]
}

/// 保存 AI 原始响应到 gen/ai-responses/ 目录，不截断
/// provider_id: 服务商标识（如 "deepseek", "siliconflow"）
/// api_type: 接口类型（如 "chat", "image_gen", "tts"）
/// response: 原始响应内容
pub fn save_raw_response(provider_id: &str, api_type: &str, response: &str) {
    let dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("gen")
        .join("ai-responses");
    std::fs::create_dir_all(&dir).ok();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = dir.join(format!("{}_{}_{}.txt", provider_id, api_type, ts));
    match std::fs::write(&path, response) {
        Ok(_) => log::info!("[RawResponse] 已保存: {} ({}字节)", path.display(), response.len()),
        Err(e) => log::warn!("[RawResponse] 保存失败: {}", e),
    }
}

/// Provider 工厂 — 根据配置创建 Provider 实例
pub struct ProviderFactory;

impl ProviderFactory {
    /// 根据服务商配置创建对应的 Provider
    pub fn create(config: &AIProviderConfig, asset_base_path: &std::path::Path) -> Result<Box<dyn IAssetProvider>, ProviderError> {
        // 优先用 provider_type（自定义服务商的原始类型），其次用 id
        let key = config.provider_type.as_deref().unwrap_or(config.id.as_str());
        match key {
            "builtin" => {
                let builtin_assets_path = asset_base_path.join("builtin-assets");
                let game_assets_path = asset_base_path.join("games");
                Ok(Box::new(builtin::BuiltinAssetProvider::new(builtin_assets_path, game_assets_path, asset_base_path.to_path_buf())))
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
            "miaoyin" => {
                Ok(Box::new(miaoyin::MiaoYinProvider::new(config, asset_base_path.to_path_buf())?))
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
            // 讯飞星辰使用 OpenAI 兼容 HTTP API，复用 DeepSeek Provider
            "xfyun-sparkdesk" => {
                Ok(Box::new(deepseek::DeepSeekProvider::new(config, asset_base_path)?))
            }
            _ => Err(ProviderError::InvalidConfig(format!(
                "Unknown provider: id={}, vendor={}",
                config.id, config.vendor
            ))),
        }
    }
}
