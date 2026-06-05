use async_trait::async_trait;
use super::{IAssetProvider, ProviderError};
use crate::engine::asset_manager::BuiltinAssetRegistry;
use crate::types::game_script::{AssetRef, AssetType as ScriptAssetType};
use crate::types::asset::{LocalAsset, AIModality, AssetType, AssetSource};
use crate::types::ai_provider::{ConnectivityCheck, ConnectivityStatus};
use crate::commands::user_asset::find_user_asset;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct BuiltinAssetProvider {
    registry: BuiltinAssetRegistry,
    asset_base_path: PathBuf,
    autofree_base_path: PathBuf,
}

impl BuiltinAssetProvider {
    pub fn new(builtin_assets_path: PathBuf, asset_base_path: PathBuf, autofree_base_path: PathBuf) -> Self {
        let mut registry = BuiltinAssetRegistry::new(builtin_assets_path);
        registry.load();
        Self {
            registry,
            asset_base_path,
            autofree_base_path,
        }
    }

    /// 将 game_script 的 AssetType 转换为 asset 模块的 AssetType
    fn convert_asset_type(script_type: &ScriptAssetType) -> AssetType {
        match script_type {
            ScriptAssetType::Image => AssetType::Image,
            ScriptAssetType::Video => AssetType::Video,
            ScriptAssetType::Audio => AssetType::Audio,
            ScriptAssetType::Voice => AssetType::Voice,
        }
    }

    /// 从 prompt 推断情绪关键词
    fn infer_mood_from_prompt(prompt: &str) -> Option<&str> {
        let lower = prompt.to_lowercase();
        if lower.contains("calm") || lower.contains("peaceful") || lower.contains("relax") {
            Some("calm")
        } else if lower.contains("tense") || lower.contains("suspense") || lower.contains("mystery") {
            Some("tense")
        } else if lower.contains("dark") || lower.contains("horror") || lower.contains("scary") {
            Some("dark")
        } else if lower.contains("happy") || lower.contains("joy") || lower.contains("cheerful") {
            Some("happy")
        } else if lower.contains("battle") || lower.contains("fight") || lower.contains("combat") {
            Some("battle")
        } else {
            None
        }
    }

    /// 从 prompt 推断性别
    fn infer_gender_from_prompt(prompt: &str) -> Option<&str> {
        let lower = prompt.to_lowercase();
        if lower.contains("male") || lower.contains("man") || lower.contains("boy") {
            Some("male")
        } else if lower.contains("female") || lower.contains("woman") || lower.contains("girl") {
            Some("female")
        } else {
            None
        }
    }

    /// 根据 AssetRef 查找匹配的内置资源（优先使用用户导入资源）
    fn find_builtin_asset(&self, asset_ref: &AssetRef) -> Option<crate::engine::asset_manager::BuiltinAssetEntry> {
        // 优先使用 builtin_asset_id 精确匹配
        if let Some(ref builtin_id) = asset_ref.builtin_asset_id {
            if let Some(entry) = self.registry.get_by_id(builtin_id) {
                return Some(entry.clone());
            }
        }

        // 根据 asset_type 进行匹配
        match asset_ref.asset_type {
            ScriptAssetType::Image => {
                // 尝试按场景图匹配
                let mood = Self::infer_mood_from_prompt(&asset_ref.prompt);
                // 尝试所有游戏类型
                let game_types = [
                    crate::types::game_script::GameType::VisualNovel,
                    crate::types::game_script::GameType::Mystery,
                    crate::types::game_script::GameType::Horror,
                    crate::types::game_script::GameType::Rpg,
                    crate::types::game_script::GameType::Simulation,
                ];
                for game_type in &game_types {
                    if let Some(entry) = self.registry.find_image(game_type, mood) {
                        return Some(entry.clone());
                    }
                }
                // 尝试按头像匹配
                if let Some(gender) = Self::infer_gender_from_prompt(&asset_ref.prompt) {
                    if let Some(entry) = self.registry.find_portrait(gender) {
                        return Some(entry.clone());
                    }
                }
            }
            ScriptAssetType::Audio => {
                // 尝试按 BGM 匹配
                if let Some(mood) = Self::infer_mood_from_prompt(&asset_ref.prompt) {
                    if let Some(entry) = self.registry.find_music(mood) {
                        return Some(entry.clone());
                    }
                }
                // 尝试按音效匹配
                let lower = asset_ref.prompt.to_lowercase();
                if lower.contains("click") {
                    if let Some(entry) = self.registry.find_sfx("click") {
                        return Some(entry.clone());
                    }
                } else if lower.contains("transition") {
                    if let Some(entry) = self.registry.find_sfx("transition") {
                        return Some(entry.clone());
                    }
                }
                // 回退：尝试任意 BGM
                for mood in &["calm", "tense", "dark", "happy", "battle"] {
                    if let Some(entry) = self.registry.find_music(mood) {
                        return Some(entry.clone());
                    }
                }
            }
            ScriptAssetType::Video | ScriptAssetType::Voice => {
                // 内置资源暂不支持视频和语音，返回 None
            }
        }
        None
    }

