use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::game_script::GameScript;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChapterGenerationStatus {
    Generating,
    Ready,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationProgress {
    pub total_assets: u32,
    pub completed_assets: u32,
    pub failed_assets: u32,
    pub chapter_status: HashMap<String, ChapterGenerationStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceRecord {
    pub choice_node_id: String,
    pub selected_option_index: u32,
    pub selected_option_text: String,
    pub timestamp: u64,
    pub chapter_id: String,
    pub scene_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub save_id: String,
    pub game_script: GameScript,
    pub current_chapter_id: String,
    pub current_scene_id: String,
    pub current_node_id: String,
    pub variables: HashMap<String, serde_json::Value>,
    pub inventory: Vec<String>,
    pub stats: HashMap<String, f64>,
    pub choice_history: Vec<ChoiceRecord>,
    pub visited_scenes: Vec<String>,
    pub unlocked_cgs: Vec<String>,
    pub generation_progress: GenerationProgress,
}
