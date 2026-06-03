use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AIModality {
    Text,
    Image,
    Video,
    Music,
    Voice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[serde(rename_all = "camelCase")]
pub struct LocalAsset {
    pub id: String,
    #[serde(rename = "type")]
    pub asset_type: AssetType,
    pub local_path: String,
    pub source: AssetSource,
    pub cache_key: String,
    pub created_at: u64,
}
