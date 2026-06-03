use tauri::command;
use crate::types::game_script::{GameScript, GameType};
use crate::types::game_state::GameState;
use crate::engine::pipeline::GenerationPipeline;
use crate::engine::asset_manager::AssetManager;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
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
    pipeline: tauri::State<'_, Arc<RwLock<GenerationPipeline>>>,
) -> Result<GameInfo, String> {
    let gt = game_type.as_deref().and_then(|s| parse_game_type(s).ok());
    let p = pipeline.read().await;
    let (game_id, script) = p.create_game(&input, gt).await
        .map_err(|e| format!("{:?}", e))?;

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
pub async fn random_outline(_game_type: Option<String>, _themes: Vec<String>) -> Result<String, String> {
    Err("not implemented".to_string())
}

#[command]
pub async fn get_game(_game_id: String) -> Result<GameInfo, String> {
    Err("not implemented".to_string())
}

#[command]
pub async fn get_game_script(
    game_id: String,
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<GameScript, String> {
    asset_manager.load_game_script(&game_id)
}

#[command]
pub async fn list_games() -> Result<Vec<GameInfo>, String> {
    Err("not implemented".to_string())
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
