pub mod connectivity;

use crate::types::ai_provider::{
    AIModelConfig, AIProviderConfig, AccountCredentials, ApiKeyField, AuthConfig, AuthType,
    CredentialField, ExtraParamField, OAuthConfig, ProviderStatus, QualityLevel,
};
use crate::types::asset::AIModality;
use std::collections::HashMap;

/// 返回所有内置服务商的默认配置
pub fn builtin_providers() -> Vec<AIProviderConfig> {
    vec![
        deepseek(),
        xfyun_spark_lite(),
        siliconflow(),
        kling(),
        hailuo(),
        skymusic(),
        edge_tts(),
        volcengine_tts(),
        tongyi(),
        zhipu(),
        jimeng(),
        vidu(),
        xfyun_tts(),
        netease_music(),
    ]
}

// ─── 文本：DeepSeek ───
fn deepseek() -> AIProviderConfig {
    AIProviderConfig {
        id: "deepseek".to_string(),
        name: "DeepSeek".to_string(),
        vendor: "深度求索".to_string(),
        description: "国内顶尖大语言模型，API 兼容 OpenAI 格式，价格极低".to_string(),
        official_url: "https://www.deepseek.com".to_string(),
        register_url: "https://platform.deepseek.com/register".to_string(),
        docs_url: "https://platform.deepseek.com/api-docs".to_string(),
        modality: vec![AIModality::Text],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "API Key".to_string(),
                placeholder: "sk-xxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://platform.deepseek.com/api_keys".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: None,
        },
        models: vec![
            AIModelConfig {
                id: "deepseek-v4-flash".to_string(),
                name: "DeepSeek-V4-Flash (推荐)".to_string(),
                modality: AIModality::Text,
                is_default: true,
                endpoint: "https://api.deepseek.com/v1/chat/completions".to_string(),
                max_tokens: Some(131072),
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.001),
                free_quota: Some("注册送 500 万 tokens".to_string()),
                quality: QualityLevel::High,
            },
            AIModelConfig {
                id: "deepseek-v4-pro".to_string(),
                name: "DeepSeek-V4-Pro (旗舰)".to_string(),
                modality: AIModality::Text,
                is_default: false,
                endpoint: "https://api.deepseek.com/v1/chat/completions".to_string(),
                max_tokens: Some(131072),
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.01),
                free_quota: Some("注册送 500 万 tokens".to_string()),
                quality: QualityLevel::High,
            },
            AIModelConfig {
                id: "deepseek-chat".to_string(),
                name: "deepseek-chat (兼容旧名)".to_string(),
                modality: AIModality::Text,
                is_default: false,
                endpoint: "https://api.deepseek.com/v1/chat/completions".to_string(),
                max_tokens: Some(131072),
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.001),
                free_quota: Some("2026-07-24 后停用，请迁移到 V4-Flash".to_string()),
                quality: QualityLevel::Standard,
            },
            AIModelConfig {
                id: "deepseek-reasoner".to_string(),
                name: "deepseek-reasoner (推理)".to_string(),
                modality: AIModality::Text,
                is_default: false,
                endpoint: "https://api.deepseek.com/v1/chat/completions".to_string(),
                max_tokens: Some(131072),
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.004),
                free_quota: Some("2026-07-24 后停用，请迁移到 V4-Pro".to_string()),
                quality: QualityLevel::High,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 文本（永久免费）：讯飞星火 Lite ───
