use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameType {
    VisualNovel,
    Rpg,
    Mystery,
    Horror,
    Simulation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameMeta {
    pub title: String,
    pub game_type: GameType,
    pub description: String,
    pub total_chapters: u32,
    pub themes: Vec<String>,
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameScript {
    pub meta: GameMeta,
    pub chapters: Vec<Chapter>,
    pub global_variables: Vec<VariableDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub scenes: Vec<Scene>,
    pub chapter_variables: Vec<VariableDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: String,
    pub title: String,
    pub description: String,
    pub assets: SceneAssets,
    pub sequence: Vec<SceneNode>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneAssets {
    pub background_image: Option<AssetRef>,
    pub background_video: Option<AssetRef>,
    pub bgm: Option<AssetRef>,
    pub ambient_sound: Option<AssetRef>,
    pub cg_animation: Option<AssetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Image,
    Video,
    Audio,
    Voice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetSource {
    AiGenerated,
    Builtin,
    LocalFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    Pending,
    Generating,
    Ready,
    Failed,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRef {
    pub id: String,
    #[serde(rename = "type")]
    pub asset_type: AssetType,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub style: Option<String>,
    pub source: AssetSource,
    pub status: AssetStatus,
    pub url: Option<String>,
    pub builtin_asset_id: Option<String>,
    pub cache_key: Option<String>,
}

// SceneNode 联合类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneNode {
    Narration(NarrationNode),
    Dialogue(DialogueNode),
    Choice(ChoiceNode),
    Condition(ConditionNode),
    Action(ActionNode),
    Cg(CGNode),
    SceneTransition(SceneTransitionNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrationNode {
    pub id: String,
    pub text: String,
    pub voice_prompt: Option<String>,
    pub voice_asset: Option<AssetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueNode {
    pub id: String,
    pub speaker: String,
    pub speaker_avatar: Option<AssetRef>,
    pub text: String,
    pub voice_asset: Option<AssetRef>,
    pub emotion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceNode {
    pub id: String,
    pub prompt: String,
    pub options: Vec<ChoiceOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionNode {
    pub id: String,
    pub condition: Condition,
    pub true_branch: String,
    pub false_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    SetVariable,
    AddItem,
    RemoveItem,
    ChangeScene,
    TriggerEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionNode {
    pub id: String,
    pub action_type: ActionKind,
    pub params: HashMap<String, serde_json::Value>,
    pub next_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CGNode {
    pub id: String,
    pub description: String,
    pub video_asset: AssetRef,
    pub duration: Option<f64>,
    pub skip_allowed: bool,
    pub next_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    Fade,
    Dissolve,
    Slide,
    Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneTransitionNode {
    pub id: String,
    pub target_scene_id: String,
    pub transition_type: TransitionKind,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    pub text: String,
    pub next_node_id: Option<String>,
    pub effects: Option<Vec<Effect>>,
    pub condition: Option<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    Number,
    String,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableDef {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: VariableType,
    pub default_value: serde_json::Value,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    SetVariable,
    AddItem,
    RemoveItem,
    ModifyStat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    #[serde(rename = "type")]
    pub effect_type: EffectKind,
    pub target: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionKind {
    VariableCheck,
    ItemCheck,
    StatCheck,
    Composite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    #[serde(rename = "==")]
    Eq,
    #[serde(rename = "!=")]
    Ne,
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = ">=")]
    Ge,
    #[serde(rename = "<=")]
    Le,
    Has,
    NotHas,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    #[serde(rename = "type")]
    pub condition_type: ConditionKind,
    pub target: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
    pub and: Option<Vec<Condition>>,
    pub or: Option<Vec<Condition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub from_scene_id: String,
    pub to_scene_id: String,
    #[serde(rename = "type")]
    pub transition_type: TransitionKind,
    pub duration: f64,
}
