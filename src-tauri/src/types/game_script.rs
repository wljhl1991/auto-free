use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum GameType {
    #[default]
    VisualNovel,
    Rpg,
    Mystery,
    Horror,
    Simulation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameMeta {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub game_type: GameType,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub total_chapters: u32,
    #[serde(default)]
    pub themes: Vec<String>,
    #[serde(default)]
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameScript {
    #[serde(default)]
    pub meta: GameMeta,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    pub global_variables: Vec<VariableDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub chapter_variables: Vec<VariableDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub assets: SceneAssets,
    #[serde(default)]
    pub sequence: Vec<SceneNode>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SceneAssets {
    pub background_image: Option<AssetRef>,
    pub background_video: Option<AssetRef>,
    pub bgm: Option<AssetRef>,
    pub ambient_sound: Option<AssetRef>,
    pub cg_animation: Option<AssetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    #[default]
    Image,
    Video,
    Audio,
    Voice,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssetSource {
    #[default]
    AiGenerated,
    Builtin,
    LocalFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    #[default]
    Pending,
    Generating,
    Ready,
    Failed,
    Fallback,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AssetRef {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default)]
    pub asset_type: AssetType,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub source: AssetSource,
    #[serde(default)]
    pub status: AssetStatus,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub builtin_asset_id: Option<String>,
    #[serde(default)]
    pub cache_key: Option<String>,
}

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
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub voice_prompt: Option<String>,
    #[serde(default)]
    pub voice_asset: Option<AssetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub speaker: String,
    #[serde(default)]
    pub speaker_avatar: Option<AssetRef>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub voice_asset: Option<AssetRef>,
    #[serde(default)]
    pub emotion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub options: Vec<ChoiceOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub condition: Option<Condition>,
    #[serde(default)]
    pub true_branch: String,
    #[serde(default)]
    pub false_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    #[default]
    SetVariable,
    AddItem,
    RemoveItem,
    ChangeScene,
    TriggerEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub action_type: ActionKind,
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub next_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CGNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub video_asset: AssetRef,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub skip_allowed: bool,
    #[serde(default)]
    pub next_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    #[default]
    Fade,
    Dissolve,
    Slide,
    Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneTransitionNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub target_scene_id: String,
    #[serde(default)]
    pub transition_type: TransitionKind,
    #[serde(default)]
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub next_node_id: Option<String>,
    #[serde(default)]
    pub effects: Option<Vec<Effect>>,
    #[serde(default)]
    pub condition: Option<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    #[default]
    Number,
    String,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableDef {
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
    pub var_type: VariableType,
    #[serde(default)]
    pub default_value: serde_json::Value,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    #[default]
    SetVariable,
    AddItem,
    RemoveItem,
    ModifyStat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    #[serde(rename = "type", default)]
    pub effect_type: EffectKind,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConditionKind {
    #[default]
    VariableCheck,
    ItemCheck,
    StatCheck,
    Composite,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    #[default]
    NotHas,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    #[serde(rename = "type", default)]
    pub condition_type: ConditionKind,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub operator: ConditionOperator,
    #[serde(default)]
    pub value: serde_json::Value,
    #[serde(default)]
    pub and: Option<Vec<Condition>>,
    #[serde(default)]
    pub or: Option<Vec<Condition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    #[serde(default)]
    pub from_scene_id: String,
    #[serde(default)]
    pub to_scene_id: String,
    #[serde(rename = "type", default)]
    pub transition_type: TransitionKind,
    #[serde(default)]
    pub duration: f64,
}
