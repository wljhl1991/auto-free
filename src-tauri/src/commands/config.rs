use tauri::command;
use crate::config::manager::ConfigManager;
use crate::types::ai_provider::{AppConfig, AIProviderConfig, ConfigPreset, ConnectivityCheck, ProviderStatus};
use crate::config::providers::connectivity::ConnectivityChecker;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    provider: AIProviderConfig,
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<(), String> {
    let mut cm = config_manager.write().await;
    cm.update_provider(provider)
}

#[command]
pub async fn check_provider(
    provider_id: String,
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<ConnectivityCheck, String> {
    // 1. 读取配置获取 provider
    let provider = {
        let cm = config_manager.read().await;
        let config = cm.get_config();
        config.providers.iter()
            .find(|p| p.id == provider_id)
            .cloned()
            .ok_or_else(|| format!("Provider '{}' not found", provider_id))?
    };

    // 2. 执行连通性检测
    let check = ConnectivityChecker::check_provider(&provider).await;

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
