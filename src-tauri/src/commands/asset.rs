use tauri::command;
use crate::engine::asset_manager::{AssetManager, BuiltinAssetEntry, BuiltinAssetRegistry};
use std::sync::Arc;

#[command]
pub async fn get_asset_path(
    game_id: String,
    asset_ref_id: String,
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<String, String> {
    let dir = asset_manager.get_game_asset_dir(&game_id);
    let asset_dir = dir.join("assets");
    if asset_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&asset_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&asset_ref_id) {
                    return Ok(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }
    Err(format!("Asset '{}' not found for game '{}'", asset_ref_id, game_id))
}

#[command]
pub async fn list_builtin_assets(
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<Vec<BuiltinAssetEntry>, String> {
    let builtin_path = asset_manager.base_path().join("builtin-assets");
    let mut registry = BuiltinAssetRegistry::new(builtin_path);
    registry.load();
    Ok(registry.list_all().to_vec())
}
