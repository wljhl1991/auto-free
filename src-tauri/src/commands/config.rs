use tauri::command;
use crate::config::manager::ConfigManager;
use crate::config::providers;
use crate::types::ai_provider::{AppConfig, AIProviderConfig, ConfigPreset, ConnectivityCheck, ProviderStatus};
use crate::types::asset::AIModality;
use crate::config::providers::connectivity::ConnectivityChecker;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalityAvailability {
    pub text: bool,
    pub image: bool,
    pub video: bool,
    pub music: bool,
    pub voice: bool,
}

#[command]
pub async fn get_config(
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<AppConfig, String> {
    let cm = config_manager.read().await;
    Ok(cm.get_config().clone())
}

#[command]
pub async fn update_config(
    config: AppConfig,
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<(), String> {
    let mut cm = config_manager.write().await;
    cm.update_config(config)
}

#[command]
pub async fn get_presets(
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<Vec<ConfigPreset>, String> {
    let cm = config_manager.read().await;
    Ok(cm.get_presets().to_vec())
}

#[command]
pub async fn apply_preset(
    preset_id: String,
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<(), String> {
    let mut cm = config_manager.write().await;
    cm.apply_preset(&preset_id)
}

#[command]
pub async fn get_providers(
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<Vec<AIProviderConfig>, String> {
    let cm = config_manager.read().await;
    Ok(cm.get_providers())
}

#[command]
pub async fn update_provider(
    mut provider: AIProviderConfig,
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<(), String> {
    // 还原脱敏字段：如果 API Key 包含 ***，从已存储的配置中取回真实值
    {
        let cm = config_manager.read().await;
        if let Some(existing) = cm.get_config().providers.iter().find(|p| p.id == provider.id) {
            // 还原 API Key
            if let Some(ref mut new_key) = provider.auth_config.api_key {
                if new_key.value.contains("***") {
                    if let Some(ref old_key) = existing.auth_config.api_key {
                        log::info!("还原脱敏 API Key: provider_id={}", provider.id);
                        new_key.value = old_key.value.clone();
                    }
                }
            }
            // 还原 extra params
            if let Some(ref mut new_extra) = provider.auth_config.extra_params {
                if let Some(ref old_extra) = existing.auth_config.extra_params {
                    for (key, field) in new_extra.iter_mut() {
                        if field.value.contains("***") {
                            if let Some(old_field) = old_extra.get(key) {
                                log::info!("还原脱敏 extra param: provider_id={}, key={}", provider.id, key);
                                field.value = old_field.value.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    log::info!("更新服务商配置: id={}, name={}", provider.id, provider.name);

    // 如果有 API Key 但状态还是 unconfigured，自动设为 configured
    let has_api_key = provider.auth_config.api_key
        .as_ref()
        .map(|k| !k.value.is_empty() && !k.value.contains("***"))
        .unwrap_or(false);
    let has_extra_creds = provider.auth_config.extra_params
        .as_ref()
        .map(|params| params.values().any(|p| !p.value.is_empty() && !p.value.contains("***")))
        .unwrap_or(false);

    if (has_api_key || has_extra_creds) && provider.status == ProviderStatus::Unconfigured {
        provider.status = ProviderStatus::Configured;
    }

    let mut cm = config_manager.write().await;
    cm.update_provider(provider)
}

#[command]
pub async fn check_provider(
    provider_id: String,
    test_prompt: Option<String>,
    model_id: Option<String>,
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<ConnectivityCheck, String> {
    log::info!("检测服务商连通性: id={}, model_id={:?}, test_prompt={:?}", provider_id, model_id, test_prompt);

    // 1. 读取配置获取 provider
    let mut provider = {
        let cm = config_manager.read().await;
        let config = cm.get_config();
        config.providers.iter()
            .find(|p| p.id == provider_id)
            .cloned()
            .ok_or_else(|| format!("Provider '{}' not found", provider_id))?
    };

    // 如果指定了 model_id，临时将该模型设为默认
    if let Some(ref mid) = model_id {
        if let Some(target_model) = provider.models.iter().find(|m| &m.id == mid) {
            // 临时修改：将指定模型设为默认
            for m in provider.models.iter_mut() {
                m.is_default = m.id == *mid;
            }
            log::info!("测试使用指定模型: {}", mid);
        } else {
            log::warn!("指定模型 '{}' 不存在，使用默认模型", mid);
        }
    }

    // 2. 执行连通性检测
    let check = ConnectivityChecker::check_provider(&provider, test_prompt.as_deref()).await;

    // 3. 更新 Provider 状态并保存
    let new_status = ConnectivityChecker::check_to_status(&check);
    {
        let mut cm = config_manager.write().await;
        cm.update_provider_status(&provider_id, new_status, check.error_message.clone());
    }

    Ok(check)
}

#[command]
pub async fn check_all_providers(
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<Vec<ConnectivityCheck>, String> {
    let providers = {
        let cm = config_manager.read().await;
        cm.get_config().providers.clone()
    };

    let checks = ConnectivityChecker::check_all(&providers).await;

    // 更新所有 Provider 状态
    {
        let mut cm = config_manager.write().await;
        for check in &checks {
            let new_status = ConnectivityChecker::check_to_status(check);
            cm.update_provider_status(&check.provider_id, new_status, check.error_message.clone());
        }
    }

    Ok(checks)
}

#[command]
pub async fn export_config(
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<String, String> {
    let cm = config_manager.read().await;
    cm.export_config()
}

#[command]
pub async fn import_config(
    config_json: String,
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<(), String> {
    let mut cm = config_manager.write().await;
    cm.import_config(&config_json)
}

#[command]
pub async fn save_dev_config(
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<(), String> {
    let cm = config_manager.read().await;
    cm.save_dev_config()
}

#[command]
pub async fn load_dev_config(
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<(), String> {
    let mut cm = config_manager.write().await;
    cm.load_dev_config()
}

/// 更新 provider 模型定义：验证 JSON 并保存到 config_dir/providers-override.json
#[command]
pub async fn update_provider_models(
    providers_json: String,
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<(), String> {
    // 1. 验证 JSON 能被正确解析
    let parsed: Vec<AIProviderConfig> = providers::load_providers_from_json(&providers_json)?;

    // 2. 获取 config 目录
    let config_dir = {
        let cm = config_manager.read().await;
        cm.config_dir().to_path_buf()
    };

    // 3. 保存到 providers-override.json
    let override_path = config_dir.join("providers-override.json");
    std::fs::write(&override_path, &providers_json)
        .map_err(|e| format!("写入 providers-override.json 失败: {}", e))?;

    log::info!("已更新 provider 模型定义 ({} 个 provider)，保存到: {}", parsed.len(), override_path.display());

    // 4. 更新当前运行时配置中的 providers
    let mut cm = config_manager.write().await;
    cm.reload_providers(parsed)?;

    Ok(())
}

/// 检测各模态是否有已连接的 AI 服务
#[command]
pub async fn check_available_modalities(
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<ModalityAvailability, String> {
    let cm = config_manager.read().await;
    let config = cm.get_config();

    let check = |modality: &AIModality| -> bool {
        config.providers.iter()
            .any(|p| p.modality.contains(modality) && p.status == ProviderStatus::Connected)
    };

    Ok(ModalityAvailability {
        text: check(&AIModality::Text),
        image: check(&AIModality::Image),
        video: check(&AIModality::Video),
        music: check(&AIModality::Music),
        voice: check(&AIModality::Voice),
    })
}
