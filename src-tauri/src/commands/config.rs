use tauri::command;
use crate::config::manager::ConfigManager;
use crate::types::ai_provider::{AppConfig, AIProviderConfig, ConfigPreset, ConnectivityCheck};
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
    let cm = config_manager.read().await;
    let config = cm.get_config();
    let provider = config.providers.iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;
    Ok(ConnectivityChecker::check_provider(provider).await)
}

#[command]
pub async fn check_all_providers(
    config_manager: tauri::State<'_, Arc<RwLock<ConfigManager>>>,
) -> Result<Vec<ConnectivityCheck>, String> {
    let cm = config_manager.read().await;
    let config = cm.get_config();
    Ok(ConnectivityChecker::check_all(&config.providers).await)
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
