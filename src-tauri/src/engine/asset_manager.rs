use crate::types::asset::AssetType;
use crate::types::game_script::{GameScript, GameType};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinAssetEntry {
    pub id: String,
    pub asset_type: AssetType,
    pub game_type: Option<GameType>,
    pub mood: Option<String>,
    pub relative_path: String,
    pub description: String,
}

pub struct BuiltinAssetRegistry {
    assets: Vec<BuiltinAssetEntry>,
    base_path: PathBuf,
}

impl BuiltinAssetRegistry {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            assets: Vec::new(),
            base_path,
        }
    }

    /// 扫描 builtin-assets/ 目录加载资源列表
    pub fn load(&mut self) {
        self.assets.clear();

        // 注册场景图
        let image_categories = [
            ("visual_novel", GameType::VisualNovel),
            ("mystery", GameType::Mystery),
            ("horror", GameType::Horror),
            ("rpg", GameType::Rpg),
            ("simulation", GameType::Simulation),
        ];

        for (category, game_type) in &image_categories {
            let category_path = self.base_path.join("images").join(category);
            if let Ok(entries) = std::fs::read_dir(&category_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "png") {
                        let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
                        let relative = format!("images/{}/{}.png", category, file_name);
                        let mood = Self::infer_mood_from_name(&file_name);
                        self.assets.push(BuiltinAssetEntry {
                            id: format!("builtin_{}_{}", category, file_name),
                            asset_type: AssetType::Image,
                            game_type: Some(game_type.clone()),
                            mood,
                            relative_path: relative,
                            description: format!("{} style scene: {}", category, file_name),
                        });
                    }
                }
            }
        }

        // 注册 BGM
        let music_moods = ["calm", "tense", "dark", "happy", "battle"];
        for mood in &music_moods {
            let relative = format!("music/{}.mp3", mood);
            let full_path = self.base_path.join(&relative);
            if full_path.exists() {
                self.assets.push(BuiltinAssetEntry {
                    id: format!("builtin_music_{}", mood),
                    asset_type: AssetType::Audio,
                    game_type: None,
                    mood: Some(mood.to_string()),
                    relative_path: relative,
                    description: format!("BGM: {} mood", mood),
                });
            }
        }

        // 注册音效
        let sfx_names = ["click", "transition"];
        for name in &sfx_names {
            let relative = format!("sfx/{}.mp3", name);
            let full_path = self.base_path.join(&relative);
            if full_path.exists() {
                self.assets.push(BuiltinAssetEntry {
                    id: format!("builtin_sfx_{}", name),
                    asset_type: AssetType::Audio,
                    game_type: None,
                    mood: None,
                    relative_path: relative,
                    description: format!("SFX: {}", name),
                });
            }
        }

        // 注册头像
        let genders = ["male", "female"];
        for gender in &genders {
            let portrait_path = self.base_path.join("portraits").join(gender);
            if let Ok(entries) = std::fs::read_dir(&portrait_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "png") {
                        let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
                        let relative = format!("portraits/{}/{}.png", gender, file_name);
                        self.assets.push(BuiltinAssetEntry {
                            id: format!("builtin_portrait_{}_{}", gender, file_name),
                            asset_type: AssetType::Image,
                            game_type: None,
                            mood: None,
                            relative_path: relative,
                            description: format!("Portrait: {} {}", gender, file_name),
                        });
                    }
                }
            }
        }
    }

    /// 根据游戏类型和情绪查找场景图
    pub fn find_image(&self, game_type: &GameType, mood: Option<&str>) -> Option<&BuiltinAssetEntry> {
        self.assets.iter().find(|a| {
            a.asset_type == AssetType::Image
                && a.game_type.as_ref() == Some(game_type)
                && mood.map_or(true, |m| a.mood.as_deref() == Some(m))
        })
    }

    /// 根据情绪查找 BGM
    pub fn find_music(&self, mood: &str) -> Option<&BuiltinAssetEntry> {
        self.assets.iter().find(|a| {
            a.asset_type == AssetType::Audio
                && a.relative_path.starts_with("music/")
                && a.mood.as_deref() == Some(mood)
        })
    }

    /// 根据性别查找头像
    pub fn find_portrait(&self, gender: &str) -> Option<&BuiltinAssetEntry> {
        self.assets.iter().find(|a| {
            a.relative_path.starts_with(&format!("portraits/{}/", gender))
        })
    }

    /// 根据名称查找音效
    pub fn find_sfx(&self, name: &str) -> Option<&BuiltinAssetEntry> {
        self.assets.iter().find(|a| {
            a.relative_path.starts_with("sfx/")
                && a.id == format!("builtin_sfx_{}", name)
        })
    }

    /// 根据 ID 查找资源
    pub fn get_by_id(&self, id: &str) -> Option<&BuiltinAssetEntry> {
        self.assets.iter().find(|a| a.id == id)
    }

    /// 获取资源的完整路径
    pub fn get_full_path(&self, entry: &BuiltinAssetEntry) -> PathBuf {
        self.base_path.join(&entry.relative_path)
    }

    /// 列出所有内置资源
    pub fn list_all(&self) -> &[BuiltinAssetEntry] {
        &self.assets
    }

    /// 从文件名推断情绪标签
    fn infer_mood_from_name(name: &str) -> Option<String> {
        let lower = name.to_lowercase();
        if lower.contains("calm") || lower.contains("vn_scene_1") || lower.contains("sim_scene_1") {
            Some("calm".to_string())
        } else if lower.contains("tense") || lower.contains("mystery_scene_1") || lower.contains("rpg_scene_2") {
            Some("tense".to_string())
        } else if lower.contains("dark") || lower.contains("horror") {
            Some("dark".to_string())
        } else if lower.contains("happy") || lower.contains("sim_scene_2") || lower.contains("vn_scene_2") {
            Some("happy".to_string())
        } else if lower.contains("battle") || lower.contains("rpg_scene_3") {
            Some("battle".to_string())
        } else {
            None
        }
    }
}

