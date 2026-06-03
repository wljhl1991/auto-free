use crate::types::ai_provider::{
    BuiltinFallback, ConfigPreset, PresetProvider,
};
use crate::types::asset::AIModality;

/// 仅文本预设方案：DeepSeek + Edge TTS
/// 专注于高质量文本生成，图片/视频/音乐使用内置默认资源
pub fn preset() -> ConfigPreset {
    ConfigPreset {
        id: "text_only".to_string(),
        name: "仅文本方案".to_string(),
        description: "DeepSeek 高质量文本 + Edge TTS 免费语音，图片/视频/音乐使用内置默认资源".to_string(),
        vendor_count: 2,
        providers: vec![
            PresetProvider {
                provider_id: "deepseek".to_string(),
                modality: AIModality::Text,
                model_id: "deepseek-v3.2".to_string(),
                note: Some("注册送 100 万 tokens/月".to_string()),
            },
            PresetProvider {
                provider_id: "edge-tts".to_string(),
                modality: AIModality::Voice,
                model_id: "edge-tts-zh".to_string(),
                note: Some("完全免费，无需注册".to_string()),
            },
        ],
        builtin_fallback: BuiltinFallback {
            image: true,
            video: true,
            music: true,
            voice: false,
        },
    }
}
