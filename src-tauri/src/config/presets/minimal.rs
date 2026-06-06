use crate::types::ai_provider::{
    BuiltinFallback, ConfigPreset, PresetProvider,
};
use crate::types::asset::AIModality;

/// 极简方案：硅基流动 + 天工 + Edge TTS
/// 最少的服务商配置，覆盖核心模态
pub fn preset() -> ConfigPreset {
    ConfigPreset {
        id: "minimal".to_string(),
        name: "极简方案".to_string(),
        description: "硅基流动（文本+图片）+ 天工音乐 + Edge TTS语音，最少配置覆盖核心功能".to_string(),
        vendor_count: 3,
        providers: vec![
            PresetProvider {
                provider_id: "siliconflow".to_string(),
                modality: AIModality::Text,
                model_id: "deepseek-ai/DeepSeek-R1".to_string(),
                note: Some("硅基流动托管的 DeepSeek-R1（免费）".to_string()),
            },
            PresetProvider {
                provider_id: "siliconflow".to_string(),
                modality: AIModality::Image,
                model_id: "Kwai-Kolors/Kolors".to_string(),
                note: Some("Kolors 免费出图".to_string()),
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
            video: true,
            music: false,
            voice: false,
        },
    }
}
