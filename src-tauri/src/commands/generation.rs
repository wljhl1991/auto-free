use tauri::command;
use crate::engine::pipeline::GenerationPipeline;
use std::sync::Arc;
use tokio::sync::RwLock;

#[command]
pub async fn get_generation_status(
    game_id: String,
    pipeline: tauri::State<'_, Arc<RwLock<GenerationPipeline>>>,
) -> Result<crate::engine::pipeline::GenerationStatus, String> {
    let p = pipeline.read().await;
    p.get_status(&game_id).await
        .ok_or_else(|| format!("Generation status not found for game '{}'", game_id))
}

#[command]
pub async fn regenerate_asset(_game_id: String, _asset_ref_id: String) -> Result<(), String> {
    Err("not implemented".to_string())
}

#[command]
pub async fn export_game(_game_id: String, _output_path: String) -> Result<(), String> {
    Err("not implemented".to_string())
}
