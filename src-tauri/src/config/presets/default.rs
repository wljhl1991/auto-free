use crate::types::ai_provider::{
    BuiltinFallback, ConfigPreset, PresetProvider,
};
use crate::types::asset::AIModality;

/// 默认推荐预设方案：DeepSeek + 硅基流动 + 可灵 + 天工 + Edge TTS
/// 覆盖所有模态，推荐给大多数用户
pub fn preset() -> ConfigPreset {
    ConfigPreset {
        id: "default".to_string(),
        name: "默认推荐方案".to_string(),
        description: "DeepSeek文本 + 硅基流动图片 + 可灵视频 + 天工音乐 + Edge TTS语音，覆盖全模态".to_string(),
        vendor_count: 5,
        providers: vec![
            PresetProvider {
                provider_id: "deepseek".to_string(),
                modality: AIModality::Text,
                model_id: "deepseek-v3.2".to_string(),
                note: Some("注册送 100 万 tokens/月".to_string()),
            },
            PresetProvider {
                provider_id: "siliconflow".to_string(),
                modality: AIModality::Image,
                model_id: "flux-1-schnell".to_string(),
                note: Some("注册送 2000 万 tokens".to_string()),
            },
            PresetProvider {
                provider_id: "kling".to_string(),
                modality: AIModality::Video,
                model_id: "kling-3.0".to_string(),
                note: Some("每日免费 6 条".to_string()),
            },
            PresetProvider {
                provider_id: "skymusic".to_string(),
                modality: AIModality::Music,
                model_id: "skymusic-v1".to_string(),
                note: Some("每日免费生成额度".to_string()),
            },
            PresetProvider {
                provider_id: "edge-tts".to_string(),
                modality: AIModality::Voice,
                model_id: "edge-tts-zh".to_string(),
                note: Some("完全免费，无需注册".to_string()),
            },
        ],
        builtin_fallback: BuiltinFallback {
            image: false,
            video: false,
            music: false,
            voice: false,
        },
    }
}
