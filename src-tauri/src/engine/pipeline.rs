use crate::config::manager::ConfigManager;
use crate::engine::outline_parser::OutlineParser;
use crate::engine::asset_manager::AssetManager;
use crate::providers::{IAssetProvider, ProviderFactory, ProviderError};
use crate::providers::builtin::BuiltinAssetProvider;
use crate::providers::deepseek::DeepSeekProvider;
use crate::types::game_script::{
    GameScript, GameType, AssetRef, AssetType as ScriptAssetType,
    AssetSource as ScriptAssetSource, AssetStatus, SceneNode,
};
use crate::types::asset::{LocalAsset, AIModality};
use crate::types::ai_provider::ProviderStatus;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::AppHandle;
use tauri::Emitter;
use uuid::Uuid;

/// 生成状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerationStatus {
    pub game_id: String,
    pub total_assets: usize,
    pub completed_assets: usize,
    pub failed_assets: usize,
    pub chapter_status: HashMap<String, ChapterStatus>,
    pub overall_progress: f32,
    pub first_chapter_ready: bool,
    pub background_generation_active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChapterStatus {
    pub chapter_id: String,
    pub chapter_title: String,
    pub total_assets: usize,
    pub completed_assets: usize,
    pub status: String,
}

pub struct GenerationPipeline {
    config_manager: Arc<RwLock<ConfigManager>>,
    asset_manager: Arc<AssetManager>,
    app_handle: Option<AppHandle>,
    statuses: Arc<RwLock<HashMap<String, GenerationStatus>>>,
    /// 存储 game_id -> GameScript，供 regenerate_asset 使用
    scripts: RwLock<HashMap<String, GameScript>>,
}

impl GenerationPipeline {
    pub fn new(config_manager: Arc<RwLock<ConfigManager>>, asset_manager: Arc<AssetManager>) -> Self {
        Self {
            config_manager,
            asset_manager,
            app_handle: None,
            statuses: Arc::new(RwLock::new(HashMap::new())),
            scripts: RwLock::new(HashMap::new()),
        }
    }

    pub fn set_app_handle(&mut self, handle: AppHandle) {
        self.app_handle = Some(handle);
    }

    /// 按游戏类型随机生成游戏大纲
    pub async fn generate_random_outline(
        &self,
        game_type: Option<String>,
        themes: Vec<String>,
    ) -> Result<String, ProviderError> {
        // 1. 如果未指定游戏类型，随机选择一个
        let game_type = game_type.unwrap_or_else(|| {
            let types = ["visual_novel", "rpg", "mystery", "horror", "simulation"];
            let idx = rand::thread_rng().gen_range(0..types.len());
            types[idx].to_string()
        });

        // 2. 构造随机生成 Prompt
        let prompt = self.build_random_outline_prompt(&game_type, &themes);

        // 3. 尝试调用文本 AI 生成大纲
        let config_manager = self.config_manager.read().await;
        let config = config_manager.get_config();

        // 优先查找 DeepSeek provider
        let ds_config = config
            .providers
            .iter()
            .find(|p| p.vendor == "deepseek" && p.status == ProviderStatus::Connected)
            .or_else(|| config.providers.iter().find(|p| p.vendor == "deepseek"))
            .cloned();

        // 其次查找任意支持 Text 模态的 provider
        let text_config = if ds_config.is_none() {
            config
                .providers
                .iter()
                .find(|p| {
                    p.modality.contains(&AIModality::Text) && p.status == ProviderStatus::Connected
                })
                .cloned()
        } else {
            None
        };

        drop(config_manager);

        if let Some(provider_config) = ds_config {
            let deepseek = DeepSeekProvider::new(&provider_config, self.asset_manager.base_path())?;
            let messages = vec![
                crate::providers::deepseek::ChatMessage {
                    role: "system".to_string(),
                    content: "你是一个创意游戏设计师。".to_string(),
                },
                crate::providers::deepseek::ChatMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ];
            let outline = deepseek.chat(messages, None).await?;
            Ok(outline)
        } else if let Some(provider_config) = text_config {
            // 对于非 DeepSeek 的文本 provider，也使用 DeepSeek 的 chat 接口
            // 因为其他文本 provider 可能也兼容 OpenAI API 格式
            let deepseek = DeepSeekProvider::new(&provider_config, self.asset_manager.base_path())?;
            let messages = vec![
                crate::providers::deepseek::ChatMessage {
                    role: "system".to_string(),
                    content: "你是一个创意游戏设计师。".to_string(),
                },
                crate::providers::deepseek::ChatMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ];
            let outline = deepseek.chat(messages, None).await?;
            Ok(outline)
        } else {
            // 4. 文本 AI 未配置，使用预设的示例大纲
            Ok(self.fallback_random_outline(&game_type))
        }
    }

