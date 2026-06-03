use crate::types::asset::AssetType;
use crate::types::game_script::GameType;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