fn xfyun_spark_lite() -> AIProviderConfig {
    let mut extra_params = HashMap::new();
    extra_params.insert(
        "apiSecret".to_string(),
        ExtraParamField {
            value: String::new(),
            label: "APISecret".to_string(),
            placeholder: "xxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
            required: true,
            secret: true,
        },
    );
    extra_params.insert(
        "apiKeyReal".to_string(),
        ExtraParamField {
            value: String::new(),
            label: "APIKey".to_string(),
            placeholder: "xxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
            required: true,
            secret: true,
        },
    );

    AIProviderConfig {
        id: "xfyun-spark-lite".to_string(),
        name: "讯飞星火 Lite".to_string(),
        vendor: "科大讯飞".to_string(),
        description: "永久免费，无限 Tokens！QPS 限制 2 次/秒，适合零成本体验".to_string(),
        official_url: "https://www.xfyun.cn".to_string(),
        register_url: "https://www.xfyun.cn/register".to_string(),
        docs_url: "https://www.xfyun.cn/doc/spark/Web.html".to_string(),
        modality: vec![AIModality::Text, AIModality::Voice],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "APPID".to_string(),
                placeholder: "xxxxxxxx".to_string(),
                help_url: "https://console.xfyun.cn/services/bm3".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: Some(extra_params),
        },
        models: vec![
            AIModelConfig {
                id: "spark-lite".to_string(),
                name: "星火 Lite".to_string(),
                modality: AIModality::Text,
                is_default: true,
                endpoint: "wss://spark-api.xf-yun.com/v1.1/chat".to_string(),
                max_tokens: Some(4096),
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.0),
                free_quota: Some("永久免费，无限 Tokens，QPS 2次/秒".to_string()),
                quality: QualityLevel::Standard,
            },
            AIModelConfig {
                id: "spark-tts".to_string(),
                name: "星火 TTS".to_string(),
                modality: AIModality::Voice,
                is_default: true,
                endpoint: "wss://tts-api.xfyun.cn/v2/tts".to_string(),
                max_tokens: None,
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.0),
                free_quota: Some("每日免费".to_string()),
                quality: QualityLevel::High,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 图片 + 文本：硅基流动 ───
fn siliconflow() -> AIProviderConfig {
    AIProviderConfig {
        id: "siliconflow".to_string(),
        name: "硅基流动 SiliconFlow".to_string(),
        vendor: "硅基流动".to_string(),
        description: "托管 FLUX/SD3 等图片模型和 DeepSeek/Qwen 文本模型，基础开源模型免费调用".to_string(),
        official_url: "https://siliconflow.cn".to_string(),
        register_url: "https://cloud.siliconflow.cn/register".to_string(),
        docs_url: "https://docs.siliconflow.cn".to_string(),
        modality: vec![AIModality::Text, AIModality::Image],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "API Key".to_string(),
                placeholder: "sk-xxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://cloud.siliconflow.cn/account/ak".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: None,
        },
        models: vec![
            AIModelConfig {
                id: "deepseek-v3.2".to_string(),
                name: "DeepSeek-V3.2 (硅基流动托管)".to_string(),
                modality: AIModality::Text,
                is_default: false,
                endpoint: "https://api.siliconflow.cn/v1/chat/completions".to_string(),
                max_tokens: Some(65536),
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.001),
                free_quota: Some("基础开源模型免费，付费模型按量计费".to_string()),
                quality: QualityLevel::High,
            },
            AIModelConfig {
                id: "flux-1-schnell".to_string(),
                name: "FLUX.1-schnell".to_string(),
                modality: AIModality::Image,
                is_default: true,
                endpoint: "https://api.siliconflow.cn/v1/images/generations".to_string(),
                max_tokens: None,
                supported_sizes: Some(vec![
                    "1:1".to_string(), "16:9".to_string(), "9:16".to_string(),
                    "4:3".to_string(), "3:4".to_string(),
                ]),
                max_duration: None,
                cost_per_call: Some(0.5),
                free_quota: Some("注册送 2000 万 tokens".to_string()),
                quality: QualityLevel::Fast,
            },
            AIModelConfig {
                id: "flux-1-dev".to_string(),
                name: "FLUX.1-dev".to_string(),
                modality: AIModality::Image,
                is_default: false,
                endpoint: "https://api.siliconflow.cn/v1/images/generations".to_string(),
                max_tokens: None,
                supported_sizes: Some(vec![
                    "1:1".to_string(), "16:9".to_string(), "9:16".to_string(),
                    "4:3".to_string(), "3:4".to_string(),
                ]),
                max_duration: None,
                cost_per_call: Some(2.0),
                free_quota: Some("注册送 2000 万 tokens".to_string()),
                quality: QualityLevel::High,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 视频：可灵 ───
fn kling() -> AIProviderConfig {
    let mut extra_params = HashMap::new();
    extra_params.insert(
        "secretKey".to_string(),
        ExtraParamField {
            value: String::new(),
            label: "Secret Key".to_string(),
            placeholder: "sk-xxxxxxxxxxxxxxxx".to_string(),
            required: true,
            secret: true,
        },
    );

    AIProviderConfig {
        id: "kling".to_string(),
        name: "可灵 Kling".to_string(),
        vendor: "快手".to_string(),
        description: "国内领先的视频生成模型，3.0 版本支持 4K 60 帧、智能分镜、原生音频".to_string(),
        official_url: "https://kling.kuaishou.com".to_string(),
        register_url: "https://kling.kuaishou.com/register".to_string(),
        docs_url: "https://platform.kuaishou.com/docs/kling".to_string(),
        modality: vec![AIModality::Video, AIModality::Image],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "Access Key".to_string(),
                placeholder: "ak-xxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://platform.kuaishou.com/developer/key".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: Some(extra_params),
        },
        models: vec![
            AIModelConfig {
                id: "kling-3.0".to_string(),
                name: "可灵 3.0".to_string(),
                modality: AIModality::Video,
                is_default: true,
                endpoint: "https://api.klingai.com/v1/videos/generations".to_string(),
                max_tokens: None,
                supported_sizes: Some(vec![
                    "16:9".to_string(), "9:16".to_string(), "1:1".to_string(),
                ]),
                max_duration: Some(10),
                cost_per_call: Some(1.0),
                free_quota: Some("每日免费 6 条".to_string()),
                quality: QualityLevel::High,
            },
            AIModelConfig {
                id: "kling-3.0-image".to_string(),
                name: "可灵 3.0 Image".to_string(),
                modality: AIModality::Image,
                is_default: false,
                endpoint: "https://api.klingai.com/v1/images/generations".to_string(),
                max_tokens: None,
                supported_sizes: Some(vec![
                    "1:1".to_string(), "16:9".to_string(), "9:16".to_string(),
                    "4:3".to_string(), "3:4".to_string(),
                ]),
                max_duration: None,
                cost_per_call: Some(0.5),
                free_quota: Some("每日免费额度".to_string()),
                quality: QualityLevel::High,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 视频（备选）：海螺AI ───
fn hailuo() -> AIProviderConfig {
    AIProviderConfig {
        id: "hailuo".to_string(),
        name: "海螺AI (MiniMax)".to_string(),
        vendor: "MiniMax".to_string(),
        description: "叙事感强的视频生成，首尾帧控制出色，适合故事性 CG".to_string(),
        official_url: "https://hailuoai.video".to_string(),
        register_url: "https://hailuoai.video/register".to_string(),
        docs_url: "https://www.minimaxi.com/docs".to_string(),
        modality: vec![AIModality::Video],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "API Key".to_string(),
                placeholder: "xxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://www.minimaxi.com/console/api-key".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: None,
        },
        models: vec![
            AIModelConfig {
                id: "hailuo-02".to_string(),
                name: "Hailuo 02".to_string(),
                modality: AIModality::Video,
                is_default: true,
                endpoint: "https://api.minimaxi.com/v1/video_generation".to_string(),
                max_tokens: None,
                supported_sizes: Some(vec![
                    "16:9".to_string(), "9:16".to_string(), "1:1".to_string(),
                ]),
                max_duration: Some(6),
                cost_per_call: Some(1.0),
                free_quota: Some("免费额度较大".to_string()),
                quality: QualityLevel::High,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 音乐：天工音乐 ───
fn skymusic() -> AIProviderConfig {
    AIProviderConfig {
        id: "skymusic".to_string(),
        name: "天工音乐 SkyMusic".to_string(),
        vendor: "昆仑万维".to_string(),
        description: "国内最早的 AI 音乐生成平台，每日免费额度".to_string(),
        official_url: "https://music.tiangong.cn".to_string(),
        register_url: "https://music.tiangong.cn/register".to_string(),
        docs_url: "https://open.tiangong.cn/docs".to_string(),
        modality: vec![AIModality::Music, AIModality::Voice],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "API Key".to_string(),
                placeholder: "tg-xxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://open.tiangong.cn/console/api-key".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: None,
        },
        models: vec![
            AIModelConfig {
                id: "skymusic-v1".to_string(),
                name: "天工音乐 V1".to_string(),
                modality: AIModality::Music,
                is_default: true,
                endpoint: "https://api.tiangong.cn/v1/music/generations".to_string(),
                max_tokens: None,
                supported_sizes: None,
                max_duration: Some(180),
                cost_per_call: Some(1.0),
                free_quota: Some("每日免费生成额度".to_string()),
                quality: QualityLevel::Standard,
            },
            AIModelConfig {
                id: "skymusic-tts".to_string(),
                name: "天工 TTS".to_string(),
                modality: AIModality::Voice,
                is_default: false,
                endpoint: "https://api.tiangong.cn/v1/tts/generations".to_string(),
                max_tokens: None,
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.1),
                free_quota: Some("每日免费额度".to_string()),
                quality: QualityLevel::Standard,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 语音（完全免费）：Edge TTS ───
fn edge_tts() -> AIProviderConfig {
    AIProviderConfig {
        id: "edge-tts".to_string(),
        name: "Edge TTS".to_string(),
        vendor: "微软".to_string(),
        description: "完全免费，无需 API Key，无需注册！支持 40+ 语言包含中文多种方言".to_string(),
        official_url: "https://github.com/rany2/edge-tts".to_string(),
        register_url: String::new(),
        docs_url: "https://github.com/rany2/edge-tts#usage".to_string(),
        modality: vec![AIModality::Voice],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: "FREE".to_string(),
                label: "API Key".to_string(),
                placeholder: "无需填写，完全免费".to_string(),
                help_url: String::new(),
            }),
            account: None,
            oauth: None,
            extra_params: None,
        },
        models: vec![
            AIModelConfig {
                id: "edge-tts-zh".to_string(),
                name: "Edge TTS 中文".to_string(),
                modality: AIModality::Voice,
                is_default: true,
                endpoint: "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1".to_string(),
                max_tokens: None,
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.0),
                free_quota: Some("完全免费，无限制".to_string()),
                quality: QualityLevel::Standard,
            },
        ],
        status: ProviderStatus::Connected,
        last_checked: None,
        error_message: None,
    }
}

// ─── 语音（备选）：火山引擎 TTS ───
fn volcengine_tts() -> AIProviderConfig {
    let mut extra_params = HashMap::new();
    extra_params.insert(
        "secretKey".to_string(),
        ExtraParamField {
            value: String::new(),
            label: "Secret Access Key".to_string(),
            placeholder: "xxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
            required: true,
            secret: true,
        },
    );
    extra_params.insert(
        "appId".to_string(),
        ExtraParamField {
            value: String::new(),
            label: "应用 ID (App ID)".to_string(),
            placeholder: "xxxxxxxx".to_string(),
            required: true,
            secret: false,
        },
    );

    AIProviderConfig {
        id: "volcengine-tts".to_string(),
        name: "火山引擎 TTS".to_string(),
        vendor: "字节跳动".to_string(),
        description: "音色丰富、支持情感控制的 TTS 服务".to_string(),
        official_url: "https://www.volcengine.com/product/tts".to_string(),
        register_url: "https://console.volcengine.com/register".to_string(),
        docs_url: "https://www.volcengine.com/docs/6561/97465".to_string(),
        modality: vec![AIModality::Voice],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "Access Key ID".to_string(),
                placeholder: "AKLT-xxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://console.volcengine.com/iam/keymanage".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: Some(extra_params),
        },
        models: vec![
            AIModelConfig {
                id: "volcengine-tts-v1".to_string(),
                name: "火山引擎 TTS V1".to_string(),
                modality: AIModality::Voice,
                is_default: true,
                endpoint: "https://openspeech.bytedance.com/api/v1/tts".to_string(),
                max_tokens: None,
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.01),
                free_quota: Some("新用户免费试用额度".to_string()),
                quality: QualityLevel::High,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 备选：通义千问 ───
fn tongyi() -> AIProviderConfig {
    AIProviderConfig {
        id: "tongyi".to_string(),
        name: "通义千问".to_string(),
        vendor: "阿里云".to_string(),
        description: "阿里云大语言模型 Qwen3.6 + 通义万相图片/视频生成".to_string(),
        official_url: "https://tongyi.aliyun.com".to_string(),
        register_url: "https://dashscope.console.aliyun.com/register".to_string(),
        docs_url: "https://help.aliyun.com/zh/dashscope".to_string(),
        modality: vec![AIModality::Text, AIModality::Image, AIModality::Video],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "DashScope API Key".to_string(),
                placeholder: "sk-xxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://dashscope.console.aliyun.com/apiKey".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: None,
        },
        models: vec![
            AIModelConfig {
                id: "qwen3.6-plus".to_string(),
                name: "Qwen3.6-Plus".to_string(),
                modality: AIModality::Text,
                is_default: true,
                endpoint: "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions".to_string(),
                max_tokens: Some(131072),
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.004),
                free_quota: Some("免费额度".to_string()),
                quality: QualityLevel::High,
            },
            AIModelConfig {
                id: "wanx-v2.1".to_string(),
                name: "通义万相 V2.1".to_string(),
                modality: AIModality::Image,
                is_default: true,
                endpoint: "https://dashscope.aliyuncs.com/api/v1/services/aigc/text2image/image-synthesis".to_string(),
                max_tokens: None,
                supported_sizes: Some(vec![
                    "1:1".to_string(), "16:9".to_string(), "9:16".to_string(),
                ]),
                max_duration: None,
                cost_per_call: Some(0.08),
                free_quota: Some("每日免费额度".to_string()),
                quality: QualityLevel::Standard,
            },
            AIModelConfig {
                id: "wanx-video".to_string(),
                name: "通义万相视频".to_string(),
                modality: AIModality::Video,
                is_default: true,
                endpoint: "https://dashscope.aliyuncs.com/api/v1/services/aigc/video-generation".to_string(),
                max_tokens: None,
                supported_sizes: Some(vec![
                    "16:9".to_string(), "9:16".to_string(), "1:1".to_string(),
                ]),
                max_duration: Some(6),
                cost_per_call: Some(1.0),
                free_quota: Some("每日免费 10 次".to_string()),
                quality: QualityLevel::Standard,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 备选：智谱 GLM ───
fn zhipu() -> AIProviderConfig {
    AIProviderConfig {
        id: "zhipu".to_string(),
        name: "智谱 GLM".to_string(),
        vendor: "智谱AI".to_string(),
        description: "智谱清言大模型 GLM-4，支持文本和图片生成".to_string(),
        official_url: "https://open.bigmodel.cn".to_string(),
        register_url: "https://open.bigmodel.cn/register".to_string(),
        docs_url: "https://open.bigmodel.cn/dev/api".to_string(),
        modality: vec![AIModality::Text, AIModality::Image],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "API Key".to_string(),
                placeholder: "xxxxxxxxxxxxxxxxxxxxxxxx.xxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://open.bigmodel.cn/usercenter/apikeys".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: None,
        },
        models: vec![
            AIModelConfig {
                id: "glm-4-plus".to_string(),
                name: "GLM-4-Plus".to_string(),
                modality: AIModality::Text,
                is_default: true,
                endpoint: "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string(),
                max_tokens: Some(128000),
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.015),
                free_quota: Some("新用户赠送额度".to_string()),
                quality: QualityLevel::High,
            },
            AIModelConfig {
                id: "cogview-4".to_string(),
                name: "CogView-4".to_string(),
                modality: AIModality::Image,
                is_default: true,
                endpoint: "https://open.bigmodel.cn/api/paas/v4/images/generations".to_string(),
                max_tokens: None,
                supported_sizes: Some(vec![
                    "1:1".to_string(), "16:9".to_string(), "9:16".to_string(),
                ]),
                max_duration: None,
                cost_per_call: Some(0.1),
                free_quota: Some("新用户赠送额度".to_string()),
                quality: QualityLevel::Standard,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 备选：即梦 ───
fn jimeng() -> AIProviderConfig {
    AIProviderConfig {
        id: "jimeng".to_string(),
        name: "即梦 Jimeng".to_string(),
        vendor: "字节跳动".to_string(),
        description: "字节跳动旗下 AI 图片/视频生成平台".to_string(),
        official_url: "https://jimeng.jianying.com".to_string(),
        register_url: "https://jimeng.jianying.com/register".to_string(),
        docs_url: "https://jimeng.jianying.com/docs".to_string(),
        modality: vec![AIModality::Image, AIModality::Video],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "API Key".to_string(),
                placeholder: "xxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://jimeng.jianying.com/api-key".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: None,
        },
        models: vec![
            AIModelConfig {
                id: "jimeng-image-v1".to_string(),
                name: "即梦图片 V1".to_string(),
                modality: AIModality::Image,
                is_default: true,
                endpoint: "https://api.jimeng.jianying.com/v1/images/generations".to_string(),
                max_tokens: None,
                supported_sizes: Some(vec![
                    "1:1".to_string(), "16:9".to_string(), "9:16".to_string(),
                ]),
                max_duration: None,
                cost_per_call: Some(0.05),
                free_quota: Some("每日免费额度".to_string()),
                quality: QualityLevel::Standard,
            },
            AIModelConfig {
                id: "jimeng-video-v1".to_string(),
                name: "即梦视频 V1".to_string(),
                modality: AIModality::Video,
                is_default: true,
                endpoint: "https://api.jimeng.jianying.com/v1/videos/generations".to_string(),
                max_tokens: None,
                supported_sizes: None,
                max_duration: Some(6),
                cost_per_call: Some(2.0),
                free_quota: Some("每日免费额度".to_string()),
                quality: QualityLevel::Standard,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 备选：Vidu ───
fn vidu() -> AIProviderConfig {
    AIProviderConfig {
        id: "vidu".to_string(),
        name: "Vidu".to_string(),
        vendor: "生数科技".to_string(),
        description: "高质量视频生成模型，参考图生成能力出色".to_string(),
        official_url: "https://www.vidu.studio".to_string(),
        register_url: "https://www.vidu.studio/register".to_string(),
        docs_url: "https://www.vidu.studio/docs".to_string(),
        modality: vec![AIModality::Video],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "API Key".to_string(),
                placeholder: "xxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://www.vidu.studio/api-key".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: None,
        },
        models: vec![
            AIModelConfig {
                id: "vidu-2.0".to_string(),
                name: "Vidu 2.0".to_string(),
                modality: AIModality::Video,
                is_default: true,
                endpoint: "https://api.vidu.studio/v1/videos/generations".to_string(),
                max_tokens: None,
                supported_sizes: Some(vec![
                    "16:9".to_string(), "9:16".to_string(), "1:1".to_string(),
                ]),
                max_duration: Some(8),
                cost_per_call: Some(2.0),
                free_quota: Some("新用户试用额度".to_string()),
                quality: QualityLevel::High,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 备选：讯飞 TTS ───
fn xfyun_tts() -> AIProviderConfig {
    let mut extra_params = HashMap::new();
    extra_params.insert(
        "apiSecret".to_string(),
        ExtraParamField {
            value: String::new(),
            label: "APISecret".to_string(),
            placeholder: "xxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
            required: true,
            secret: true,
        },
    );
    extra_params.insert(
        "apiKeyReal".to_string(),
        ExtraParamField {
            value: String::new(),
            label: "APIKey".to_string(),
            placeholder: "xxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
            required: true,
            secret: true,
        },
    );

    AIProviderConfig {
        id: "xfyun-tts".to_string(),
        name: "讯飞 TTS".to_string(),
        vendor: "科大讯飞".to_string(),
        description: "国内老牌 TTS 服务，音色自然".to_string(),
        official_url: "https://www.xfyun.cn/services/online_tts".to_string(),
        register_url: "https://www.xfyun.cn/register".to_string(),
        docs_url: "https://www.xfyun.cn/doc/tts/online_tts/API.html".to_string(),
        modality: vec![AIModality::Voice],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "APPID".to_string(),
                placeholder: "xxxxxxxx".to_string(),
                help_url: "https://console.xfyun.cn/services/tts".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: Some(extra_params),
        },
        models: vec![
            AIModelConfig {
                id: "xfyun-tts-v1".to_string(),
                name: "讯飞在线语音合成".to_string(),
                modality: AIModality::Voice,
                is_default: true,
                endpoint: "wss://tts-api.xfyun.cn/v2/tts".to_string(),
                max_tokens: None,
                supported_sizes: None,
                max_duration: None,
                cost_per_call: Some(0.005),
                free_quota: Some("新用户 5 万次/月".to_string()),
                quality: QualityLevel::High,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}

// ─── 备选：网易天音 ───
fn netease_music() -> AIProviderConfig {
    AIProviderConfig {
        id: "netease-music".to_string(),
        name: "网易天音".to_string(),
        vendor: "网易".to_string(),
        description: "网易 AI 音乐生成平台".to_string(),
        official_url: "https://tianyin.163.com".to_string(),
        register_url: "https://tianyin.163.com/register".to_string(),
        docs_url: "https://tianyin.163.com/docs".to_string(),
        modality: vec![AIModality::Music],
        auth_type: AuthType::ApiKey,
        auth_config: AuthConfig {
            api_key: Some(ApiKeyField {
                value: String::new(),
                label: "API Key".to_string(),
                placeholder: "xxxxxxxxxxxxxxxx".to_string(),
                help_url: "https://tianyin.163.com/console/api-key".to_string(),
            }),
            account: None,
            oauth: None,
            extra_params: None,
        },
        models: vec![
            AIModelConfig {
                id: "netease-music-v1".to_string(),
                name: "网易天音 V1".to_string(),
                modality: AIModality::Music,
                is_default: true,
                endpoint: "https://api.tianyin.163.com/v1/music/generations".to_string(),
                max_tokens: None,
                supported_sizes: None,
                max_duration: Some(120),
                cost_per_call: Some(1.0),
                free_quota: Some("试用额度".to_string()),
                quality: QualityLevel::Standard,
            },
        ],
        status: ProviderStatus::Unconfigured,
        last_checked: None,
        error_message: None,
    }
}
