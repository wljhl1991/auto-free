use crate::types::ai_provider::{
    BuiltinFallback, ConfigPreset, PresetProvider,
};
use crate::types::asset::AIModality;

/// 零成本预设方案：讯飞星火Lite + Edge TTS
/// 完全免费，无需付费 API
pub fn preset() -> ConfigPreset {
    ConfigPreset {
        id: "zero_cost".to_string(),
        name: "零成本方案".to_string(),
        description: "完全免费！讯飞星火Lite（永久免费文本）+ Edge TTS（免费语音），零花费体验 AI 游戏制作".to_string(),
        vendor_count: 2,
        providers: vec![
            PresetProvider {
                provider_id: "xfyun-spark-lite".to_string(),
                modality: AIModality::Text,
                model_id: "spark-lite".to_string(),
                note: Some("永久免费，QPS 2次/秒".to_string()),
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