/// 资源管理器 — 管理本地资源存储和缓存
pub struct AssetManager {
    base_path: PathBuf,  // gen/
    #[allow(dead_code)]
    cache_path: PathBuf, // gen/cache/
}

impl AssetManager {
    pub fn new(base_path: PathBuf) -> Self {
        let cache_path = base_path.join("cache");
        Self { base_path, cache_path }
    }

    /// 获取游戏目录
    pub fn get_game_dir(&self, game_id: &str) -> PathBuf {
        self.base_path.join("games").join(game_id)
    }

    /// 获取游戏资源目录
    pub fn get_game_asset_dir(&self, game_id: &str) -> PathBuf {
        self.get_game_dir(game_id).join("assets")
    }

    /// 确保游戏目录存在
    pub fn ensure_game_dirs(&self, game_id: &str) -> Result<(), String> {
        let _game_dir = self.get_game_dir(game_id);
        let asset_dir = self.get_game_asset_dir(game_id);
        std::fs::create_dir_all(&asset_dir)
            .map_err(|e| format!("Failed to create game directories: {}", e))?;
        Ok(())
    }

    /// 获取基础路径
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// 保存 GameScript 到游戏目录的 script.json
    pub fn save_game_script(&self, game_id: &str, script: &GameScript) -> Result<(), String> {
        self.ensure_game_dirs(game_id)?;
        let script_path = self.get_game_dir(game_id).join("script.json");
        let json = serde_json::to_string_pretty(script)
            .map_err(|e| format!("Failed to serialize GameScript: {}", e))?;
        std::fs::write(&script_path, json)
            .map_err(|e| format!("Failed to write script.json: {}", e))?;
        Ok(())
    }

