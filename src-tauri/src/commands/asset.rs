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

/// 读取本地文件并返回 base64 编码的 data URL（用于前端展示图片/音频/视频）
#[command]
pub async fn read_file_as_data_url(
    file_path: String,
) -> Result<String, String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    let data = std::fs::read(path)
        .map_err(|e| format!("读取文件失败: {}", e))?;

    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    };

    let base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
    Ok(format!("data:{};base64,{}", mime, base64))
}
