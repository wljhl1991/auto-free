use serde::{Deserialize, Serialize};
use tauri::command;
use crate::engine::asset_manager::AssetManager;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAssetEntry {
    pub id: String,
    pub name: String,
    pub asset_type: String, // "image" | "music" | "video" | "voice"
    pub file_path: String,  // relative path under user-assets/
    pub tags: Vec<String>,
    pub created_at: u64,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserAssetRegistry {
    assets: Vec<UserAssetEntry>,
}

impl UserAssetRegistry {
    fn load(base_path: &std::path::Path) -> Self {
        let registry_path = base_path.join("user-assets").join("registry.json");
        if registry_path.exists() {
            let json = std::fs::read_to_string(&registry_path).unwrap_or_default();
            serde_json::from_str(&json).unwrap_or(Self { assets: Vec::new() })
        } else {
            Self { assets: Vec::new() }
        }
    }

    fn save(&self, base_path: &std::path::Path) -> Result<(), String> {
        let user_assets_dir = base_path.join("user-assets");
        std::fs::create_dir_all(&user_assets_dir)
            .map_err(|e| format!("Failed to create user-assets dir: {}", e))?;
        let registry_path = user_assets_dir.join("registry.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize registry: {}", e))?;
        std::fs::write(&registry_path, json)
            .map_err(|e| format!("Failed to write registry: {}", e))
    }
}

/// Ensure user-assets subdirectories exist
pub fn ensure_user_assets_dirs(base_path: &std::path::Path) -> Result<(), String> {
    let user_assets = base_path.join("user-assets");
    for sub in &["images", "music", "videos", "voices"] {
        std::fs::create_dir_all(user_assets.join(sub))
            .map_err(|e| format!("Failed to create user-assets/{}: {}", sub, e))?;
    }
    Ok(())
}

/// Get the subdirectory name for a given asset type
fn asset_type_to_subdir(asset_type: &str) -> Result<&'static str, String> {
    match asset_type {
        "image" => Ok("images"),
        "music" => Ok("music"),
        "video" => Ok("videos"),
        "voice" => Ok("voices"),
        _ => Err(format!("Unknown asset type: {}. Must be one of: image, music, video, voice", asset_type)),
    }
}

/// Import a user asset by copying from source_path into user-assets directory
#[command]
pub async fn import_user_asset(
    source_path: String,
    asset_type: String,
    name: String,
    tags: Vec<String>,
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<UserAssetEntry, String> {
    let base_path = asset_manager.base_path();
    ensure_user_assets_dirs(base_path)?;

    let subdir = asset_type_to_subdir(&asset_type)?;
    let source = PathBuf::from(&source_path);

    if !source.exists() {
        return Err(format!("Source file not found: {}", source_path));
    }

    let file_size = source.metadata()
        .map_err(|e| format!("Failed to read source file metadata: {}", e))?
        .len();

    // Generate unique ID
    let id = format!("user_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..12].to_string());

    // Determine destination filename: keep original extension
    let extension = source.extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_else(|| match asset_type.as_str() {
            "image" => "png".to_string(),
            "music" => "mp3".to_string(),
            "video" => "mp4".to_string(),
            "voice" => "mp3".to_string(),
            _ => "bin".to_string(),
        });

    let dest_filename = format!("{}.{}", id, extension);
    let relative_path = format!("{}/{}", subdir, dest_filename);
    let dest_path = base_path.join("user-assets").join(&relative_path);

    // Copy file
    std::fs::copy(&source, &dest_path)
        .map_err(|e| format!("Failed to copy file: {}", e))?;

    let entry = UserAssetEntry {
        id,
        name,
        asset_type,
        file_path: relative_path,
        tags,
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        file_size,
    };

    // Update registry
    let mut registry = UserAssetRegistry::load(base_path);
    registry.assets.push(entry.clone());
    registry.save(base_path)?;

    Ok(entry)
}

/// List user-imported assets, optionally filtered by type
#[command]
pub async fn list_user_assets(
    asset_type: Option<String>,
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<Vec<UserAssetEntry>, String> {
    let base_path = asset_manager.base_path();
    let registry = UserAssetRegistry::load(base_path);

    let result = match asset_type {
        Some(ref at) => registry.assets.into_iter().filter(|a| a.asset_type == *at).collect(),
        None => registry.assets,
    };

    Ok(result)
}

/// Delete a user asset by ID
#[command]
pub async fn delete_user_asset(
    asset_id: String,
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<(), String> {
    let base_path = asset_manager.base_path();
    let mut registry = UserAssetRegistry::load(base_path);

    let idx = registry.assets.iter().position(|a| a.id == asset_id)
        .ok_or_else(|| format!("Asset '{}' not found", asset_id))?;

    let entry = registry.assets.remove(idx);

    // Delete the file
    let file_path = base_path.join("user-assets").join(&entry.file_path);
    if file_path.exists() {
        std::fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to delete file: {}", e))?;
    }

    registry.save(base_path)?;

    Ok(())
}

/// Get the local file path for a user asset
#[command]
pub async fn get_user_asset_path(
    asset_id: String,
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<String, String> {
    let base_path = asset_manager.base_path();
    let registry = UserAssetRegistry::load(base_path);

    let entry = registry.assets.iter().find(|a| a.id == asset_id)
        .ok_or_else(|| format!("Asset '{}' not found", asset_id))?;

    let full_path = base_path.join("user-assets").join(&entry.file_path);
    if !full_path.exists() {
        return Err(format!("Asset file not found: {:?}", full_path));
    }

    Ok(full_path.to_string_lossy().to_string())
}

/// Find a user asset matching the given asset type and optional tags/mood
/// Used by BuiltinAssetProvider to check user assets first
pub fn find_user_asset(base_path: &std::path::Path, asset_type: &str, tags: &[String]) -> Option<PathBuf> {
    let registry = UserAssetRegistry::load(base_path);

    // Try to find an asset matching the type and at least one tag
    for entry in &registry.assets {
        if entry.asset_type != asset_type {
            continue;
        }
        if tags.is_empty() || entry.tags.iter().any(|t| tags.contains(t)) {
            let full_path = base_path.join("user-assets").join(&entry.file_path);
            if full_path.exists() {
                return Some(full_path);
            }
        }
    }

    // Fallback: any asset of the matching type
    for entry in &registry.assets {
        if entry.asset_type == asset_type {
            let full_path = base_path.join("user-assets").join(&entry.file_path);
            if full_path.exists() {
                return Some(full_path);
            }
        }
    }

    None
}