    /// 构造随机大纲生成 Prompt
    fn build_random_outline_prompt(&self, game_type: &str, themes: &[String]) -> String {
        let game_type_display = match game_type {
            "visual_novel" => "视觉小说",
            "rpg" => "RPG",
            "mystery" => "悬疑解谜",
            "horror" => "恐怖生存",
            "simulation" => "模拟经营",
            _ => game_type,
        };

        let themes_str = if themes.is_empty() {
            "随机选择".to_string()
        } else {
            themes.join("、")
        };

        let moods = ["神秘", "温馨", "紧张", "荒诞", "浪漫", "黑暗", "欢快", "忧郁"];
        let mood = moods[rand::thread_rng().gen_range(0..moods.len())];

        format!(
            r#"请根据以下要求随机生成一个有趣的游戏大纲。

游戏类型：{game_type_display}
主题关键词：{themes_str}
氛围：{mood}

要求：
1. 生成一个引人入胜的游戏名称
2. 设计 3 个章节，每章有独特的主题和冲突
3. 创建 3-5 个有深度的角色
4. 每章至少一个关键选择分支
5. 设计至少 2 个不同结局
6. 确保故事有悬念和转折

输出格式：
游戏名称：...
类型：...
描述：...

第一章：[章节名]
- 场景1：...
- 场景2：...
- 关键选择：...

第二章：[章节名]
...

第三章：[章节名]
...

结局：
- 结局A：...
- 结局B：..."#,
            game_type_display = game_type_display,
            themes_str = themes_str,
            mood = mood,
        )
    }

    /// 文本 AI 未配置时的预设示例大纲
    fn fallback_random_outline(&self, game_type: &str) -> String {
        let outlines: &[(&str, &str)] = match game_type {
            "visual_novel" => &[
                ("樱花物语", "一个转学生来到樱花飘落的小镇，在古老的校园中遇见了三位性格迥异的少女。随着季节流转，他逐渐发现校园中隐藏的关于「时间回溯」的秘密，而每一次选择都将改变他与她们之间的命运。"),
                ("星尘记忆", "在一座漂浮于云端的城市里，一位失去记忆的少年遇到了自称是他恋人的少女。随着记忆碎片逐渐拼凑，他发现两人之间存在着跨越三生三世的约定，而这次是最后的机会。"),
            ],
            "rpg" => &[
                ("裂隙守护者", "世界之间的裂隙正在扩大，怪物从裂缝中不断涌出。你是一名被选中的守护者，需要穿越五个不同的领域，寻找封印裂隙的古老神器。每个领域都有独特的文明和挑战等待着你。"),
                ("龙魂觉醒", "你是一名普通的铁匠学徒，却在一次意外中觉醒了远古龙族的血脉。为了阻止黑暗龙的复活，你必须踏上寻找龙魂碎片的旅途，招募同伴，揭开大陆被遗忘的历史。"),
            ],
            "mystery" => &[
                ("消失的第十三天", "一座偏远的灯塔中，看守人每隔十二天就会消失一天，而他自己对此毫无记忆。当一位侦探前来调查时，发现灯塔中隐藏着通往平行世界的入口，而另一个自己正在试图取代他。"),
                ("回声庄园", "六位互不相识的人收到了同一封邀请函，来到一座古老的庄园。每当午夜钟声响起，就会有一人消失，而剩下的墙壁上会多出一幅画像。他们必须在黎明前找出庄园主人的真实身份。"),
            ],
            "horror" => &[
                ("深渊医院", "你在一座废弃的地下医院中醒来，身上插满了管子，记忆全无。走廊深处传来拖拽声，墙壁上的旧病历记录着一项关于「永生」的禁忌实验——而你可能就是实验体7号。"),
                ("镜中世界", "你发现自己家中的镜子开始映出不存在的房间。当你伸手触碰镜面时，一只冰冷的手从镜中伸出将你拉入。镜中世界与现实完全相反，而你的镜像已经取代了你的生活。"),
            ],
            "simulation" => &[
                ("星际酒馆", "在银河系边缘的小行星带上，你继承了一家破旧的太空酒馆。各路星际旅人、赏金猎人、走私商人来来往往，你需要经营酒馆、收集情报、结交盟友，在星际势力的夹缝中生存发展。"),
                ("浮岛物语", "你在一座漂浮于云海之上的小岛上醒来，身边只有一把锄头和几颗种子。通过耕种、建造和探索，将荒芜的浮岛发展成繁荣的空中家园，并揭开浮岛文明消失的真相。"),
            ],
            _ => &[
                ("未知旅途", "一段充满未知的冒险旅程，你的每一个选择都将影响故事的走向。在这条路上，你会遇到各种各样的角色，面对不同的挑战，最终走向属于你的结局。"),
            ],
        };

        let idx = rand::thread_rng().gen_range(0..outlines.len());
        let (title, desc) = outlines[idx];
        format!("游戏名称：{}\n类型：{}\n\n{}\n\n第一章：启程\n- 场景1：故事开始\n- 场景2：初遇挑战\n- 关键选择：前进的方向\n\n第二章：深入\n- 场景1：真相浮现\n- 场景2：危机四伏\n- 关键选择：信任与背叛\n\n第三章：终局\n- 场景1：最终对决\n- 场景2：命运抉择\n\n结局：\n- 结局A：光明\n- 结局B：黑暗", title, game_type, desc)
    }