    /// 尝试从用户导入资源中查找匹配的资源
    fn find_user_asset_path(&self, asset_ref: &AssetRef) -> Option<PathBuf> {
        let (asset_type_str, tags) = match asset_ref.asset_type {
            ScriptAssetType::Image => {
                let mood = Self::infer_mood_from_prompt(&asset_ref.prompt);
                let tags = mood.map(|m| vec![m.to_string()]).unwrap_or_default();
                ("image", tags)
            }
            ScriptAssetType::Audio => {
                let mood = Self::infer_mood_from_prompt(&asset_ref.prompt);
                let tags = mood.map(|m| vec![m.to_string()]).unwrap_or_default();
                ("music", tags)
            }
            ScriptAssetType::Video => ("video", vec![]),
            ScriptAssetType::Voice => ("voice", vec![]),
        };
        find_user_asset(&self.autofree_base_path, asset_type_str, &tags)
    }

    /// 生成 cacheKey
    fn generate_cache_key(asset_ref: &AssetRef) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(asset_ref.id.as_bytes());
        hasher.update(asset_ref.prompt.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }
}

#[async_trait]
impl IAssetProvider for BuiltinAssetProvider {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError> {
        let cache_key = asset_ref.cache_key.clone()
            .unwrap_or_else(|| Self::generate_cache_key(asset_ref));

        // 优先检查用户导入资源
        if let Some(user_path) = self.find_user_asset_path(asset_ref) {
            if user_path.exists() {
                // 目标路径：asset_base_path / cache_key / 文件名
                let dest_dir = self.asset_base_path.join(&cache_key);
                std::fs::create_dir_all(&dest_dir)
                    .map_err(|e| ProviderError::GenerationFailed(format!("Failed to create asset dir: {}", e)))?;

                let file_name = user_path.file_name()
                    .ok_or_else(|| ProviderError::GenerationFailed("Invalid source file name".to_string()))?;
                let dest_path = dest_dir.join(file_name);

                // 如果目标文件已存在且 cacheKey 匹配，直接返回
                if dest_path.exists() {
                    return Ok(LocalAsset {
                        id: asset_ref.id.clone(),
                        asset_type: Self::convert_asset_type(&asset_ref.asset_type),
                        local_path: dest_path.to_string_lossy().to_string(),
                        source: AssetSource::Builtin,
                        cache_key,
                        created_at: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    });
                }

                // 复制文件到目标路径
                std::fs::copy(&user_path, &dest_path)
                    .map_err(|e| ProviderError::GenerationFailed(format!("Failed to copy user asset: {}", e)))?;

                return Ok(LocalAsset {
                    id: asset_ref.id.clone(),
                    asset_type: Self::convert_asset_type(&asset_ref.asset_type),
                    local_path: dest_path.to_string_lossy().to_string(),
                    source: AssetSource::Builtin,
                    cache_key,
                    created_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
            }
        }

        // 回退到内置资源
        let entry = self.find_builtin_asset(asset_ref)
            .ok_or_else(|| ProviderError::NotFound(format!(
                "No builtin asset found for asset_ref: {} (type: {:?}, prompt: {})",
                asset_ref.id, asset_ref.asset_type, asset_ref.prompt
            )))?;

        let source_path = self.registry.get_full_path(&entry);
        if !source_path.exists() {
            return Err(ProviderError::NotFound(format!(
                "Builtin asset file not found: {:?}",
                source_path
            )));
        }

        // 目标路径：asset_base_path / cache_key / 文件名
        let dest_dir = self.asset_base_path.join(&cache_key);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| ProviderError::GenerationFailed(format!("Failed to create asset dir: {}", e)))?;

        let file_name = source_path.file_name()
            .ok_or_else(|| ProviderError::GenerationFailed("Invalid source file name".to_string()))?;
        let dest_path = dest_dir.join(file_name);

        // 如果目标文件已存在且 cacheKey 匹配，直接返回
        if dest_path.exists() {
            return Ok(LocalAsset {
                id: asset_ref.id.clone(),
                asset_type: Self::convert_asset_type(&asset_ref.asset_type),
                local_path: dest_path.to_string_lossy().to_string(),
                source: AssetSource::Builtin,
                cache_key,
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }

        // 复制文件到目标路径
        std::fs::copy(&source_path, &dest_path)
            .map_err(|e| ProviderError::GenerationFailed(format!("Failed to copy builtin asset: {}", e)))?;

        Ok(LocalAsset {
            id: asset_ref.id.clone(),
            asset_type: Self::convert_asset_type(&asset_ref.asset_type),
            local_path: dest_path.to_string_lossy().to_string(),
            source: AssetSource::Builtin,
            cache_key,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }

    async fn check_connectivity(&self) -> Result<ConnectivityCheck, ProviderError> {
        // 内置 Provider 始终返回 ok（无需网络）
        Ok(ConnectivityCheck {
            provider_id: "builtin".to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            status: ConnectivityStatus::Ok,
            latency: Some(0),
            error_message: None,
            quota_info: None,
                response_preview: None,
                test_prompt: None,
        })
    }

    fn supported_modalities(&self) -> Vec<AIModality> {
        // 内置资源覆盖所有模态
        vec![
            AIModality::Text,
            AIModality::Image,
            AIModality::Video,
            AIModality::Music,
            AIModality::Voice,
        ]
    }

    fn provider_id(&self) -> &str {
        "builtin"
    }
}
