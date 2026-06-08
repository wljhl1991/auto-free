use crate::types::ai_provider::{
    BuiltinFallback, ConfigPreset, PresetProvider,
};
use crate::types::asset::AIModality;

/// 零成本预设方案：硅基流动（免费文本+图片）+ 妙音AI + Edge TTS（免费语音）
/// 完全免费，零花费体验 AI 游戏制作
pub fn preset() -> ConfigPreset {
    ConfigPreset {
        id: "zero_cost".to_string(),
        name: "零成本方案".to_string(),
        description: "完全免费！硅基流动（免费文本+图片）+ 妙音AI + Edge TTS（免费语音），推荐也可搭配 DeepSeek V4 Flash（便宜但速度和效果更好）".to_string(),
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
                provider_id: "miaoyin".to_string(),
                modality: AIModality::Music,
                model_id: "moka-v9".to_string(),
                note: Some("集成 SUNO、Mureka 等顶级模型".to_string()),
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
