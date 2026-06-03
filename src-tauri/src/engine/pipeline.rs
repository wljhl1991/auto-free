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
    statuses: RwLock<HashMap<String, GenerationStatus>>,
}

impl GenerationPipeline {
    pub fn new(config_manager: Arc<RwLock<ConfigManager>>, asset_manager: Arc<AssetManager>) -> Self {
        Self {
            config_manager,
            asset_manager,
            app_handle: None,
            statuses: RwLock::new(HashMap::new()),
        }
    }

    pub fn set_app_handle(&mut self, handle: AppHandle) {
        self.app_handle = Some(handle);
    }

    /// 完整的游戏创建流程
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

        // 8. 按优先级排序：场景背景图 > NPC头像 > BGM > 语音 > CG视频
        asset_refs.sort_by(|a, b| {
            Self::asset_priority(&a.1.asset_type).cmp(&Self::asset_priority(&b.1.asset_type))
        });

        // 9. 初始化生成状态
        let total = asset_refs.len();
        let mut chapter_map: HashMap<String, ChapterStatus> = HashMap::new();
        for chapter in &game_script.chapters {
            let chapter_asset_count = asset_refs
                .iter()
                .filter(|(cid, _)| cid == &chapter.id)
                .count();
            chapter_map.insert(
                chapter.id.clone(),
                ChapterStatus {
                    chapter_id: chapter.id.clone(),
                    chapter_title: chapter.title.clone(),
                    total_assets: chapter_asset_count,
                    completed_assets: 0,
                    status: "generating".to_string(),
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
            },
        );

        // 10. 并行获取资源
        let results = self.fetch_assets(&game_id, &asset_refs).await;

        // 11. 处理结果
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut chapter_completed: HashMap<String, usize> = HashMap::new();
        let mut chapter_failed: HashMap<String, usize> = HashMap::new();

        for (i, result) in results.into_iter().enumerate() {
            let (chapter_id, asset_ref) = &asset_refs[i];
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

        // 12. 更新最终状态
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
                cs.completed_assets = *chapter_completed.get(cid).unwrap_or(&0);
                let chap_failed = *chapter_failed.get(cid).unwrap_or(&0);
                cs.status = if cs.completed_assets + chap_failed >= cs.total_assets {
                    if chap_failed > 0 {
                        "partial".to_string()
                    } else {
                        "ready".to_string()
                    }
                } else if cs.completed_assets > 0 {
                    "partial".to_string()
                } else {
                    "generating".to_string()
                };
            }
        }

        // 13. 发送 generation-complete 事件
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(
                "generation-complete",
                serde_json::json!({ "gameId": game_id }),
            );
        }

        Ok((game_id, game_script))
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
