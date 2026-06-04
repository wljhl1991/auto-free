pub mod connectivity;

use crate::types::ai_provider::AIProviderConfig;

/// 编译时嵌入的默认 provider 定义 JSON
const DEFAULT_PROVIDERS_JSON: &str = include_str!("../../../providers.json");

/// 返回所有内置服务商的默认配置（从编译时嵌入的 JSON 加载）
pub fn builtin_providers() -> Vec<AIProviderConfig> {
    load_providers_from_json(DEFAULT_PROVIDERS_JSON)
        .unwrap_or_else(|e| {
            log::error!("加载内置 providers.json 失败: {}", e);
            Vec::new()
        })
}

/// 从 JSON 字符串解析 provider 定义
pub fn load_providers_from_json(json: &str) -> Result<Vec<AIProviderConfig>, String> {
    serde_json::from_str(json).map_err(|e| format!("解析 providers JSON 失败: {}", e))
}

/// 从外部文件加载 provider 定义（用于运行时更新）
pub fn load_providers_from_file(path: &std::path::Path) -> Result<Vec<AIProviderConfig>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取 providers 文件失败 {}: {}", path.display(), e))?;
    load_providers_from_json(&content)
}

/// 带覆盖的内置 providers 加载：
/// 优先从 config_dir/providers-override.json 加载，如果不存在则回退到编译时嵌入的默认值
pub fn builtin_providers_with_override(config_dir: &std::path::Path) -> Vec<AIProviderConfig> {
    let override_path = config_dir.join("providers-override.json");
    if override_path.exists() {
        log::info!("检测到 providers-override.json，从文件加载: {}", override_path.display());
        load_providers_from_file(&override_path)
            .unwrap_or_else(|e| {
                log::warn!("加载 providers-override.json 失败，回退到默认值: {}", e);
                builtin_providers()
            })
    } else {
        builtin_providers()
    }
}