    /// 从游戏目录加载 GameScript
    pub fn load_game_script(&self, game_id: &str) -> Result<GameScript, String> {
        let script_path = self.get_game_dir(game_id).join("script.json");
        let json = std::fs::read_to_string(&script_path)
            .map_err(|e| format!("Failed to read script.json: {}", e))?;
        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse GameScript: {}", e))
    }

    /// 修复游戏资源：把错误位置的资源移动到 assets 目录，并更新 GameScript
    pub fn repair_game(&self, game_id: &str) -> Result<usize, String> {
        self.ensure_game_dirs(game_id)?;
        
        let mut script = self.load_game_script(game_id)?;
        let mut assets_moved = 0;
        
        // 辅助函数：移动资源并更新 AssetRef
        let mut repair_asset_ref = |ar: &mut crate::types::game_script::AssetRef| {
            if let Some(old_url) = &ar.url {
                let old_path = PathBuf::from(old_url);
                
                // 确定新文件名
                let file_ext = match ar.asset_type {
                    crate::types::game_script::AssetType::Image => "png",
                    crate::types::game_script::AssetType::Video => "mp4",
                    crate::types::game_script::AssetType::Audio | crate::types::game_script::AssetType::Voice => "mp3",
                };
                
                let new_path = self.get_game_asset_dir(game_id).join(format!("{}.{}", ar.id, file_ext));
                
                // 如果旧文件存在，且不是已经在正确位置，则移动
                if old_path.exists() && old_path != new_path {
                    // 确保目标目录存在
                    if let Some(parent) = new_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    
                    match std::fs::copy(&old_path, &new_path) {
                        Ok(_) => {
                            // 更新 AssetRef 的 URL
                            ar.url = Some(new_path.to_string_lossy().to_string());
                            assets_moved += 1;
                        }
                        Err(e) => {
                            log::warn!("Failed to move asset {}: {:?}", ar.id, e);
                        }
                    }
                }
            }
        };
        
        // 遍历所有场景和节点，修复所有 AssetRef
        for chapter in &mut script.chapters {
            for scene in &mut chapter.scenes {
                // 修复场景资源
                if let Some(ref mut bg) = scene.assets.background_image {
                    repair_asset_ref(bg);
                }
                if let Some(ref mut bv) = scene.assets.background_video {
                    repair_asset_ref(bv);
                }
                if let Some(ref mut bgm) = scene.assets.bgm {
                    repair_asset_ref(bgm);
                }
                if let Some(ref mut amb) = scene.assets.ambient_sound {
                    repair_asset_ref(amb);
                }
                if let Some(ref mut cg) = scene.assets.cg_animation {
                    repair_asset_ref(cg);
                }
                
                // 修复节点资源
                for node in &mut scene.sequence {
                    match node {
                        crate::types::game_script::SceneNode::Dialogue(d) => {
                            if let Some(ref mut sa) = d.speaker_avatar {
                                repair_asset_ref(sa);
                            }
                            if let Some(ref mut va) = d.voice_asset {
                                repair_asset_ref(va);
                            }
                        }
                        crate::types::game_script::SceneNode::Narration(n) => {
                            if let Some(ref mut va) = n.voice_asset {
                                repair_asset_ref(va);
                            }
                        }
                        crate::types::game_script::SceneNode::Cg(c) => {
                            repair_asset_ref(&mut c.video_asset);
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // 保存修复后的 GameScript
        self.save_game_script(game_id, &script)?;
        
        Ok(assets_moved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = BuiltinAssetRegistry::new(PathBuf::from("/tmp/builtin-assets"));
        assert!(registry.list_all().is_empty());
    }

    #[test]
    fn test_infer_mood() {
        assert_eq!(BuiltinAssetRegistry::infer_mood_from_name("calm_bg"), Some("calm".to_string()));
        assert_eq!(BuiltinAssetRegistry::infer_mood_from_name("horror_scene_1"), Some("dark".to_string()));
        assert_eq!(BuiltinAssetRegistry::infer_mood_from_name("battle_theme"), Some("battle".to_string()));
    }
}