    /// 完整的游戏创建流程 — 渐进式加载：第一章优先生成，后续章节后台生成
    pub async fn create_game(
        &self,
        input: &str,
        game_type: Option<GameType>,
    ) -> Result<(String, GameScript), ProviderError> {
        // 1. 生成 game_id
        let game_id = Uuid::new_v4().to_string();

        // 2. 创建游戏目录
        self.asset_manager
            .ensure_game_dirs(&game_id)
            .map_err(ProviderError::GenerationFailed)?;

        // 3. 调用 OutlineParser 解析大纲为 GameScript
        let mut game_script = self.parse_outline(input, game_type.clone()).await?;

        // 4. 提取所有 AssetRef
        let mut asset_refs = self.extract_asset_refs(&game_script);

        // 5. 为每个 AssetRef 确定来源
        self.resolve_sources(&mut asset_refs).await?;

        // 6. 将来源写回 GameScript
        Self::apply_sources_to_script(&mut game_script, &asset_refs);

        // 7. 保存 GameScript 到 script.json
        self.asset_manager
            .save_game_script(&game_id, &game_script)
            .map_err(ProviderError::GenerationFailed)?;

        // 8. 存储 GameScript 供后续 regenerate_asset 使用
        self.scripts.write().await.insert(game_id.clone(), game_script.clone());

        // 9. 按优先级排序：场景背景图 > NPC头像 > BGM > 语音 > CG视频
        asset_refs.sort_by(|a, b| {
            Self::asset_priority(&a.1.asset_type).cmp(&Self::asset_priority(&b.1.asset_type))
        });

        // 10. 分离第一章和后续章节的 AssetRef
        let first_chapter_id = game_script.chapters.first().map(|c| c.id.clone());
        let (first_chapter_refs, remaining_refs): (Vec<_>, Vec<_>) = asset_refs
            .into_iter()
            .partition(|(cid, _)| first_chapter_id.as_ref() == Some(cid));

        // 11. 初始化生成状态
        let total = first_chapter_refs.len() + remaining_refs.len();
        let mut chapter_map: HashMap<String, ChapterStatus> = HashMap::new();
        for chapter in &game_script.chapters {
            let is_first = first_chapter_id.as_ref() == Some(&chapter.id);
            let chapter_asset_count = if is_first {
                first_chapter_refs.iter().filter(|(cid, _)| cid == &chapter.id).count()
            } else {
                remaining_refs.iter().filter(|(cid, _)| cid == &chapter.id).count()
            };
            chapter_map.insert(
                chapter.id.clone(),
                ChapterStatus {
                    chapter_id: chapter.id.clone(),
                    chapter_title: chapter.title.clone(),
                    total_assets: chapter_asset_count,
                    completed_assets: 0,
                    status: if is_first { "generating".to_string() } else { "pending".to_string() },
                },
            );
        }
        self.statuses.write().await.insert(
            game_id.clone(),
            GenerationStatus {
                game_id: game_id.clone(),
                total_assets: total,
                completed_assets: 0,
                failed_assets: 0,
                chapter_status: chapter_map,
                overall_progress: 0.0,
                first_chapter_ready: false,
                background_generation_active: false,
            },
        );

        // 12. 优先生成第一章资源
        let first_results = self.fetch_assets(&game_id, &first_chapter_refs).await;

        // 13. 处理第一章结果
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut chapter_completed: HashMap<String, usize> = HashMap::new();
        let mut chapter_failed: HashMap<String, usize> = HashMap::new();

        for (i, result) in first_results.into_iter().enumerate() {
            let (chapter_id, asset_ref) = &first_chapter_refs[i];
            match result {
                Ok(ref local_asset) => {
                    self.on_asset_ready(&game_id, asset_ref, local_asset);
                    completed += 1;
                    *chapter_completed.entry(chapter_id.clone()).or_insert(0) += 1;
                }
                Err(ref error) => {
                    self.on_asset_failed(&game_id, asset_ref, error);
                    failed += 1;
                    *chapter_failed.entry(chapter_id.clone()).or_insert(0) += 1;
                }
            }
        }

        // 14. 更新第一章状态
        {
            let mut statuses = self.statuses.write().await;
            if let Some(status) = statuses.get_mut(&game_id) {
                status.completed_assets = completed;
                status.failed_assets = failed;
                status.overall_progress = if total > 0 {
                    (completed + failed) as f32 / total as f32
                } else {
                    1.0
                };
                for (cid, cs) in status.chapter_status.iter_mut() {
                    if first_chapter_id.as_ref() == Some(cid) {
                        let chap_completed = *chapter_completed.get(cid).unwrap_or(&0);
                        let chap_failed = *chapter_failed.get(cid).unwrap_or(&0);
                        cs.completed_assets = chap_completed;
                        cs.status = if chap_completed + chap_failed >= cs.total_assets {
                            if chap_failed > 0 { "partial".to_string() } else { "ready".to_string() }
                        } else if chap_completed > 0 {
                            "partial".to_string()
                        } else {
                            "generating".to_string()
                        };
                    }
                }
                status.first_chapter_ready = status.chapter_status
                    .get(first_chapter_id.as_deref().unwrap_or(""))
                    .map(|cs| cs.status == "ready" || cs.status == "partial")
                    .unwrap_or(false);
            }
        }

        // 15. 第一章就绪后立即发送 generation-complete 事件
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(
                "generation-complete",
                serde_json::json!({ "gameId": game_id, "chapterId": first_chapter_id }),
            );
        }

