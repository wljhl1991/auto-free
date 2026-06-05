use tauri::command;
use crate::types::game_script::{GameScript, GameType};
use crate::types::game_state::GameState;
use crate::engine::pipeline::GenerationPipeline;
use crate::engine::asset_manager::AssetManager;
use crate::providers::ProviderError;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub id: String,
    pub title: String,
    pub game_type: GameType,
    pub total_chapters: usize,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveInfo {
    pub save_id: String,
    pub game_id: String,
    pub chapter_id: String,
    pub scene_id: String,
    pub created_at: u64,
}

fn parse_game_type(s: &str) -> Result<GameType, String> {
    match s.to_lowercase().as_str() {
        "visual_novel" | "visualnovel" => Ok(GameType::VisualNovel),
        "rpg" => Ok(GameType::Rpg),
        "mystery" => Ok(GameType::Mystery),
        "horror" => Ok(GameType::Horror),
        "simulation" => Ok(GameType::Simulation),
        _ => Err(format!("Unknown game type: {}", s)),
    }
}

#[command]
pub async fn create_game(
    input: String,
    game_type: Option<String>,
    use_local_fallback: Option<bool>,
    pipeline: tauri::State<'_, Arc<RwLock<GenerationPipeline>>>,
) -> Result<GameInfo, String> {
    let gt = game_type.as_deref().and_then(|s| parse_game_type(s).ok());
    let fallback = use_local_fallback.unwrap_or(true);
    log::info!("创建游戏: input_len={}, game_type={:?}, use_local_fallback={}", input.len(), gt, fallback);
    let p = pipeline.read().await;
    let (game_id, script) = p.create_game(&input, gt, fallback).await
        .map_err(|e| {
            let msg = match e {
                ProviderError::InvalidConfig(msg) => format!("配置错误：{}。请在设置中配置 AI 服务后重试。", msg),
                ProviderError::AuthFailed(_) => "认证失败：API Key 无效，请检查设置。".to_string(),
                ProviderError::NetworkError(msg) => format!("网络错误：{}。请检查网络连接。", msg),
                ProviderError::GenerationFailed(msg) => format!("生成失败：{}", msg),
                _ => format!("创建失败：{:?}", e),
            };
            log::error!("创建游戏失败: {}", msg);
            msg
        })?;

    log::info!("创建游戏成功: game_id={}, title={}", game_id, script.meta.title);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(GameInfo {
        id: game_id,
        title: script.meta.title,
        game_type: script.meta.game_type,
        total_chapters: script.meta.total_chapters as usize,
        created_at: now,
        updated_at: now,
    })
}

#[command]
pub async fn create_game_from_script(
    script_json: String,
    pipeline: tauri::State<'_, Arc<RwLock<GenerationPipeline>>>,
) -> Result<GameInfo, String> {
    let script: GameScript = serde_json::from_str(&script_json)
        .map_err(|e| format!("解析游戏脚本失败: {}", e))?;
    log::info!("从脚本创建游戏: title={}", script.meta.title);
    let p = pipeline.read().await;
    let (game_id, script) = p.create_game_from_script(script).await
        .map_err(|e| {
            let msg = match e {
                ProviderError::InvalidConfig(msg) => format!("配置错误：{}", msg),
                ProviderError::AuthFailed(_) => "认证失败：API Key 无效，请检查设置。".to_string(),
                ProviderError::NetworkError(msg) => format!("网络错误：{}", msg),
                ProviderError::GenerationFailed(msg) => format!("生成失败：{}", msg),
                _ => format!("创建失败：{:?}", e),
            };
            log::error!("从脚本创建游戏失败: {}", msg);
            msg
        })?;

    log::info!("从脚本创建游戏成功: game_id={}, title={}", game_id, script.meta.title);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(GameInfo {
        id: game_id,
        title: script.meta.title,
        game_type: script.meta.game_type,
        total_chapters: script.meta.total_chapters as usize,
        created_at: now,
        updated_at: now,
    })
}

#[command]
pub async fn random_outline(
    game_type: Option<String>,
    themes: Vec<String>,
    pipeline: tauri::State<'_, Arc<RwLock<GenerationPipeline>>>,
) -> Result<String, String> {
    let p = pipeline.read().await;
    p.generate_random_outline(game_type, themes).await.map_err(|e| format!("{:?}", e))
}

#[command]
pub async fn get_game(
    game_id: String,
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<GameInfo, String> {
    let script = asset_manager.load_game_script(&game_id)?;

    let script_path = asset_manager.get_game_asset_dir(&game_id).join("script.json");
    let metadata = std::fs::metadata(&script_path)
        .map_err(|e| format!("Failed to read script.json metadata: {}", e))?;

    let created_at = metadata.created()
        .or_else(|_| metadata.modified())
        .unwrap_or_else(|_| SystemTime::now())
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let updated_at = metadata.modified()
        .unwrap_or_else(|_| SystemTime::now())
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(GameInfo {
        id: game_id,
        title: script.meta.title,
        game_type: script.meta.game_type,
        total_chapters: script.meta.total_chapters as usize,
        created_at,
        updated_at,
    })
}

#[command]
pub async fn get_game_script(
    game_id: String,
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<GameScript, String> {
    asset_manager.load_game_script(&game_id)
}

#[command]
pub async fn list_games(
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<Vec<GameInfo>, String> {
    let games_dir = asset_manager.base_path().join("games");

    if !games_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&games_dir)
        .map_err(|e| format!("Failed to read games directory: {}", e))?;

    let mut games = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let game_id = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        let script_path = path.join("script.json");
        if !script_path.exists() {
            continue;
        }

        let script = match asset_manager.load_game_script(&game_id) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let metadata = match std::fs::metadata(&script_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let created_at = metadata.created()
            .or_else(|_| metadata.modified())
            .unwrap_or_else(|_| SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let updated_at = metadata.modified()
            .unwrap_or_else(|_| SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        games.push(GameInfo {
            id: game_id,
            title: script.meta.title,
            game_type: script.meta.game_type,
            total_chapters: script.meta.total_chapters as usize,
            created_at,
            updated_at,
        });
    }

    // 按更新时间降序排列
    games.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(games)
}

#[command]
pub async fn delete_game(_game_id: String) -> Result<(), String> {
    Err("not implemented".to_string())
}

#[command]
pub async fn save_game(_game_id: String, _state: GameState) -> Result<String, String> {
    Err("not implemented".to_string())
}

#[command]
pub async fn load_save(_game_id: String, _save_id: String) -> Result<GameState, String> {
    Err("not implemented".to_string())
}

#[command]
pub async fn list_saves(_game_id: String) -> Result<Vec<SaveInfo>, String> {
    Err("not implemented".to_string())
}