        // 16. 启动后台生成后续章节
        if !remaining_refs.is_empty() {
            self.start_background_generation(game_id.clone(), remaining_refs, first_chapter_id.clone());
        }

        Ok((game_id, game_script))
    }

    /// 后台预生成后续章节
    pub fn start_background_generation(
        &self,
        game_id: String,
        remaining_refs: Vec<(String, AssetRef)>,
        first_chapter_id: Option<String>,
    ) {
        let config_manager = self.config_manager.clone();
        let asset_manager = self.asset_manager.clone();
        let app_handle = self.app_handle.clone();
        let statuses = self.statuses.clone();

        tokio::spawn(async move {
            // 标记后台生成已激活
            {
                let mut s = statuses.write().await;
                if let Some(status) = s.get_mut(&game_id) {
                    status.background_generation_active = true;
                }
            }

            // 逐章节生成
            let mut by_chapter: HashMap<String, Vec<(String, AssetRef)>> = HashMap::new();
            for (cid, aref) in remaining_refs {
                by_chapter.entry(cid.clone()).or_default().push((cid, aref));
            }

            // 按章节顺序排列
            let mut chapter_ids: Vec<String> = by_chapter.keys().cloned().collect();
            // 将 first_chapter_id 排除（已生成）
            chapter_ids.retain(|id| first_chapter_id.as_ref() != Some(id));

            for chapter_id in chapter_ids {
                let refs = match by_chapter.get(&chapter_id) {
                    Some(r) => r.clone(),
                    None => continue,
                };

                // 更新章节状态为 generating
                {
                    let mut s = statuses.write().await;
                    if let Some(status) = s.get_mut(&game_id) {
                        if let Some(cs) = status.chapter_status.get_mut(&chapter_id) {
                            cs.status = "generating".to_string();
                        }
                    }
                }

                // 发送章节开始生成事件
                if let Some(ref handle) = app_handle {
                    let _ = handle.emit(
                        "generation-progress",
                        serde_json::json!({ "gameId": game_id, "chapterId": chapter_id }),
                    );
                }

                let results = Self::fetch_assets_static(
                    &config_manager,
                    &asset_manager,
                    &game_id,
                    &refs,
                ).await;

                // 处理结果
                let mut chap_completed = 0usize;
                let mut chap_failed = 0usize;

                for (i, result) in results.into_iter().enumerate() {
                    let (_, asset_ref) = &refs[i];
                    match result {
                        Ok(ref local_asset) => {
                            // 发送 asset-ready 事件
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "asset-ready",
                                    serde_json::json!({
                                        "gameId": game_id,
                                        "assetRefId": asset_ref.id,
                                        "assetType": format!("{:?}", asset_ref.asset_type),
                                        "localPath": local_asset.local_path,
                                        "source": format!("{:?}", local_asset.source),
                                        "chapterId": chapter_id,
                                    }),
                                );
                            }
                            chap_completed += 1;
                        }
                        Err(ref error) => {
                            // 发送 asset-failed 事件
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "asset-failed",
                                    serde_json::json!({
                                        "gameId": game_id,
                                        "assetRefId": asset_ref.id,
                                        "error": format!("{:?}", error),
                                        "fallbackToBuiltin": true,
                                        "chapterId": chapter_id,
                                    }),
                                );
                            }
                            chap_failed += 1;
                        }
                    }
                }

                // 更新状态
                {
                    let mut s = statuses.write().await;
                    if let Some(status) = s.get_mut(&game_id) {
                        status.completed_assets += chap_completed;
                        status.failed_assets += chap_failed;
                        status.overall_progress = if status.total_assets > 0 {
                            (status.completed_assets + status.failed_assets) as f32 / status.total_assets as f32
                        } else {
                            1.0
                        };
                        if let Some(cs) = status.chapter_status.get_mut(&chapter_id) {
                            cs.completed_assets = chap_completed;
                            cs.status = if chap_completed + chap_failed >= cs.total_assets {
                                if chap_failed > 0 { "partial".to_string() } else { "ready".to_string() }
                            } else if chap_completed > 0 {
                                "partial".to_string()
                            } else {
                                "generating".to_string()
                            };
                        }

                        // 检查是否所有章节都完成了
                        let all_done = status.chapter_status.values().all(|cs| {
                            cs.status == "ready" || cs.status == "partial"
                        });
                        if all_done {
                            status.background_generation_active = false;
                        }
                    }
                }

                // 每个章节完成后发送 generation-complete 事件
                if let Some(ref handle) = app_handle {
                    let _ = handle.emit(
                        "generation-complete",
                        serde_json::json!({ "gameId": game_id, "chapterId": chapter_id }),
                    );
                }
            }

            // 全部完成后发送最终事件
            if let Some(ref handle) = app_handle {
                let _ = handle.emit(
                    "generation-complete",
                    serde_json::json!({ "gameId": game_id, "allChapters": true }),
                );
            }
        });
    }

    /// 重新生成指定资源 — 强制使用 AI Provider
    pub async fn regenerate_asset(
        &self,
        game_id: &str,
        asset_ref_id: &str,
    ) -> Result<(), ProviderError> {
        // 从存储的 GameScript 中查找 AssetRef
        let scripts = self.scripts.read().await;
        let script = scripts.get(game_id).ok_or_else(|| {
            ProviderError::NotFound(format!("Game script not found for game '{}'", game_id))
        })?;

        let asset_ref = Self::find_asset_ref_by_id(script, asset_ref_id).ok_or_else(|| {
            ProviderError::NotFound(format!("AssetRef '{}' not found", asset_ref_id))
        })?;

        let mut asset_ref = asset_ref.clone();
        // 强制使用 AI 生成
        asset_ref.source = ScriptAssetSource::AiGenerated;
        asset_ref.status = AssetStatus::Pending;

        drop(scripts);

        // 获取 AI Provider 并重新生成
        let modality = Self::asset_type_to_modality(&asset_ref.asset_type);
        let provider = self.resolve_ai_provider(modality).await?;

        match provider.get_asset(&asset_ref).await {
            Ok(local_asset) => {
                self.on_asset_ready(game_id, &asset_ref, &local_asset);
                Ok(())
            }
            Err(e) => {
                self.on_asset_failed(game_id, &asset_ref, &e);
                Err(e)
            }
        }
    }

    /// 多候选重新生成 — 对图片和视频资源一次生成多个候选供用户选择
    /// 文本类型资源不提供多候选，返回空 Vec
    pub async fn regenerate_asset_with_candidates(
        &self,
        game_id: &str,
        asset_ref_id: &str,
        count: u32,
    ) -> Result<Vec<LocalAsset>, ProviderError> {
        // 从存储的 GameScript 中查找 AssetRef
        let scripts = self.scripts.read().await;
        let script = scripts.get(game_id).ok_or_else(|| {
            ProviderError::NotFound(format!("Game script not found for game '{}'", game_id))
        })?;

        let asset_ref = Self::find_asset_ref_by_id(script, asset_ref_id).ok_or_else(|| {
            ProviderError::NotFound(format!("AssetRef '{}' not found", asset_ref_id))
        })?;

        let mut asset_ref = asset_ref.clone();
        // 强制使用 AI 生成
        asset_ref.source = ScriptAssetSource::AiGenerated;
        asset_ref.status = AssetStatus::Pending;

        drop(scripts);

        // 文本类型资源不支持多候选
        if !Self::is_visual_asset_type(&asset_ref.asset_type) {
            return Ok(Vec::new());
        }

        let modality = Self::asset_type_to_modality(&asset_ref.asset_type);
        let provider = self.resolve_ai_provider(modality).await?;

        let actual_count = count.max(2).min(4) as usize;
        let mut candidates = Vec::with_capacity(actual_count);
        let mut errors = Vec::new();

        for _ in 0..actual_count {
            match provider.get_asset(&asset_ref).await {
                Ok(local_asset) => {
                    candidates.push(local_asset);
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }

        if candidates.is_empty() {
            // 所有候选都失败了
            let first_error = errors.into_iter().next().unwrap_or_else(|| {
                ProviderError::GenerationFailed("All candidate generations failed".to_string())
            });
            self.on_asset_failed(game_id, &asset_ref, &first_error);
            Err(first_error)
        } else {
            // 至少有一个候选成功，通知第一个就绪
            self.on_asset_ready(game_id, &asset_ref, &candidates[0]);
            Ok(candidates)
        }
    }

    /// 判断资源类型是否为可视类型（图片或视频），只有可视类型支持多候选
    fn is_visual_asset_type(asset_type: &ScriptAssetType) -> bool {
        matches!(asset_type, ScriptAssetType::Image | ScriptAssetType::Video)
    }

    /// 在 GameScript 中查找指定 ID 的 AssetRef
    fn find_asset_ref_by_id<'a>(script: &'a GameScript, asset_ref_id: &str) -> Option<&'a AssetRef> {
        for chapter in &script.chapters {
            for scene in &chapter.scenes {
                if let Some(ref bg) = scene.assets.background_image {
                    if bg.id == asset_ref_id { return Some(bg); }
                }
                if let Some(ref video) = scene.assets.background_video {
                    if video.id == asset_ref_id { return Some(video); }
                }
                if let Some(ref bgm) = scene.assets.bgm {
                    if bgm.id == asset_ref_id { return Some(bgm); }
                }
                if let Some(ref ambient) = scene.assets.ambient_sound {
                    if ambient.id == asset_ref_id { return Some(ambient); }
                }
                if let Some(ref cg) = scene.assets.cg_animation {
                    if cg.id == asset_ref_id { return Some(cg); }
                }
                for node in &scene.sequence {
                    match node {
                        SceneNode::Dialogue(d) => {
                            if let Some(ref avatar) = d.speaker_avatar {
                                if avatar.id == asset_ref_id { return Some(avatar); }
                            }
                            if let Some(ref voice) = d.voice_asset {
                                if voice.id == asset_ref_id { return Some(voice); }
                            }
                        }
                        SceneNode::Narration(n) => {
                            if let Some(ref voice) = n.voice_asset {
                                if voice.id == asset_ref_id { return Some(voice); }
                            }
                        }
                        SceneNode::Cg(c) => {
                            if c.video_asset.id == asset_ref_id { return Some(&c.video_asset); }
                        }
                        _ => {}
                    }
                }
            }
        }
        None
    }

    /// 强制获取 AI Provider（用于重新生成）
    async fn resolve_ai_provider(
        &self,
        modality: AIModality,
    ) -> Result<Box<dyn IAssetProvider>, ProviderError> {
        let config_manager = self.config_manager.read().await;
        let config = config_manager.get_config();

        let provider_config = config.providers.iter().find(|p| {
            p.modality.contains(&modality) && p.status == ProviderStatus::Connected
        });

        if let Some(pc) = provider_config {
            ProviderFactory::create(pc, self.asset_manager.base_path())
        } else {
            Err(ProviderError::InvalidConfig(format!(
                "No AI provider configured for modality {:?}",
                modality
            )))
        }
    }

    /// 静态版本的 fetch_assets，用于后台任务
    async fn fetch_assets_static(
        config_manager: &Arc<RwLock<ConfigManager>>,
        asset_manager: &Arc<AssetManager>,
        game_id: &str,
        asset_refs: &[(String, AssetRef)],
    ) -> Vec<Result<LocalAsset, ProviderError>> {
        let mut handles: Vec<tokio::task::JoinHandle<Result<LocalAsset, ProviderError>>> =
            Vec::with_capacity(asset_refs.len());

        for (_chapter_id, asset_ref) in asset_refs {
            let asset_ref = asset_ref.clone();
            let config_manager = config_manager.clone();
            let asset_base_path = asset_manager.base_path().to_path_buf();

            let handle = tokio::spawn(async move {
                let modality = Self::asset_type_to_modality(&asset_ref.asset_type);

                let provider = {
                    let config_mgr = config_manager.read().await;
                    let config = config_mgr.get_config();
                    let pc = config.providers.iter().find(|p| {
                        p.modality.contains(&modality) && p.status == ProviderStatus::Connected
                    });
                    if let Some(pc) = pc {
                        ProviderFactory::create(pc, &asset_base_path)?
                    } else {
                        let builtin_path = asset_base_path.join("builtin-assets");
                        let games_path = asset_base_path.join("games");
                        Box::new(BuiltinAssetProvider::new(builtin_path, games_path))
                            as Box<dyn IAssetProvider>
                    }
                };

                let result = provider.get_asset(&asset_ref).await;

                match result {
                    Ok(asset) => Ok(asset),
                    Err(original_error) => {
                        let retry_result = provider.get_asset(&asset_ref).await;
                        match retry_result {
                            Ok(asset) => Ok(asset),
                            Err(_) => {
                                let builtin_path = asset_base_path.join("builtin-assets");
                                let games_path = asset_base_path.join("games");
                                let builtin = BuiltinAssetProvider::new(builtin_path, games_path);
                                match builtin.get_asset(&asset_ref).await {
                                    Ok(asset) => Ok(asset),
                                    Err(_) => Err(original_error),
                                }
                            }
                        }
                    }
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(ProviderError::GenerationFailed(format!(
                    "Task error: {}",
                    e
                )))),
            }
        }
        results
    }

    /// 解析大纲为 GameScript
    async fn parse_outline(
        &self,
        input: &str,
        game_type: Option<GameType>,
    ) -> Result<GameScript, ProviderError> {
        let config_manager = self.config_manager.read().await;
        let config = config_manager.get_config();

        // 找到 DeepSeek Provider 配置
        let ds_config = config
            .providers
            .iter()
            .find(|p| p.vendor == "deepseek" && p.status == ProviderStatus::Connected)
            .or_else(|| config.providers.iter().find(|p| p.vendor == "deepseek"))
            .ok_or_else(|| {
                ProviderError::InvalidConfig("No DeepSeek provider configured".to_string())
            })?;

        let deepseek = DeepSeekProvider::new(ds_config, self.asset_manager.base_path())?;

        let prompts_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("autofree")
            .join("prompts");

        let parser = OutlineParser::new(deepseek, prompts_dir);
        parser.parse(input, game_type).await
    }

    /// 从 GameScript 提取所有 AssetRef
    fn extract_asset_refs(&self, script: &GameScript) -> Vec<(String, AssetRef)> {
        let mut refs = Vec::new();
        for chapter in &script.chapters {
            for scene in &chapter.scenes {
                if let Some(ref bg) = scene.assets.background_image {
                    refs.push((chapter.id.clone(), bg.clone()));
                }
                if let Some(ref video) = scene.assets.background_video {
                    refs.push((chapter.id.clone(), video.clone()));
                }
                if let Some(ref bgm) = scene.assets.bgm {
                    refs.push((chapter.id.clone(), bgm.clone()));
                }
                if let Some(ref ambient) = scene.assets.ambient_sound {
                    refs.push((chapter.id.clone(), ambient.clone()));
                }
                if let Some(ref cg) = scene.assets.cg_animation {
                    refs.push((chapter.id.clone(), cg.clone()));
                }
                for node in &scene.sequence {
                    match node {
                        SceneNode::Dialogue(d) => {
                            if let Some(ref avatar) = d.speaker_avatar {
                                refs.push((chapter.id.clone(), avatar.clone()));
                            }
                            if let Some(ref voice) = d.voice_asset {
                                refs.push((chapter.id.clone(), voice.clone()));
                            }
                        }
                        SceneNode::Narration(n) => {
                            if let Some(ref voice) = n.voice_asset {
                                refs.push((chapter.id.clone(), voice.clone()));
                            }
                        }
                        SceneNode::Cg(c) => {
                            refs.push((chapter.id.clone(), c.video_asset.clone()));
                        }
                        _ => {}
                    }
                }
            }
        }
        refs
    }

    /// 为每个 AssetRef 确定来源（AI 已配置 → ai_generated，否则 → builtin）
    async fn resolve_sources(
        &self,
        asset_refs: &mut [(String, AssetRef)],
    ) -> Result<(), ProviderError> {
        let config_manager = self.config_manager.read().await;
        let config = config_manager.get_config();

        for (_, asset_ref) in asset_refs.iter_mut() {
            let modality = Self::asset_type_to_modality(&asset_ref.asset_type);
            let has_ai_provider = config
                .providers
                .iter()
                .any(|p| p.modality.contains(&modality) && p.status == ProviderStatus::Connected);

            if has_ai_provider {
                asset_ref.source = ScriptAssetSource::AiGenerated;
                asset_ref.status = AssetStatus::Pending;
            } else {
                asset_ref.source = ScriptAssetSource::Builtin;
                asset_ref.status = AssetStatus::Fallback;
            }
        }
        Ok(())
    }

    /// 根据模态选择 Provider
    async fn resolve_provider(
        &self,
        modality: AIModality,
    ) -> Result<Box<dyn IAssetProvider>, ProviderError> {
        let config_manager = self.config_manager.read().await;
        let config = config_manager.get_config();

        let provider_config = config.providers.iter().find(|p| {
            p.modality.contains(&modality) && p.status == ProviderStatus::Connected
        });

        if let Some(pc) = provider_config {
            ProviderFactory::create(pc, self.asset_manager.base_path())
        } else {
            self.create_builtin_provider()
        }
    }

    /// 创建内置 Provider
    fn create_builtin_provider(&self) -> Result<Box<dyn IAssetProvider>, ProviderError> {
        let builtin_assets_path = self.asset_manager.base_path().join("builtin-assets");
        let game_assets_path = self.asset_manager.base_path().join("games");
        Ok(Box::new(BuiltinAssetProvider::new(
            builtin_assets_path,
            game_assets_path,
        )))
    }

    /// 并行获取资源
    async fn fetch_assets(
        &self,
        game_id: &str,
        asset_refs: &[(String, AssetRef)],
    ) -> Vec<Result<LocalAsset, ProviderError>> {
        let mut handles: Vec<tokio::task::JoinHandle<Result<LocalAsset, ProviderError>>> =
            Vec::with_capacity(asset_refs.len());

        for (_chapter_id, asset_ref) in asset_refs {
            let asset_ref = asset_ref.clone();
            let config_manager = self.config_manager.clone();
            let asset_base_path = self.asset_manager.base_path().to_path_buf();
            let game_id = game_id.to_string();

            let handle = tokio::spawn(async move {
                let modality = Self::asset_type_to_modality(&asset_ref.asset_type);

                // 创建 Provider
                let provider = {
                    let config_mgr = config_manager.read().await;
                    let config = config_mgr.get_config();
                    let pc = config.providers.iter().find(|p| {
                        p.modality.contains(&modality) && p.status == ProviderStatus::Connected
                    });
                    if let Some(pc) = pc {
                        ProviderFactory::create(pc, &asset_base_path)?
                    } else {
                        let builtin_path = asset_base_path.join("builtin-assets");
                        let games_path = asset_base_path.join("games");
                        Box::new(BuiltinAssetProvider::new(builtin_path, games_path))
                            as Box<dyn IAssetProvider>
                    }
                };

                // 尝试获取资源
                let result = provider.get_asset(&asset_ref).await;

                // 失败时重试 1 次，仍失败则降级到 BuiltinAssetProvider
                match result {
                    Ok(asset) => Ok(asset),
                    Err(original_error) => {
                        // 重试 1 次
                        let retry_result = provider.get_asset(&asset_ref).await;
                        match retry_result {
                            Ok(asset) => Ok(asset),
                            Err(_) => {
                                // 降级到 BuiltinAssetProvider
                                let builtin_path = asset_base_path.join("builtin-assets");
                                let games_path = asset_base_path.join("games");
                                let builtin = BuiltinAssetProvider::new(builtin_path, games_path);
                                match builtin.get_asset(&asset_ref).await {
                                    Ok(asset) => Ok(asset),
                                    Err(_) => Err(original_error),
                                }
                            }
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // 收集结果（保持顺序）
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(ProviderError::GenerationFailed(format!(
                    "Task error: {}",
                    e
                )))),
            }
        }
        results
    }

    /// 资源就绪回调 → Tauri 事件通知前端
    fn on_asset_ready(&self, game_id: &str, asset_ref: &AssetRef, local_asset: &LocalAsset) {
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(
                "asset-ready",
                serde_json::json!({
                    "gameId": game_id,
                    "assetRefId": asset_ref.id,
                    "assetType": format!("{:?}", asset_ref.asset_type),
                    "localPath": local_asset.local_path,
                    "source": format!("{:?}", local_asset.source),
                }),
            );
        }
    }

    /// 资源失败回调
    fn on_asset_failed(&self, game_id: &str, asset_ref: &AssetRef, error: &ProviderError) {
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(
                "asset-failed",
                serde_json::json!({
                    "gameId": game_id,
                    "assetRefId": asset_ref.id,
                    "error": format!("{:?}", error),
                    "fallbackToBuiltin": true,
                }),
            );
        }
    }

    /// 获取生成状态
    pub async fn get_status(&self, game_id: &str) -> Option<GenerationStatus> {
        let statuses = self.statuses.read().await;
        statuses.get(game_id).cloned()
    }

    // --- Helper methods ---

    /// AssetType → AIModality 映射
    fn asset_type_to_modality(asset_type: &ScriptAssetType) -> AIModality {
        match asset_type {
            ScriptAssetType::Image => AIModality::Image,
            ScriptAssetType::Video => AIModality::Video,
            ScriptAssetType::Audio => AIModality::Music,
            ScriptAssetType::Voice => AIModality::Voice,
        }
    }

    /// 资源优先级（数值越小优先级越高）
    /// 场景背景图 > NPC头像 > BGM > 语音 > CG视频
    fn asset_priority(asset_type: &ScriptAssetType) -> u32 {
        match asset_type {
            ScriptAssetType::Image => 0,
            ScriptAssetType::Audio => 2,
            ScriptAssetType::Voice => 3,
            ScriptAssetType::Video => 4,
        }
    }

    /// 将解析后的来源写回 GameScript
    fn apply_sources_to_script(script: &mut GameScript, resolved_refs: &[(String, AssetRef)]) {
        let source_map: HashMap<&str, &AssetRef> = resolved_refs
            .iter()
            .map(|(_, ar)| (ar.id.as_str(), ar))
            .collect();

        Self::update_script_assets(script, |asset_ref: &mut AssetRef| {
            if let Some(resolved) = source_map.get(asset_ref.id.as_str()) {
                asset_ref.source = resolved.source.clone();
                asset_ref.status = resolved.status.clone();
            }
        });
    }

    /// 遍历 GameScript 中所有 AssetRef 并应用修改函数
    fn update_script_assets<F>(script: &mut GameScript, mut f: F)
    where
        F: FnMut(&mut AssetRef),
    {
        for chapter in &mut script.chapters {
            for scene in &mut chapter.scenes {
                if let Some(ref mut bg) = scene.assets.background_image {
                    f(bg);
                }
                if let Some(ref mut video) = scene.assets.background_video {
                    f(video);
                }
                if let Some(ref mut bgm) = scene.assets.bgm {
                    f(bgm);
                }
                if let Some(ref mut ambient) = scene.assets.ambient_sound {
                    f(ambient);
                }
                if let Some(ref mut cg) = scene.assets.cg_animation {
                    f(cg);
                }
                for node in &mut scene.sequence {
                    match node {
                        SceneNode::Dialogue(d) => {
                            if let Some(ref mut avatar) = d.speaker_avatar {
                                f(avatar);
                            }
                            if let Some(ref mut voice) = d.voice_asset {
                                f(voice);
                            }
                        }
                        SceneNode::Narration(n) => {
                            if let Some(ref mut voice) = n.voice_asset {
                                f(voice);
                            }
                        }
                        SceneNode::Cg(c) => {
                            f(&mut c.video_asset);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
