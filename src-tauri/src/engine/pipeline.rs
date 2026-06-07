use crate::config::manager::ConfigManager;
use crate::engine::outline_parser::OutlineParser;
use crate::engine::asset_manager::AssetManager;
use crate::engine::call_history::{CallHistory, build_record};
use crate::providers::{IAssetProvider, ProviderFactory, ProviderError};
use crate::providers::builtin::BuiltinAssetProvider;
use crate::providers::deepseek::DeepSeekProvider;
use crate::types::game_script::{
    GameScript, GameType, AssetRef, AssetType as ScriptAssetType,
    AssetSource as ScriptAssetSource, AssetStatus, SceneNode, SceneAssets,
};
use crate::types::asset::{LocalAsset, AIModality};
use crate::types::ai_provider::ProviderStatus;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tauri::AppHandle;
use tauri::Emitter;
use uuid::Uuid;

// 内嵌 Prompt 模板，供高质量模式使用
const PROMPT_COMBINED: &str = include_str!("../../../prompts/outline-parser/combined.md");
const PROMPT_CORE_ELEMENTS: &str = include_str!("../../../prompts/outline-parser/core_elements.md");

/// 生成上下文：保存高质量模式中阶段1和阶段2的信息，供后续章节后台生成使用
#[derive(Clone)]
#[allow(dead_code)]
struct GenerationContext {
    core_elements: String,
    chapter_details: Vec<String>,
    total_chapters: usize,
    game_type: Option<GameType>,
    input: String,
    messages: Vec<crate::providers::deepseek::ChatMessage>,
    /// 后台生成是否被取消
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

/// 进度步骤记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProgressStep {
    pub step: String,
    pub detail: String,
    pub model_name: String,
    pub timestamp: u64,
}

/// 生成状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerationStatus {
    pub game_id: String,
    pub game_title: Option<String>,
    pub total_assets: usize,
    pub completed_assets: usize,
    pub failed_assets: usize,
    pub chapter_status: HashMap<String, ChapterStatus>,
    pub overall_progress: f32,
    pub first_chapter_ready: bool,
    pub background_generation_active: bool,
    pub progress_steps: Vec<ProgressStep>,
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
    scripts: Arc<RwLock<HashMap<String, GameScript>>>,
    /// AI 调用历史记录器
    call_history: Arc<CallHistory>,
    /// 高质量模式的生成上下文，供后续章节后台生成使用
    generation_contexts: Arc<RwLock<HashMap<String, GenerationContext>>>,
}

impl GenerationPipeline {
    pub fn new(config_manager: Arc<RwLock<ConfigManager>>, asset_manager: Arc<AssetManager>) -> Self {
        let base_path = asset_manager.base_path().to_path_buf();
        let call_history = Arc::new(CallHistory::new(&base_path));
        Self {
            config_manager,
            asset_manager,
            app_handle: None,
            statuses: Arc::new(RwLock::new(HashMap::new())),
            scripts: Arc::new(RwLock::new(HashMap::new())),
            call_history,
            generation_contexts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 根据模态查找首选 provider 配置
    /// 优先级：1. 用户在 preferred_providers 中指定的 provider
    ///         2. deepseek（仅文本模态的硬编码优先级）
    ///         3. 任意 Connected 的同模态 provider
    ///         4. 任意同模态 provider（不论状态）
    fn find_provider_for_modality<'a>(
        config: &'a crate::types::ai_provider::AppConfig,
        modality: &AIModality,
    ) -> Option<&'a crate::types::ai_provider::AIProviderConfig> {
        let modality_str = match modality {
            AIModality::Text => "text",
            AIModality::Image => "image",
            AIModality::Video => "video",
            AIModality::Music => "music",
            AIModality::Voice => "voice",
        };

        // 1. 用户指定的首选 provider
        if let Some(preferred_id) = config.global_settings.preferred_providers.get(modality_str) {
            if let Some(pc) = config.providers.iter()
                .find(|p| p.id == *preferred_id && p.modality.contains(modality) && p.status == ProviderStatus::Connected)
                .or_else(|| config.providers.iter().find(|p| p.id == *preferred_id && p.modality.contains(modality)))
            {
                log::info!("使用用户首选 provider: id={}, modality={}", pc.id, modality_str);
                return Some(pc);
            }
        }

        // 2. 文本模态：deepseek 硬编码优先级（向后兼容）
        if matches!(modality, AIModality::Text) {
            if let Some(pc) = config.providers.iter()
                .find(|p| p.id == "deepseek" && p.status == ProviderStatus::Connected)
                .or_else(|| config.providers.iter().find(|p| p.id == "deepseek"))
            {
                return Some(pc);
            }
        }

        // 3. 任意 Connected 的同模态 provider
        if let Some(pc) = config.providers.iter()
            .find(|p| p.modality.contains(modality) && p.status == ProviderStatus::Connected)
        {
            return Some(pc);
        }

        // 4. 任意同模态 provider
        config.providers.iter().find(|p| p.modality.contains(modality))
    }

    pub fn set_app_handle(&mut self, handle: AppHandle) {
        self.app_handle = Some(handle);
    }

    /// 发送生成进度事件到前端
    fn emit_progress(&self, game_id: &str, step: &str, detail: &str, model_name: &str) {
        if let Some(ref handle) = self.app_handle {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = handle.emit(
                "generation-step",
                serde_json::json!({
                    "gameId": game_id,
                    "step": step,
                    "detail": detail,
                    "modelName": model_name,
                    "timestamp": timestamp,
                }),
            );
        }
    }

    /// 获取指定模态的 AI 模型显示名称
    #[allow(dead_code)]
    async fn get_model_display_name_for_modality(&self, modality: &AIModality) -> String {
        let config_manager = self.config_manager.read().await;
        let config = config_manager.get_config();
        if let Some(pc) = Self::find_provider_for_modality(config, modality) {
            let model = pc.models.iter()
                .find(|m| m.is_default)
                .map(|m| m.id.clone())
                .unwrap_or_default();
            format!("{}/{}", pc.id, model)
        } else {
            "内置资源".to_string()
        }
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

        // 使用统一的 provider 查找逻辑（优先用户首选 > deepseek > 任意 Connected）
        let text_config = Self::find_provider_for_modality(config, &AIModality::Text).cloned();

        drop(config_manager);

        if let Some(provider_config) = text_config {
            // 所有文本 provider 都使用兼容 OpenAI API 的 chat 接口
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
            Ok(OutlineParser::strip_think_tags(&outline))
        } else {
            // 文本 AI 未配置，使用预设的示例大纲
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

    /// 异步版本的游戏创建流程 — 只做初始化工作，立即返回 game_id，生成流程在后台进行
    /// 前端拿到 game_id 后跳转到进度页，通过事件监听实时显示进度
    pub async fn create_game_async(
        &self,
        input: &str,
        game_type: Option<GameType>,
        use_local_fallback: bool,
        high_quality: bool,
        chapter_count: Option<u32>,
    ) -> Result<String, ProviderError> {
        log::info!("异步创建游戏: input_len={}, game_type={:?}, high_quality={}, chapter_count={:?}", input.len(), game_type, high_quality, chapter_count);

        // 1. 生成 game_id
        let game_id = Uuid::new_v4().to_string();

        // 2. 创建游戏目录
        self.asset_manager
            .ensure_game_dirs(&game_id)
            .map_err(ProviderError::GenerationFailed)?;

        // 3. 初始化生成状态（初始为空，后台 task 会更新）
        self.statuses.write().await.insert(
            game_id.clone(),
            GenerationStatus {
                game_id: game_id.clone(),
                game_title: None,
                total_assets: 0,
                completed_assets: 0,
                failed_assets: 0,
                chapter_status: HashMap::new(),
                overall_progress: 0.0,
                first_chapter_ready: false,
                background_generation_active: false,
                progress_steps: Vec::new(),
            },
        );

        // 4. 发送初始进度事件
        self.emit_progress(&game_id, "starting", "正在初始化生成任务", "");

        // 5. Clone 所有需要的 Arc 引用，spawn 后台 task
        let config_manager = self.config_manager.clone();
        let asset_manager = self.asset_manager.clone();
        let app_handle = self.app_handle.clone();
        let statuses = self.statuses.clone();
        let scripts = self.scripts.clone();
        let call_history = self.call_history.clone();
        let generation_contexts = self.generation_contexts.clone();

        let input_owned = input.to_string();
        let game_id_clone = game_id.clone();

        tokio::spawn(async move {
            // 在后台 task 中执行完整的生成流程
            if let Err(e) = Self::run_generation_background(
                &config_manager,
                &asset_manager,
                &app_handle,
                &statuses,
                &scripts,
                &call_history,
                &generation_contexts,
                &input_owned,
                game_type,
                use_local_fallback,
                high_quality,
                chapter_count,
                &game_id_clone,
            ).await {
                // 生成失败时通过 generation-error 事件通知前端
                log::error!("后台生成失败: game_id={}, error={:?}", game_id_clone, e);
                if let Some(ref handle) = app_handle {
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let _ = handle.emit(
                        "generation-error",
                        serde_json::json!({
                            "gameId": game_id_clone,
                            "error": format!("{:?}", e),
                            "timestamp": timestamp,
                        }),
                    );
                }
            }
        });

        // 6. 立即返回 game_id
        Ok(game_id)
    }

    /// 后台生成流程的实际执行逻辑
    async fn run_generation_background(
        config_manager: &Arc<RwLock<ConfigManager>>,
        asset_manager: &Arc<AssetManager>,
        app_handle: &Option<AppHandle>,
        statuses: &Arc<RwLock<HashMap<String, GenerationStatus>>>,
        scripts: &Arc<RwLock<HashMap<String, GameScript>>>,
        call_history: &Arc<CallHistory>,
        generation_contexts: &Arc<RwLock<HashMap<String, GenerationContext>>>,
        input: &str,
        game_type: Option<GameType>,
        use_local_fallback: bool,
        high_quality: bool,
        chapter_count: Option<u32>,
        game_id: &str,
    ) -> Result<(), ProviderError> {
        // 发送进度辅助函数
        let collected_steps: Arc<std::sync::Mutex<Vec<ProgressStep>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
        let emit = |step: &str, detail: &str, model_name: &str| {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            // 记录步骤（用于页面返回后恢复）
            if let Ok(mut steps) = collected_steps.lock() {
                steps.push(ProgressStep {
                    step: step.to_string(),
                    detail: detail.to_string(),
                    model_name: model_name.to_string(),
                    timestamp,
                });
            }
            if let Some(ref handle) = app_handle {
                let _ = handle.emit(
                    "generation-step",
                    serde_json::json!({
                        "gameId": game_id,
                        "step": step,
                        "detail": detail,
                        "modelName": model_name,
                        "timestamp": timestamp,
                    }),
                );
            }
        };

        // 获取当前文本 AI 模型的显示名称
        let model_name = {
            let cm = config_manager.read().await;
            let config = cm.get_config();
            if let Some(pc) = Self::find_provider_for_modality(config, &AIModality::Text) {
                let model = pc.models.iter()
                    .find(|m| m.is_default)
                    .map(|m| m.id.clone())
                    .unwrap_or_default();
                format!("{}/{}", pc.id, model)
            } else {
                "本地模板".to_string()
            }
        };

        // 1. 解析大纲为 GameScript
        log::info!("后台生成: 解析大纲 game_id={}", game_id);
        if high_quality {
            emit("generating_outline", "正在生成故事大纲", &model_name);
        } else {
            emit("generating_script", "正在生成游戏脚本", &model_name);
        }

        // 创建一个临时的 GenerationPipeline 实例用于调用内部方法
        // 由于 parse_outline 需要 &self，我们用静态方法替代
        let mut game_script = Self::parse_outline_static(
            config_manager,
            asset_manager,
            call_history,
            app_handle,
            generation_contexts,
            input,
            game_type.clone(),
            high_quality,
            game_id,
            chapter_count,
        ).await?;

        // 2. 提取所有 AssetRef
        emit("parsing_script", "正在解析游戏脚本", "");
        let mut asset_refs = Self::extract_asset_refs_static(&game_script);

        // 3. 为每个 AssetRef 确定来源
        Self::resolve_sources_static(config_manager, &mut asset_refs, use_local_fallback).await?;

        // 4. 将来源写回 GameScript
        Self::apply_sources_to_script(&mut game_script, &asset_refs);

        // 5. 保存 GameScript 到 script.json
        log::info!("后台生成: 保存 GameScript game_id={}", game_id);
        asset_manager
            .save_game_script(game_id, &game_script)
            .map_err(ProviderError::GenerationFailed)?;

        // 6. 存储 GameScript 供后续 regenerate_asset 使用
        scripts.write().await.insert(game_id.to_string(), game_script.clone());

        // 7. 按优先级排序
        asset_refs.sort_by(|a, b| {
            Self::asset_priority(&a.1.asset_type).cmp(&Self::asset_priority(&b.1.asset_type))
        });

        // 8. 分离第一章和后续章节
        let first_chapter_id = game_script.chapters.first().map(|c| c.id.clone());
        let (first_chapter_refs, remaining_refs): (Vec<_>, Vec<_>) = asset_refs
            .into_iter()
            .partition(|(cid, _)| first_chapter_id.as_ref() == Some(cid));

        // 9. 初始化生成状态
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
        statuses.write().await.insert(
            game_id.to_string(),
            GenerationStatus {
                game_id: game_id.to_string(),
                game_title: Some(game_script.meta.title.clone()),
                total_assets: total,
                completed_assets: 0,
                failed_assets: 0,
                chapter_status: chapter_map,
                overall_progress: 0.0,
                first_chapter_ready: false,
                background_generation_active: false,
                progress_steps: collected_steps.lock().unwrap_or_else(|e| e.into_inner()).clone(),
            },
        );

        // 10. 优先生成第一章资源
        emit("generating_assets", &format!("正在生成第一章资源（共{}项）", first_chapter_refs.len()), "");
        let first_results = Self::fetch_assets_static(
            config_manager,
            asset_manager,
            call_history,
            game_id,
            &first_chapter_refs,
        ).await;

        // 11. 处理第一章结果
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut chapter_completed: HashMap<String, usize> = HashMap::new();
        let mut chapter_failed: HashMap<String, usize> = HashMap::new();

        for (i, result) in first_results.into_iter().enumerate() {
            let (chapter_id, asset_ref) = &first_chapter_refs[i];
            match result {
                Ok(ref local_asset) => {
                    // 复制资源到游戏的 assets 目录
                    let game_asset_path = asset_manager.get_game_asset_dir(game_id);
                    // 根据资源类型确定文件名扩展名
                    let file_ext = match asset_ref.asset_type {
                        ScriptAssetType::Image => "png",
                        ScriptAssetType::Video => "mp4",
                        ScriptAssetType::Audio | ScriptAssetType::Voice => "mp3",
                    };
                    let target_path = game_asset_path.join(format!("{}.{}", asset_ref.id, file_ext));
                    
                    // 确保目录存在
                    let _ = std::fs::create_dir_all(&game_asset_path);
                    
                    // 复制文件
                    let final_path = match std::fs::copy(&local_asset.local_path, &target_path) {
                        Ok(_) => target_path.to_string_lossy().to_string(),
                        Err(_) => local_asset.local_path.clone(), // 如果复制失败，继续用原路径
                    };
                    
                    // 更新 GameScript
                    {
                        let mut scripts_lock = scripts.write().await;
                        if let Some(script) = scripts_lock.get_mut(game_id) {
                            Self::update_script_assets(script, |ar: &mut AssetRef| {
                                if ar.id == asset_ref.id {
                                    ar.url = Some(final_path.clone());
                                    ar.status = AssetStatus::Ready;
                                    ar.source = ScriptAssetSource::AiGenerated;
                                }
                            });
                            let _ = asset_manager.save_game_script(game_id, script);
                        }
                    }
                    // 发送 asset-ready 事件
                    if let Some(ref handle) = app_handle {
                        let _ = handle.emit(
                            "asset-ready",
                            serde_json::json!({
                                "gameId": game_id,
                                "assetRefId": asset_ref.id,
                                "assetType": format!("{:?}", asset_ref.asset_type),
                                "localPath": final_path,
                                "source": format!("{:?}", local_asset.source),
                            }),
                        );
                    }
                    completed += 1;
                    *chapter_completed.entry(chapter_id.clone()).or_insert(0) += 1;
                }
                Err(ref error) => {
                    if let Some(ref handle) = app_handle {
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
                    failed += 1;
                    *chapter_failed.entry(chapter_id.clone()).or_insert(0) += 1;
                }
            }
        }

        // 12. 更新第一章状态
        {
            let mut s = statuses.write().await;
            if let Some(status) = s.get_mut(game_id) {
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

        // 13. 第一章就绪后发送 generation-complete 事件
        emit("first_chapter_ready", "第一章生成完成", "");
        // 同步 progress_steps 到 statuses
        {
            let steps = collected_steps.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let mut s = statuses.write().await;
            if let Some(status) = s.get_mut(game_id) {
                status.progress_steps = steps;
            }
        }
        if let Some(ref handle) = app_handle {
            let _ = handle.emit(
                "generation-complete",
                serde_json::json!({
                    "gameId": game_id,
                    "chapterId": first_chapter_id,
                    "gameTitle": game_script.meta.title
                }),
            );
        }

        // 14. 启动后台生成后续章节
        if !remaining_refs.is_empty() {
            let config_manager_bg = config_manager.clone();
            let asset_manager_bg = asset_manager.clone();
            let app_handle_bg = app_handle.clone();
            let statuses_bg = statuses.clone();
            let call_history_bg = call_history.clone();
            let scripts_bg = scripts.clone();
            let game_id_bg = game_id.to_string();
            let first_chapter_id_bg = first_chapter_id.clone();

            tokio::spawn(async move {
                // 标记后台生成已激活
                {
                    let mut s = statuses_bg.write().await;
                    if let Some(status) = s.get_mut(&game_id_bg) {
                        status.background_generation_active = true;
                    }
                }

                // 逐章节生成
                let mut by_chapter: HashMap<String, Vec<(String, AssetRef)>> = HashMap::new();
                for (cid, aref) in remaining_refs {
                    by_chapter.entry(cid.clone()).or_default().push((cid, aref));
                }

                let mut chapter_ids: Vec<String> = by_chapter.keys().cloned().collect();
                chapter_ids.retain(|id| first_chapter_id_bg.as_ref() != Some(id));

                for chapter_id in chapter_ids {
                    let refs = match by_chapter.get(&chapter_id) {
                        Some(r) => r.clone(),
                        None => continue,
                    };

                    // 更新章节状态为 generating
                    {
                        let mut s = statuses_bg.write().await;
                        if let Some(status) = s.get_mut(&game_id_bg) {
                            if let Some(cs) = status.chapter_status.get_mut(&chapter_id) {
                                cs.status = "generating".to_string();
                            }
                        }
                    }

                    if let Some(ref handle) = app_handle_bg {
                        let _ = handle.emit(
                            "generation-progress",
                            serde_json::json!({ "gameId": game_id_bg, "chapterId": chapter_id }),
                        );
                    }

                    let results = Self::fetch_assets_static(
                        &config_manager_bg,
                        &asset_manager_bg,
                        &call_history_bg,
                        &game_id_bg,
                        &refs,
                    ).await;

                    let mut chap_completed = 0usize;
                    let mut chap_failed = 0usize;

                    for (i, result) in results.into_iter().enumerate() {
                        let (_, asset_ref) = &refs[i];
                        match result {
                            Ok(ref local_asset) => {
                                // 复制资源到游戏的 assets 目录
                                let game_asset_path = asset_manager_bg.get_game_asset_dir(&game_id_bg);
                                // 根据资源类型确定文件名扩展名
                                let file_ext = match asset_ref.asset_type {
                                    ScriptAssetType::Image => "png",
                                    ScriptAssetType::Video => "mp4",
                                    ScriptAssetType::Audio | ScriptAssetType::Voice => "mp3",
                                };
                                let target_path = game_asset_path.join(format!("{}.{}", asset_ref.id, file_ext));
                                
                                // 确保目录存在
                                let _ = std::fs::create_dir_all(&game_asset_path);
                                
                                // 复制文件
                                let final_path = match std::fs::copy(&local_asset.local_path, &target_path) {
                                    Ok(_) => target_path.to_string_lossy().to_string(),
                                    Err(_) => local_asset.local_path.clone(), // 如果复制失败，继续用原路径
                                };
                                
                                {
                                    let mut scripts_lock = scripts_bg.write().await;
                                    if let Some(script) = scripts_lock.get_mut(&game_id_bg) {
                                        Self::update_script_assets(script, |ar: &mut AssetRef| {
                                            if ar.id == asset_ref.id {
                                                ar.url = Some(final_path.clone());
                                                ar.status = AssetStatus::Ready;
                                                ar.source = ScriptAssetSource::AiGenerated;
                                            }
                                        });
                                        let _ = asset_manager_bg.save_game_script(&game_id_bg, script);
                                    }
                                }
                                if let Some(ref handle) = app_handle_bg {
                                    let _ = handle.emit(
                                        "asset-ready",
                                        serde_json::json!({
                                            "gameId": game_id_bg,
                                            "assetRefId": asset_ref.id,
                                            "assetType": format!("{:?}", asset_ref.asset_type),
                                            "localPath": final_path,
                                            "source": format!("{:?}", local_asset.source),
                                            "chapterId": chapter_id,
                                        }),
                                    );
                                }
                                chap_completed += 1;
                            }
                            Err(ref error) => {
                                if let Some(ref handle) = app_handle_bg {
                                    let _ = handle.emit(
                                        "asset-failed",
                                        serde_json::json!({
                                            "gameId": game_id_bg,
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
                        let mut s = statuses_bg.write().await;
                        if let Some(status) = s.get_mut(&game_id_bg) {
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

                            let all_done = status.chapter_status.values().all(|cs| {
                                cs.status == "ready" || cs.status == "partial"
                            });
                            if all_done {
                                status.background_generation_active = false;
                            }
                        }
                    }

                    if let Some(ref handle) = app_handle_bg {
                        let _ = handle.emit(
                            "generation-complete",
                            serde_json::json!({ "gameId": game_id_bg, "chapterId": chapter_id }),
                        );
                    }
                }

                // 全部完成
                if let Some(ref handle) = app_handle_bg {
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let _ = handle.emit(
                        "generation-step",
                        serde_json::json!({
                            "gameId": game_id_bg,
                            "step": "completed",
                            "detail": "游戏生成完成",
                            "modelName": "",
                            "timestamp": timestamp,
                        }),
                    );
                    let _ = handle.emit(
                        "generation-complete",
                        serde_json::json!({ "gameId": game_id_bg, "allChapters": true }),
                    );
                }
            });
        }

        // 15. 高质量模式下，启动后台章节生成
        if high_quality {
            let has_context = generation_contexts.read().await.contains_key(game_id);
            if has_context {
                {
                    let mut s = statuses.write().await;
                    if let Some(status) = s.get_mut(game_id) {
                        status.background_generation_active = true;
                    }
                }
                // 使用 start_remaining_chapters 的逻辑
                let ctx = {
                    let mut contexts = generation_contexts.write().await;
                    contexts.remove(game_id)
                };
                if let Some(ctx) = ctx {
                    let config_manager_hq = config_manager.clone();
                    let asset_manager_hq = asset_manager.clone();
                    let app_handle_hq = app_handle.clone();
                    let statuses_hq = statuses.clone();
                    let call_history_hq = call_history.clone();
                    let scripts_hq = scripts.clone();
                    let game_id_hq = game_id.to_string();

                    tokio::spawn(async move {
                        Self::run_remaining_chapters(
                            &config_manager_hq,
                            &asset_manager_hq,
                            &app_handle_hq,
                            &statuses_hq,
                            &scripts_hq,
                            &call_history_hq,
                            ctx,
                            &game_id_hq,
                        ).await;
                    });
                }
            }
        }

        Ok(())
    }

    /// 静态版本的 parse_outline，用于后台任务
    async fn parse_outline_static(
        config_manager: &Arc<RwLock<ConfigManager>>,
        asset_manager: &Arc<AssetManager>,
        call_history: &Arc<CallHistory>,
        app_handle: &Option<AppHandle>,
        generation_contexts: &Arc<RwLock<HashMap<String, GenerationContext>>>,
        input: &str,
        game_type: Option<GameType>,
        high_quality: bool,
        game_id: &str,
        chapter_count: Option<u32>,
    ) -> Result<GameScript, ProviderError> {
        log::info!("静态解析大纲: input_len={}, game_type={:?}, high_quality={}", input.len(), game_type, high_quality);
        let cm = config_manager.read().await;
        let config = cm.get_config();

        let text_config = Self::find_provider_for_modality(config, &AIModality::Text).cloned();

        drop(cm);

        if let Some(provider_config) = text_config {
            let deepseek = DeepSeekProvider::new(&provider_config, asset_manager.base_path())?;

            if high_quality {
                let start = Instant::now();
                let result = Self::parse_outline_high_quality_static(
                    &deepseek,
                    app_handle,
                    generation_contexts,
                    input,
                    game_type.clone(),
                    game_id,
                    chapter_count,
                ).await;
                let duration = start.elapsed().as_millis() as u64;

                let model = provider_config.models.iter()
                    .find(|m| m.is_default)
                    .map(|m| m.id.clone())
                    .unwrap_or_default();
                let endpoint = provider_config.models.iter()
                    .find(|m| m.is_default)
                    .map(|m| m.endpoint.clone())
                    .unwrap_or_default();
                let record = match &result {
                    Ok(script) => build_record(
                        &provider_config.id, "text", &model, &endpoint,
                        input, duration, "success", None,
                        Some(format!("chapters: {} (high_quality)", script.chapters.len())), None,
                    ),
                    Err(e) => build_record(
                        &provider_config.id, "text", &model, &endpoint,
                        input, duration, "error", Some(format!("{:?}", e)),
                        None, None,
                    ),
                };
                call_history.record(record);
                result
            } else {
                let parser = OutlineParser::new(deepseek);
                let start = Instant::now();
                let result = parser.parse(input, game_type).await;
                let duration = start.elapsed().as_millis() as u64;

                let model = provider_config.models.iter()
                    .find(|m| m.is_default)
                    .map(|m| m.id.clone())
                    .unwrap_or_default();
                let endpoint = provider_config.models.iter()
                    .find(|m| m.is_default)
                    .map(|m| m.endpoint.clone())
                    .unwrap_or_default();
                let record = match &result {
                    Ok(script) => build_record(
                        &provider_config.id, "text", &model, &endpoint,
                        input, duration, "success", None,
                        Some(format!("chapters: {}", script.chapters.len())), None,
                    ),
                    Err(e) => build_record(
                        &provider_config.id, "text", &model, &endpoint,
                        input, duration, "error", Some(format!("{:?}", e)),
                        None, None,
                    ),
                };
                call_history.record(record);
                result
            }
        } else {
            eprintln!("No text AI provider configured, using local template fallback");
            Ok(Self::fallback_game_script(input, game_type))
        }
    }

    /// 静态版本的高质量模式大纲解析
    async fn parse_outline_high_quality_static(
        deepseek: &DeepSeekProvider,
        app_handle: &Option<AppHandle>,
        generation_contexts: &Arc<RwLock<HashMap<String, GenerationContext>>>,
        input: &str,
        game_type: Option<GameType>,
        game_id: &str,
        chapter_count: Option<u32>,
    ) -> Result<GameScript, ProviderError> {
        let game_type_str = game_type.as_ref().map(|gt| Self::game_type_display_str(gt)).unwrap_or("互动叙事");
        let game_type_prompt = Self::get_game_type_prompt(game_type_str);

        let core_system = PROMPT_CORE_ELEMENTS.to_string();
        let effective_chapter_count = chapter_count.unwrap_or(3);

        let core_user = format!(
            "游戏类型：{}\n\n玩家构想：{}\n\n{}\n\n请生成游戏的核心要素，章节数限定为{}章。",
            game_type_str, input, game_type_prompt, effective_chapter_count
        );

        let mut messages = vec![
            crate::providers::deepseek::ChatMessage {
                role: "system".to_string(),
                content: core_system,
            },
            crate::providers::deepseek::ChatMessage {
                role: "user".to_string(),
                content: core_user,
            },
        ];

        // 阶段1：生成核心要素
        log::info!("高质量模式 阶段1: 生成核心要素 (后台)");
        if let Some(ref handle) = app_handle {
            let _ = handle.emit(
                "generation-step",
                serde_json::json!({
                    "gameId": game_id,
                    "step": "generating_core",
                    "detail": "正在生成游戏核心要素（世界观、角色、章节梗概）",
                    "modelName": "",
                    "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                }),
            );
        }
        let core_elements = deepseek.chat(messages.clone(), None).await?;
        let core_elements = OutlineParser::strip_think_tags(&core_elements);
        OutlineParser::save_raw_ai_response_sync("core_elements", &core_elements);

        messages.push(crate::providers::deepseek::ChatMessage {
            role: "assistant".to_string(),
            content: core_elements.clone(),
        });

        // 阶段2：只生成第一章详细内容
        let chapter_count_usize = effective_chapter_count as usize;
        let mut chapter_details = Vec::new();
        let chapter_label = Self::get_chapter_label(1);

        log::info!("高质量模式 阶段2: 生成{}详细内容 (后台)", chapter_label);
        if let Some(ref handle) = app_handle {
            let _ = handle.emit(
                "generation-step",
                serde_json::json!({
                    "gameId": game_id,
                    "step": "generating_chapter",
                    "detail": format!("正在生成{}详细内容（1/{}）", chapter_label, chapter_count_usize),
                    "modelName": "",
                    "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                }),
            );
        }

        let detail_user = format!(
            "现在请根据核心要素，生成{}的详细内容。\n\n\
             核心要素中关于{}的梗概是关键参考，请严格遵循。\
             同时保持与前面章节的连续性和一致性。\
             每个场景至少3-5个对话/旁白节点，选择节点至少2-3个选项。\
             场景描写要细腻，为后续生成资源提供充分的提示词依据。\n\n\
             {}",
            chapter_label, chapter_label, game_type_prompt
        );

        messages.push(crate::providers::deepseek::ChatMessage {
            role: "user".to_string(),
            content: detail_user,
        });

        let chapter_detail = deepseek.chat(messages.clone(), None).await?;
        let chapter_detail = OutlineParser::strip_think_tags(&chapter_detail);
        OutlineParser::save_raw_ai_response_sync("chapter_1_detail", &chapter_detail);

        messages.push(crate::providers::deepseek::ChatMessage {
            role: "assistant".to_string(),
            content: chapter_detail.clone(),
        });

        chapter_details.push(chapter_detail);

        // 阶段3：生成第一章的 GameScript JSON
        log::info!("高质量模式 阶段3: 生成第一章 GameScript JSON (后台)");
        if let Some(ref handle) = app_handle {
            let _ = handle.emit(
                "generation-step",
                serde_json::json!({
                    "gameId": game_id,
                    "step": "generating_script",
                    "detail": "正在根据第一章详情生成游戏脚本",
                    "modelName": "",
                    "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                }),
            );
        }

        let script_system = PROMPT_COMBINED.to_string();
        let first_chapter_detail = &chapter_details[0];
        let first_chapter_label = Self::get_chapter_label(1);

        let script_user = format!(
            "根据以下核心要素和{}的详细内容，生成该章节的游戏脚本 JSON。\n\n\
             == 核心要素 ==\n{}\n\n\
             == {} 详细内容 ==\n{}\n\n\
             要求：只生成{}的脚本，包含完整的 scenes 和 sequence。\n\
             - 严格遵循核心要素中的世界观、角色设定和剧情走向\n\
             - 每个场景至少3-5个对话/旁白节点\n\
             - 选择节点至少2-3个选项\n\
             - 场景描写要细腻，注重氛围\n\
             - 角色对话要符合其性格\n\
             - 为所有资源编写详细的生成 prompt\n\
             {}\n\n\
             玩家描述：{}",
            first_chapter_label, core_elements, first_chapter_label, first_chapter_detail,
            first_chapter_label, game_type_prompt, input
        );

        let script_messages = vec![
            crate::providers::deepseek::ChatMessage {
                role: "system".to_string(),
                content: script_system,
            },
            crate::providers::deepseek::ChatMessage {
                role: "user".to_string(),
                content: script_user,
            },
        ];

        let response = deepseek.chat(script_messages, None).await?;
        let response = OutlineParser::strip_think_tags(&response);
        OutlineParser::save_raw_ai_response_sync("high_quality_chapter1", &response);

        let json_str = Self::extract_json_from_response(&response).map_err(|e| {
            OutlineParser::save_raw_ai_response_sync("high_quality_chapter1_error", &response);
            e
        })?;
        let json_str = OutlineParser::normalize_json(&json_str);
        let mut script: GameScript = match serde_json::from_str(&json_str) {
            Ok(s) => s,
            Err(e) => {
                log::error!("GameScript JSON 解析失败: {}, 尝试宽松解析", e);
                OutlineParser::save_raw_ai_response_sync("high_quality_chapter1_json_error", &response);
                Self::parse_script_lenient(&json_str)?
            }
        };

        Self::validate_and_fix_script(&mut script)?;
        script.meta.total_chapters = effective_chapter_count;

        // 保存 GenerationContext
        let ctx = GenerationContext {
            core_elements,
            chapter_details,
            total_chapters: chapter_count_usize,
            game_type,
            input: input.to_string(),
            messages,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        generation_contexts.write().await.insert(game_id.to_string(), ctx);

        Ok(script)
    }

    /// 静态版本的 resolve_sources
    async fn resolve_sources_static(
        config_manager: &Arc<RwLock<ConfigManager>>,
        asset_refs: &mut [(String, AssetRef)],
        use_local_fallback: bool,
    ) -> Result<(), ProviderError> {
        let cm = config_manager.read().await;
        let config = cm.get_config();

        for (_, asset_ref) in asset_refs.iter_mut() {
            let modality = Self::asset_type_to_modality(&asset_ref.asset_type);
            let has_ai_provider = config
                .providers
                .iter()
                .any(|p| p.modality.contains(&modality) && p.status == ProviderStatus::Connected);

            if has_ai_provider {
                asset_ref.source = ScriptAssetSource::AiGenerated;
                asset_ref.status = AssetStatus::Pending;
            } else if use_local_fallback {
                asset_ref.source = ScriptAssetSource::Builtin;
                asset_ref.status = AssetStatus::Fallback;
            } else {
                asset_ref.source = ScriptAssetSource::Builtin;
                asset_ref.status = AssetStatus::Skipped;
            }
        }
        Ok(())
    }

    /// 后台执行后续章节生成（高质量模式专用）
    async fn run_remaining_chapters(
        config_manager: &Arc<RwLock<ConfigManager>>,
        asset_manager: &Arc<AssetManager>,
        app_handle: &Option<AppHandle>,
        statuses: &Arc<RwLock<HashMap<String, GenerationStatus>>>,
        scripts: &Arc<RwLock<HashMap<String, GameScript>>>,
        call_history: &Arc<CallHistory>,
        ctx: GenerationContext,
        game_id: &str,
    ) {
        let game_type_str = ctx.game_type.as_ref().map(|gt| Self::game_type_display_str(gt)).unwrap_or("互动叙事");
        let game_type_prompt = Self::get_game_type_prompt(game_type_str);

        let text_provider = {
            let cm = config_manager.read().await;
            let config = cm.get_config();
            let provider_config = Self::find_provider_for_modality(config, &AIModality::Text).cloned();
            drop(cm);

            match provider_config {
                Some(pc) => match DeepSeekProvider::new(&pc, asset_manager.base_path()) {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("后续章节生成失败：创建 Provider 失败: {:?}", e);
                        return;
                    }
                },
                None => {
                    log::error!("后续章节生成失败：无可用的文本 AI Provider");
                    return;
                }
            }
        };

        let model_name = {
            let cm = config_manager.read().await;
            let config = cm.get_config();
            if let Some(pc) = Self::find_provider_for_modality(config, &AIModality::Text) {
                let model = pc.models.iter().find(|m| m.is_default).map(|m| m.id.clone()).unwrap_or_default();
                format!("{}/{}", pc.id, model)
            } else {
                "未知".to_string()
            }
        };

        let mut messages = ctx.messages.clone();
        let total_chapters = ctx.total_chapters;
        let already_generated = ctx.chapter_details.len();

        for ch_idx in (already_generated + 1)..=total_chapters {
            if ctx.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                log::info!("后续章节生成已取消: game_id={}", game_id);
                if let Some(ref handle) = app_handle {
                    let _ = handle.emit(
                        "generation-step",
                        serde_json::json!({
                            "gameId": game_id,
                            "step": "cancelled",
                            "detail": "后续章节生成已取消",
                            "modelName": "",
                            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                        }),
                    );
                }
                return;
            }

            let label = Self::get_chapter_label(ch_idx);

            if let Some(ref handle) = app_handle {
                let _ = handle.emit(
                    "generation-step",
                    serde_json::json!({
                        "gameId": game_id,
                        "step": "generating_chapter",
                        "detail": format!("正在生成{}详细内容（{}/{}）", label, ch_idx, total_chapters),
                        "modelName": model_name,
                        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                    }),
                );
            }

            let detail_user = format!(
                "现在请根据核心要素，生成{}的详细内容。\n\n\
                 核心要素中关于{}的梗概是关键参考，请严格遵循。\
                 同时保持与前面章节的连续性和一致性。\
                 每个场景至少3-5个对话/旁白节点，选择节点至少2-3个选项。\
                 场景描写要细腻，为后续生成资源提供充分的提示词依据。\n\n\
                 {}",
                label, label, game_type_prompt
            );

            messages.push(crate::providers::deepseek::ChatMessage {
                role: "user".to_string(),
                content: detail_user,
            });

            let chapter_detail = match text_provider.chat(messages.clone(), None).await {
                Ok(d) => OutlineParser::strip_think_tags(&d),
                Err(e) => {
                    log::error!("第{}章详情生成失败: {:?}", ch_idx, e);
                    messages.pop();
                    continue;
                }
            };

            OutlineParser::save_raw_ai_response_sync(
                &format!("chapter_{}_detail", ch_idx),
                &chapter_detail,
            );

            messages.push(crate::providers::deepseek::ChatMessage {
                role: "assistant".to_string(),
                content: chapter_detail.clone(),
            });

            if ctx.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                log::info!("后续章节生成已取消: game_id={}", game_id);
                return;
            }

            if let Some(ref handle) = app_handle {
                let _ = handle.emit(
                    "generation-step",
                    serde_json::json!({
                        "gameId": game_id,
                        "step": "generating_chapter_script",
                        "detail": format!("正在生成{}游戏脚本", label),
                        "modelName": model_name,
                        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                    }),
                );
            }

            let script_user = format!(
                "根据核心要素和{}的详细内容，生成该章节的游戏脚本 JSON。\n\n\
                 == 核心要素 ==\n{}\n\n\
                 == {} 详细内容 ==\n{}\n\n\
                 要求：只生成{}的脚本，包含完整的 scenes 和 sequence。\n\
                 - 严格遵循核心要素中的世界观、角色设定和剧情走向\n\
                 - 每个场景至少3-5个对话/旁白节点\n\
                 - 选择节点至少2-3个选项\n\
                 - 场景描写要细腻，注重氛围\n\
                 - 角色对话要符合其性格\n\
                 - 为所有资源编写详细的生成 prompt\n\
                 {}",
                label, ctx.core_elements, label, chapter_detail, label, game_type_prompt
            );

            let script_messages = vec![
                crate::providers::deepseek::ChatMessage {
                    role: "system".to_string(),
                    content: PROMPT_COMBINED.to_string(),
                },
                crate::providers::deepseek::ChatMessage {
                    role: "user".to_string(),
                    content: script_user,
                },
            ];

            let response = match text_provider.chat(script_messages, None).await {
                Ok(r) => OutlineParser::strip_think_tags(&r),
                Err(e) => {
                    log::error!("第{}章脚本生成失败: {:?}", ch_idx, e);
                    continue;
                }
            };

            match Self::extract_json_from_response(&response) {
                Ok(json_str) => {
                    let json_str = OutlineParser::normalize_json(&json_str);
                    match serde_json::from_str::<GameScript>(&json_str) {
                        Ok(mut partial_script) => {
                            Self::validate_and_fix_script(&mut partial_script).ok();

                            if !partial_script.chapters.is_empty() {
                                let new_chapter = partial_script.chapters.remove(0);
                                let chapter_id = new_chapter.id.clone();
                                let chapter_title = new_chapter.title.clone();

                                {
                                    let mut scripts_lock = scripts.write().await;
                                    if let Some(existing_script) = scripts_lock.get_mut(game_id) {
                                        existing_script.chapters.push(new_chapter);
                                        let _ = asset_manager.save_game_script(game_id, existing_script);
                                    }
                                }

                                {
                                    let mut s = statuses.write().await;
                                    if let Some(status) = s.get_mut(game_id) {
                                        status.chapter_status.insert(
                                            chapter_id.clone(),
                                            ChapterStatus {
                                                chapter_id: chapter_id.clone(),
                                                chapter_title: chapter_title.clone(),
                                                total_assets: 0,
                                                completed_assets: 0,
                                                status: "pending".to_string(),
                                            },
                                        );
                                    }
                                }

                                if let Some(ref handle) = app_handle {
                                    let _ = handle.emit(
                                        "chapter-ready",
                                        serde_json::json!({
                                            "gameId": game_id,
                                            "chapterIndex": ch_idx - 1,
                                            "totalChapters": total_chapters,
                                            "chapterId": chapter_id,
                                            "chapterTitle": chapter_title,
                                        }),
                                    );
                                }

                                let asset_refs = {
                                    let scripts_lock = scripts.read().await;
                                    if let Some(script) = scripts_lock.get(game_id) {
                                        Self::extract_asset_refs_static(script)
                                            .into_iter()
                                            .filter(|(cid, _)| cid == &chapter_id)
                                            .collect::<Vec<_>>()
                                    } else {
                                        Vec::new()
                                    }
                                };

                                if !asset_refs.is_empty() {
                                    {
                                        let mut s = statuses.write().await;
                                        if let Some(status) = s.get_mut(game_id) {
                                            if let Some(cs) = status.chapter_status.get_mut(&chapter_id) {
                                                cs.total_assets = asset_refs.len();
                                                cs.status = "generating".to_string();
                                            }
                                            status.total_assets += asset_refs.len();
                                        }
                                    }

                                    let results = Self::fetch_assets_static(
                                        config_manager,
                                        asset_manager,
                                        call_history,
                                        game_id,
                                        &asset_refs,
                                    ).await;

                                    let mut chap_completed = 0usize;
                                    let mut chap_failed = 0usize;

                                    for (i, result) in results.into_iter().enumerate() {
                                        let (_, asset_ref) = &asset_refs[i];
                                        match result {
                                            Ok(ref local_asset) => {
                                                {
                                                    let mut scripts_lock = scripts.write().await;
                                                    if let Some(script) = scripts_lock.get_mut(game_id) {
                                                        Self::update_script_assets(script, |ar: &mut AssetRef| {
                                                            if ar.id == asset_ref.id {
                                                                ar.url = Some(local_asset.local_path.clone());
                                                                ar.status = AssetStatus::Ready;
                                                                ar.source = ScriptAssetSource::AiGenerated;
                                                            }
                                                        });
                                                        let _ = asset_manager.save_game_script(game_id, script);
                                                    }
                                                }
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

                                    {
                                        let mut s = statuses.write().await;
                                        if let Some(status) = s.get_mut(game_id) {
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
                                        }
                                    }
                                }

                                log::info!("第{}章生成完成: game_id={}", ch_idx, game_id);
                            }
                        }
                        Err(e) => {
                            log::warn!("第{}章脚本解析失败: {}, 跳过", ch_idx, e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("第{}章JSON提取失败: {:?}, 跳过", ch_idx, e);
                }
            }
        }

        // 所有章节生成完成
        {
            let mut s = statuses.write().await;
            if let Some(status) = s.get_mut(game_id) {
                status.background_generation_active = false;
            }
        }

        if let Some(ref handle) = app_handle {
            let _ = handle.emit(
                "all-chapters-ready",
                serde_json::json!({ "gameId": game_id }),
            );
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = handle.emit(
                "generation-step",
                serde_json::json!({
                    "gameId": game_id,
                    "step": "completed",
                    "detail": "所有章节生成完成",
                    "modelName": "",
                    "timestamp": timestamp,
                }),
            );
            let _ = handle.emit(
                "generation-complete",
                serde_json::json!({ "gameId": game_id, "allChapters": true }),
            );
        }

        log::info!("所有后续章节生成完成: game_id={}", game_id);
    }

    /// 从已有的 GameScript 直接创建游戏（跳过大纲解析步骤，用于调试）
    pub async fn create_game_from_script(
        &self,
        mut game_script: GameScript,
    ) -> Result<(String, GameScript), ProviderError> {
        log::info!("从脚本创建游戏: title={}", game_script.meta.title);

        // 1. 生成 game_id
        let game_id = Uuid::new_v4().to_string();

        // 2. 创建游戏目录
        self.asset_manager
            .ensure_game_dirs(&game_id)
            .map_err(ProviderError::GenerationFailed)?;

        // 3. 提取所有 AssetRef
        let mut asset_refs = self.extract_asset_refs(&game_script);

        // 4. 为每个 AssetRef 确定来源
        self.resolve_sources(&mut asset_refs, true).await?;

        // 5. 将来源写回 GameScript
        Self::apply_sources_to_script(&mut game_script, &asset_refs);

        // 6. 保存 GameScript 到 script.json
        log::info!("保存 GameScript: game_id={}", game_id);
        self.asset_manager
            .save_game_script(&game_id, &game_script)
            .map_err(ProviderError::GenerationFailed)?;

        // 7. 存储 GameScript 供后续 regenerate_asset 使用
        self.scripts.write().await.insert(game_id.clone(), game_script.clone());

        // 8. 按优先级排序
        asset_refs.sort_by(|a, b| {
            Self::asset_priority(&a.1.asset_type).cmp(&Self::asset_priority(&b.1.asset_type))
        });

        // 9. 分离第一章和后续章节
        let first_chapter_id = game_script.chapters.first().map(|c| c.id.clone());
        let (first_chapter_refs, remaining_refs): (Vec<_>, Vec<_>) = asset_refs
            .into_iter()
            .partition(|(cid, _)| first_chapter_id.as_ref() == Some(cid));

        // 10. 初始化生成状态
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
                game_title: Some(game_script.meta.title.clone()),
                total_assets: total,
                completed_assets: 0,
                failed_assets: 0,
                chapter_status: chapter_map,
                overall_progress: 0.0,
                first_chapter_ready: false,
                background_generation_active: false,
                progress_steps: Vec::new(),
            },
        );

        // 11. 优先生成第一章资源
        let first_results = self.fetch_assets(&game_id, &first_chapter_refs).await;

        // 12. 处理第一章结果
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut chapter_completed: HashMap<String, usize> = HashMap::new();
        let mut chapter_failed: HashMap<String, usize> = HashMap::new();

        for (i, result) in first_results.into_iter().enumerate() {
            let (chapter_id, asset_ref) = &first_chapter_refs[i];
            match result {
                Ok(ref local_asset) => {
                    self.on_asset_ready(&game_id, asset_ref, local_asset).await;
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

        // 13. 更新第一章状态
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

        // 14. 第一章就绪后立即发送 generation-complete 事件
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(
                "generation-complete",
                serde_json::json!({ "gameId": game_id, "chapterId": first_chapter_id }),
            );
        }

        // 15. 启动后台生成后续章节
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
        let call_history = self.call_history.clone();
        let scripts = self.scripts.clone();

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
                    &call_history,
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
                            // 更新 GameScript 中对应 AssetRef 的 url 字段
                            {
                                let mut scripts = scripts.write().await;
                                if let Some(script) = scripts.get_mut(&game_id) {
                                    Self::update_script_assets(script, |ar: &mut AssetRef| {
                                        if ar.id == asset_ref.id {
                                            ar.url = Some(local_asset.local_path.clone());
                                            ar.status = AssetStatus::Ready;
                                            ar.source = ScriptAssetSource::AiGenerated;
                                        }
                                    });
                                    let _ = asset_manager.save_game_script(&game_id, script);
                                }
                            }
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
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let _ = handle.emit(
                    "generation-step",
                    serde_json::json!({
                        "gameId": game_id,
                        "step": "completed",
                        "detail": "游戏生成完成",
                        "modelName": "",
                        "timestamp": timestamp,
                    }),
                );
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
        let provider = self.resolve_ai_provider(modality.clone()).await?;

        let start = Instant::now();
        let result = provider.get_asset(&asset_ref).await;
        let duration = start.elapsed().as_millis() as u64;

        // 记录调用历史
        let (provider_id, model, endpoint) = self.get_provider_info(&modality).await;
        let modality_str = format!("{:?}", modality).to_lowercase();
        let record = match &result {
            Ok(local_asset) => build_record(
                &provider_id, &modality_str, &model, &endpoint,
                &asset_ref.prompt, duration, "success", None,
                Some(local_asset.local_path.clone()), None,
            ),
            Err(e) => build_record(
                &provider_id, &modality_str, &model, &endpoint,
                &asset_ref.prompt, duration, "error", Some(format!("{:?}", e)),
                None, None,
            ),
        };
        self.call_history.record(record);

        match result {
            Ok(local_asset) => {
                self.on_asset_ready(game_id, &asset_ref, &local_asset).await;
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
            self.on_asset_ready(game_id, &asset_ref, &candidates[0]).await;
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

        let provider_config = Self::find_provider_for_modality(config, &modality);

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
        call_history: &Arc<CallHistory>,
        _game_id: &str,
        asset_refs: &[(String, AssetRef)],
    ) -> Vec<Result<LocalAsset, ProviderError>> {
        let mut handles: Vec<tokio::task::JoinHandle<Result<LocalAsset, ProviderError>>> =
            Vec::with_capacity(asset_refs.len());

        for (_chapter_id, asset_ref) in asset_refs {
            let asset_ref = asset_ref.clone();
            let config_manager = config_manager.clone();
            let asset_base_path = asset_manager.base_path().to_path_buf();
            let call_history = call_history.clone();

            let handle = tokio::spawn(async move {
                let modality = Self::asset_type_to_modality(&asset_ref.asset_type);

                let provider = {
                    let config_mgr = config_manager.read().await;
                    let config = config_mgr.get_config();
                    let pc = Self::find_provider_for_modality(config, &modality);
                    if let Some(pc) = pc {
                        ProviderFactory::create(pc, &asset_base_path)?
                    } else {
                        let builtin_path = asset_base_path.join("builtin-assets");
                        let games_path = asset_base_path.join("games");
                        Box::new(BuiltinAssetProvider::new(builtin_path, games_path, asset_base_path.clone()))
                            as Box<dyn IAssetProvider>
                    }
                };

                let start = Instant::now();
                let result = provider.get_asset(&asset_ref).await;
                let duration = start.elapsed().as_millis() as u64;

                // 记录调用历史
                let (provider_id, model, endpoint) = {
                    let cm = config_manager.read().await;
                    let config = cm.get_config();
                    if let Some(pc) = Self::find_provider_for_modality(config, &modality) {
                        let m = pc.models.iter().find(|m| m.is_default).map(|m| m.id.clone()).unwrap_or_default();
                        let e = pc.models.iter().find(|m| m.is_default).map(|m| m.endpoint.clone()).unwrap_or_default();
                        (pc.id.clone(), m, e)
                    } else {
                        ("builtin".to_string(), "default".to_string(), String::new())
                    }
                };
                let modality_str = format!("{:?}", modality).to_lowercase();
                let record = match &result {
                    Ok(local_asset) => build_record(
                        &provider_id, &modality_str, &model, &endpoint,
                        &asset_ref.prompt, duration, "success", None,
                        Some(local_asset.local_path.clone()), None,
                    ),
                    Err(e) => build_record(
                        &provider_id, &modality_str, &model, &endpoint,
                        &asset_ref.prompt, duration, "error", Some(format!("{:?}", e)),
                        None, None,
                    ),
                };
                call_history.record(record);

                match result {
                    Ok(asset) => Ok(asset),
                    Err(original_error) => {
                        let retry_start = Instant::now();
                        let retry_result = provider.get_asset(&asset_ref).await;
                        let retry_duration = retry_start.elapsed().as_millis() as u64;

                        let retry_record = match &retry_result {
                            Ok(local_asset) => build_record(
                                &provider_id, &modality_str, &model, &endpoint,
                                &asset_ref.prompt, retry_duration, "success", None,
                                Some(local_asset.local_path.clone()), None,
                            ),
                            Err(e) => build_record(
                                &provider_id, &modality_str, &model, &endpoint,
                                &asset_ref.prompt, retry_duration, "retry_error", Some(format!("{:?}", e)),
                                None, None,
                            ),
                        };
                        call_history.record(retry_record);

                        match retry_result {
                            Ok(asset) => Ok(asset),
                            Err(_) => {
                                let builtin_path = asset_base_path.join("builtin-assets");
                                let games_path = asset_base_path.join("games");
                                let builtin = BuiltinAssetProvider::new(builtin_path, games_path, asset_base_path.clone());
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

    /// 宽松解析 GameScript JSON：逐章解析，跳过有问题的节点
    fn parse_script_lenient(json_str: &str) -> Result<GameScript, ProviderError> {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| ProviderError::GenerationFailed(format!("JSON 格式无效: {}", e)))?;

        // 提取 meta
        let meta = value.get("meta")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        // 逐章解析
        let chapters = value.get("chapters")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|ch| {
                        if let Some(scenes) = ch.get("scenes").and_then(|v| v.as_array()) {
                            let fixed_scenes: Vec<serde_json::Value> = scenes.iter()
                                .filter_map(|scene| {
                                    if let Some(seq) = scene.get("sequence").and_then(|v| v.as_array()) {
                                        let fixed_seq: Vec<serde_json::Value> = seq.iter()
                                            .filter_map(|node| {
                                                match serde_json::from_value::<SceneNode>(node.clone()) {
                                                    Ok(_) => Some(node.clone()),
                                                    Err(e) => {
                                                        log::warn!("跳过无法解析的节点: {}", e);
                                                        // 尝试降级为 action
                                                        let mut fixed = node.clone();
                                                        if let Some(obj) = fixed.as_object_mut() {
                                                            obj.insert("type".to_string(), serde_json::Value::String("action".to_string()));
                                                        }
                                                        match serde_json::from_value::<SceneNode>(fixed.clone()) {
                                                            Ok(_) => Some(fixed),
                                                            Err(_) => None,
                                                        }
                                                    }
                                                }
                                            })
                                            .collect();
                                        let mut fixed_scene = scene.clone();
                                        if let Some(obj) = fixed_scene.as_object_mut() {
                                            obj.insert("sequence".to_string(), serde_json::Value::Array(fixed_seq));
                                        }
                                        Some(fixed_scene)
                                    } else {
                                        Some(scene.clone())
                                    }
                                })
                                .collect();
                            let mut fixed_ch = ch.clone();
                            if let Some(obj) = fixed_ch.as_object_mut() {
                                obj.insert("scenes".to_string(), serde_json::Value::Array(fixed_scenes));
                            }
                            serde_json::from_value(fixed_ch).ok()
                        } else {
                            serde_json::from_value(ch.clone()).ok()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(GameScript { meta, chapters, global_variables: vec![] })
    }

    /// 获取章节标签（如"第一章"）
    fn get_chapter_label(idx: usize) -> String {
        let labels = ["第一章", "第二章", "第三章", "第四章", "第五章", "第六章", "第七章", "第八章", "第九章", "第十章"];
        if idx <= labels.len() {
            labels[idx - 1].to_string()
        } else {
            format!("第{}章", idx)
        }
    }

    /// 从 AI 响应中提取 JSON（与 OutlineParser::extract_json 逻辑一致）
    fn extract_json_from_response(response: &str) -> Result<String, ProviderError> {
        let trimmed = response.trim();

        // 尝试提取 ```json ... ``` 代码块
        if let Some(start) = trimmed.find("```json") {
            let json_start = start + 7;
            if let Some(end) = trimmed[json_start..].find("```") {
                let json_str = trimmed[json_start..json_start + end].trim();
                return Ok(json_str.to_string());
            }
        }

        // 尝试提取 ``` ... ``` 代码块（无语言标记）
        if let Some(start) = trimmed.find("```") {
            let json_start = start + 3;
            let after_ticks = &trimmed[json_start..];
            let first_newline = after_ticks.find('\n').unwrap_or(0);
            let content_start = json_start + first_newline + 1;
            if let Some(end) = trimmed[content_start..].find("```") {
                let json_str = trimmed[content_start..content_start + end].trim();
                if json_str.starts_with('{') {
                    return Ok(json_str.to_string());
                }
            }
        }

        // 尝试直接提取 JSON 对象
        if let Some(start) = trimmed.find('{') {
            let mut depth = 0;
            let mut end = start;
            for (i, c) in trimmed[start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if depth == 0 {
                return Ok(trimmed[start..end].to_string());
            }
        }

        Err(ProviderError::GenerationFailed(
            "No valid JSON found in AI response".to_string(),
        ))
    }

    /// 获取游戏类型对应的增强提示词
    fn get_game_type_prompt(game_type: &str) -> &'static str {
        match game_type {
            "视觉小说" => "这是一个视觉小说游戏。注重角色对话和情感发展，每个场景应有丰富的对话选项影响剧情走向。场景描述要细腻，注重氛围营造。",
            "RPG" => "这是一个RPG游戏。注重角色成长系统（属性、技能、装备），战斗场景，任务系统。设计多条支线任务和隐藏内容。每个选择应有明确的属性影响。",
            "悬疑解谜" => "这是一个悬疑解谜游戏。注重线索收集和逻辑推理，设计多层谜题和反转。每个场景应隐藏关键线索，选择影响推理方向。确保谜题有逻辑自洽的解答。",
            "恐怖生存" => "这是一个恐怖生存游戏。注重恐怖氛围营造，资源管理，生死抉择。场景描述要有压迫感，音效提示要充分。设计多个jump scare时机和逃生路线。",
            "模拟经营" => "这是一个模拟经营游戏。注重资源管理系统，经营决策，时间推进。设计经济系统和随机事件。每个选择影响经营状况。",
            _ => "",
        }
    }

    /// 获取游戏类型的显示名称（字符串版本）
    fn game_type_display_str(game_type: &GameType) -> &'static str {
        match game_type {
            GameType::VisualNovel => "视觉小说",
            GameType::Rpg => "RPG",
            GameType::Mystery => "悬疑解谜",
            GameType::Horror => "恐怖生存",
            GameType::Simulation => "模拟经营",
        }
    }

    /// 校验并修复 GameScript（用于高质量模式的独立调用）
    fn validate_and_fix_script(script: &mut GameScript) -> Result<(), ProviderError> {
        use crate::engine::validator::GameScriptValidator;

        // 修复缺失的 id
        for (ch_idx, chapter) in script.chapters.iter_mut().enumerate() {
            if chapter.id.is_empty() {
                chapter.id = format!("chapter_{}", ch_idx + 1);
            }
            for (sc_idx, scene) in chapter.scenes.iter_mut().enumerate() {
                if scene.id.is_empty() {
                    scene.id = format!("scene_{}_{}", ch_idx + 1, sc_idx + 1);
                }
                for (node_idx, node) in scene.sequence.iter_mut().enumerate() {
                    let node_id = format!("node_{}_{}_{}", ch_idx + 1, sc_idx + 1, node_idx + 1);
                    if let SceneNode::Narration(n) = node {
                        if n.id.is_empty() { n.id = node_id; }
                    } else if let SceneNode::Dialogue(d) = node {
                        if d.id.is_empty() { d.id = node_id; }
                    } else if let SceneNode::Choice(c) = node {
                        if c.id.is_empty() { c.id = node_id; }
                    } else if let SceneNode::Condition(c) = node {
                        if c.id.is_empty() { c.id = node_id; }
                    } else if let SceneNode::Action(a) = node {
                        if a.id.is_empty() { a.id = node_id; }
                    } else if let SceneNode::Cg(c) = node {
                        if c.id.is_empty() { c.id = node_id; }
                    } else if let SceneNode::SceneTransition(t) = node {
                        if t.id.is_empty() { t.id = node_id; }
                    }
                }
            }
        }

        // 修复 AssetRef status
        for chapter in &mut script.chapters {
            for scene in &mut chapter.scenes {
                Self::fix_scene_assets_inline(&mut scene.assets);
                for node in &mut scene.sequence {
                    match node {
                        SceneNode::Narration(n) => {
                            if let Some(ref mut voice) = n.voice_asset {
                                Self::fix_asset_ref_inline(voice);
                            }
                        }
                        SceneNode::Dialogue(d) => {
                            if let Some(ref mut avatar) = d.speaker_avatar {
                                Self::fix_asset_ref_inline(avatar);
                            }
                            if let Some(ref mut voice) = d.voice_asset {
                                Self::fix_asset_ref_inline(voice);
                            }
                        }
                        SceneNode::Cg(c) => {
                            Self::fix_asset_ref_inline(&mut c.video_asset);
                        }
                        _ => {}
                    }
                }
            }
        }

        let validator = GameScriptValidator::new();
        let result = validator.validate_and_fix(script);
        if !result.is_valid {
            eprintln!(
                "GameScript validation warnings: {:?}",
                result.warnings
            );
        }
        Ok(())
    }

    fn fix_scene_assets_inline(assets: &mut SceneAssets) {
        if let Some(ref mut bg) = assets.background_image {
            Self::fix_asset_ref_inline(bg);
        }
        if let Some(ref mut video) = assets.background_video {
            Self::fix_asset_ref_inline(video);
        }
        if let Some(ref mut bgm) = assets.bgm {
            Self::fix_asset_ref_inline(bgm);
        }
        if let Some(ref mut ambient) = assets.ambient_sound {
            Self::fix_asset_ref_inline(ambient);
        }
        if let Some(ref mut cg) = assets.cg_animation {
            Self::fix_asset_ref_inline(cg);
        }
    }

    fn fix_asset_ref_inline(asset: &mut AssetRef) {
        if asset.id.is_empty() {
            asset.id = uuid::Uuid::new_v4().to_string();
        }
        match asset.status {
            AssetStatus::Ready | AssetStatus::Fallback => {
                if asset.url.is_none() && asset.builtin_asset_id.is_none() {
                    asset.status = AssetStatus::Pending;
                }
            }
            _ => {}
        }
    }

    /// 无 AI 配置时的本地模板生成
    fn fallback_game_script(input: &str, game_type: Option<GameType>) -> GameScript {
        let gt = game_type.unwrap_or(GameType::VisualNovel);
        let title = if input.chars().count() > 20 {
            format!("{}...", &input.chars().take(20).collect::<String>())
        } else {
            input.to_string()
        };

        use crate::types::game_script::{
            GameMeta, Chapter, Scene, SceneAssets, DialogueNode, NarrationNode, ChoiceNode, ChoiceOption,
            Transition, TransitionKind,
        };

        GameScript {
            meta: GameMeta {
                title: title.clone(),
                game_type: gt.clone(),
                total_chapters: 3,
                description: input.to_string(),
                themes: vec!["auto-generated".to_string()],
                tone: "neutral".to_string(),
            },
            global_variables: vec![],
            chapters: vec![
                Chapter {
                    id: "ch1".to_string(),
                    title: "第一章：启程".to_string(),
                    summary: "故事开始".to_string(),
                    chapter_variables: vec![],
                    scenes: vec![Scene {
                        id: "ch1_s1".to_string(),
                        title: "故事开始".to_string(),
                        description: "冒险的起点".to_string(),
                        assets: SceneAssets {
                            background_image: None,
                            background_video: None,
                            bgm: None,
                            ambient_sound: None,
                            cg_animation: None,
                        },
                        transitions: vec![Transition {
                            from_scene_id: "ch1_s1".to_string(),
                            to_scene_id: "ch2_s1".to_string(),
                            transition_type: TransitionKind::Fade,
                            duration: 1.0,
                        }],
                        sequence: vec![
                            SceneNode::Narration(NarrationNode {
                                id: "ch1_n1".to_string(),
                                text: format!("{}——一段新的冒险就此展开。", input),
                                voice_prompt: None,
                                voice_asset: None,
                            }),
                            SceneNode::Dialogue(DialogueNode {
                                id: "ch1_d1".to_string(),
                                speaker: "旁白".to_string(),
                                speaker_avatar: None,
                                text: "你站在命运的十字路口，前方是未知的旅途。".to_string(),
                                emotion: None,
                                voice_asset: None,
                            }),
                            SceneNode::Choice(ChoiceNode {
                                id: "ch1_c1".to_string(),
                                prompt: "你将如何选择？".to_string(),
                                options: vec![
                                    ChoiceOption {
                                        text: "勇敢前行".to_string(),
                                        next_node_id: Some("ch1_d2".to_string()),
                                        condition: None,
                                        effects: None,
                                    },
                                    ChoiceOption {
                                        text: "谨慎观察".to_string(),
                                        next_node_id: Some("ch1_d3".to_string()),
                                        condition: None,
                                        effects: None,
                                    },
                                ],
                            }),
                            SceneNode::Dialogue(DialogueNode {
                                id: "ch1_d2".to_string(),
                                speaker: "旁白".to_string(),
                                speaker_avatar: None,
                                text: "你鼓起勇气，踏上了旅途。".to_string(),
                                emotion: None,
                                voice_asset: None,
                            }),
                            SceneNode::Dialogue(DialogueNode {
                                id: "ch1_d3".to_string(),
                                speaker: "旁白".to_string(),
                                speaker_avatar: None,
                                text: "你仔细观察着周围的一切，寻找线索。".to_string(),
                                emotion: None,
                                voice_asset: None,
                            }),
                        ],
                    }],
                },
                Chapter {
                    id: "ch2".to_string(),
                    title: "第二章：探索".to_string(),
                    summary: "深入探索".to_string(),
                    chapter_variables: vec![],
                    scenes: vec![Scene {
                        id: "ch2_s1".to_string(),
                        title: "深入探索".to_string(),
                        description: "发现更多秘密".to_string(),
                        assets: SceneAssets {
                            background_image: None,
                            background_video: None,
                            bgm: None,
                            ambient_sound: None,
                            cg_animation: None,
                        },
                        transitions: vec![Transition {
                            from_scene_id: "ch2_s1".to_string(),
                            to_scene_id: "ch3_s1".to_string(),
                            transition_type: TransitionKind::Fade,
                            duration: 1.0,
                        }],
                        sequence: vec![
                            SceneNode::Narration(NarrationNode {
                                id: "ch2_n1".to_string(),
                                text: "随着旅途的深入，你发现了更多秘密。".to_string(),
                                voice_prompt: None,
                                voice_asset: None,
                            }),
                        ],
                    }],
                },
                Chapter {
                    id: "ch3".to_string(),
                    title: "第三章：终局".to_string(),
                    summary: "命运抉择".to_string(),
                    chapter_variables: vec![],
                    scenes: vec![Scene {
                        id: "ch3_s1".to_string(),
                        title: "命运抉择".to_string(),
                        description: "一切终将迎来结局".to_string(),
                        assets: SceneAssets {
                            background_image: None,
                            background_video: None,
                            bgm: None,
                            ambient_sound: None,
                            cg_animation: None,
                        },
                        transitions: vec![],
                        sequence: vec![
                            SceneNode::Narration(NarrationNode {
                                id: "ch3_n1".to_string(),
                                text: "一切终将迎来结局。".to_string(),
                                voice_prompt: None,
                                voice_asset: None,
                            }),
                        ],
                    }],
                },
            ],
        }
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

    /// 为每个 AssetRef 确定来源（AI 已配置 → ai_generated，否则根据 use_local_fallback 决定）
    async fn resolve_sources(
        &self,
        asset_refs: &mut [(String, AssetRef)],
        use_local_fallback: bool,
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
            } else if use_local_fallback {
                asset_ref.source = ScriptAssetSource::Builtin;
                asset_ref.status = AssetStatus::Fallback;
            } else {
                asset_ref.source = ScriptAssetSource::Builtin;
                asset_ref.status = AssetStatus::Skipped;
            }
        }
        Ok(())
    }

    /// 根据模态选择 Provider
    #[allow(dead_code)]
    async fn resolve_provider(
        &self,
        modality: AIModality,
    ) -> Result<Box<dyn IAssetProvider>, ProviderError> {
        let config_manager = self.config_manager.read().await;
        let config = config_manager.get_config();

        let provider_config = Self::find_provider_for_modality(config, &modality);

        if let Some(pc) = provider_config {
            ProviderFactory::create(pc, self.asset_manager.base_path())
        } else {
            self.create_builtin_provider()
        }
    }

    /// 创建内置 Provider
    #[allow(dead_code)]
    fn create_builtin_provider(&self) -> Result<Box<dyn IAssetProvider>, ProviderError> {
        let builtin_assets_path = self.asset_manager.base_path().join("builtin-assets");
        let game_assets_path = self.asset_manager.base_path().join("games");
        let autofree_base_path = self.asset_manager.base_path().to_path_buf();
        Ok(Box::new(BuiltinAssetProvider::new(
            builtin_assets_path,
            game_assets_path,
            autofree_base_path,
        )))
    }

    /// 并行获取资源
    async fn fetch_assets(
        &self,
        _game_id: &str,
        asset_refs: &[(String, AssetRef)],
    ) -> Vec<Result<LocalAsset, ProviderError>> {
        let mut handles: Vec<tokio::task::JoinHandle<Result<LocalAsset, ProviderError>>> =
            Vec::with_capacity(asset_refs.len());

        for (_chapter_id, asset_ref) in asset_refs {
            let asset_ref = asset_ref.clone();
            let config_manager = self.config_manager.clone();
            let asset_base_path = self.asset_manager.base_path().to_path_buf();
            let call_history = self.call_history.clone();

            let handle = tokio::spawn(async move {
                let modality = Self::asset_type_to_modality(&asset_ref.asset_type);

                // 创建 Provider
                let provider = {
                    let config_mgr = config_manager.read().await;
                    let config = config_mgr.get_config();
                    let pc = Self::find_provider_for_modality(config, &modality);
                    if let Some(pc) = pc {
                        ProviderFactory::create(pc, &asset_base_path)?
                    } else {
                        let builtin_path = asset_base_path.join("builtin-assets");
                        let games_path = asset_base_path.join("games");
                        Box::new(BuiltinAssetProvider::new(builtin_path, games_path, asset_base_path.clone()))
                            as Box<dyn IAssetProvider>
                    }
                };

                // 尝试获取资源
                let start = Instant::now();
                let result = provider.get_asset(&asset_ref).await;
                let duration = start.elapsed().as_millis() as u64;

                // 记录调用历史
                let (provider_id, model, endpoint) = {
                    let cm = config_manager.read().await;
                    let config = cm.get_config();
                    if let Some(pc) = Self::find_provider_for_modality(config, &modality) {
                        let m = pc.models.iter().find(|m| m.is_default).map(|m| m.id.clone()).unwrap_or_default();
                        let e = pc.models.iter().find(|m| m.is_default).map(|m| m.endpoint.clone()).unwrap_or_default();
                        (pc.id.clone(), m, e)
                    } else {
                        ("builtin".to_string(), "default".to_string(), String::new())
                    }
                };
                let modality_str = format!("{:?}", modality).to_lowercase();
                let record = match &result {
                    Ok(local_asset) => build_record(
                        &provider_id, &modality_str, &model, &endpoint,
                        &asset_ref.prompt, duration, "success", None,
                        Some(local_asset.local_path.clone()), None,
                    ),
                    Err(e) => build_record(
                        &provider_id, &modality_str, &model, &endpoint,
                        &asset_ref.prompt, duration, "error", Some(format!("{:?}", e)),
                        None, None,
                    ),
                };
                call_history.record(record);

                // 失败时重试 1 次，仍失败则降级到 BuiltinAssetProvider
                match result {
                    Ok(asset) => Ok(asset),
                    Err(original_error) => {
                        // 重试 1 次
                        let retry_start = Instant::now();
                        let retry_result = provider.get_asset(&asset_ref).await;
                        let retry_duration = retry_start.elapsed().as_millis() as u64;

                        // 记录重试调用
                        let retry_record = match &retry_result {
                            Ok(local_asset) => build_record(
                                &provider_id, &modality_str, &model, &endpoint,
                                &asset_ref.prompt, retry_duration, "success", None,
                                Some(local_asset.local_path.clone()), None,
                            ),
                            Err(e) => build_record(
                                &provider_id, &modality_str, &model, &endpoint,
                                &asset_ref.prompt, retry_duration, "retry_error", Some(format!("{:?}", e)),
                                None, None,
                            ),
                        };
                        call_history.record(retry_record);

                        match retry_result {
                            Ok(asset) => Ok(asset),
                            Err(_) => {
                                // 降级到 BuiltinAssetProvider
                                let builtin_path = asset_base_path.join("builtin-assets");
                                let games_path = asset_base_path.join("games");
                                let builtin = BuiltinAssetProvider::new(builtin_path, games_path, asset_base_path.clone());
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

    /// 资源就绪回调 → 更新 GameScript URL + Tauri 事件通知前端
    async fn on_asset_ready(&self, game_id: &str, asset_ref: &AssetRef, local_asset: &LocalAsset) {
        let asset_type_label = Self::asset_type_label(&asset_ref.asset_type);
        let desc = if asset_ref.prompt.is_empty() {
            format!("{}生成完成", asset_type_label)
        } else {
            let short_desc: String = asset_ref.prompt.chars().take(30).collect();
            format!("{}: {}", asset_type_label, short_desc)
        };
        self.emit_progress(game_id, "asset_ready", &desc, "");

        // 更新 GameScript 中对应 AssetRef 的 url 字段
        {
            let mut scripts = self.scripts.write().await;
            if let Some(script) = scripts.get_mut(game_id) {
                Self::update_script_assets(script, |ar: &mut AssetRef| {
                    if ar.id == asset_ref.id {
                        ar.url = Some(local_asset.local_path.clone());
                        ar.status = AssetStatus::Ready;
                        ar.source = ScriptAssetSource::AiGenerated;
                    }
                });
                // 保存更新后的 GameScript
                let _ = self.asset_manager.save_game_script(game_id, script);
            }
        }

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

    /// 获取所有正在生成中的游戏 ID 列表
    pub async fn get_active_generations_list(&self) -> Vec<String> {
        let statuses = self.statuses.read().await;
        statuses.iter()
            .filter(|(_, s)| s.background_generation_active || !s.first_chapter_ready)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// 启动后续章节的后台生成（高质量模式专用）
    /// 从 generation_contexts 中取出保存的上下文，逐章生成后续章节
    pub async fn start_remaining_chapters(&self, game_id: &str) -> Result<(), ProviderError> {
        // 取出 GenerationContext（移出，避免长期持有锁）
        let ctx = {
            let mut contexts = self.generation_contexts.write().await;
            contexts.remove(game_id).ok_or_else(|| {
                ProviderError::NotFound(format!(
                    "Generation context not found for game '{}'. This may not be a high-quality mode game or chapters are already being generated.",
                    game_id
                ))
            })?
        };

        let config_manager = self.config_manager.clone();
        let asset_manager = self.asset_manager.clone();
        let app_handle = self.app_handle.clone();
        let statuses = self.statuses.clone();
        let call_history = self.call_history.clone();
        let scripts = self.scripts.clone();
        let game_id = game_id.to_string();

        tokio::spawn(async move {
            let game_type_str = ctx.game_type.as_ref().map(|gt| Self::game_type_display_str(gt)).unwrap_or("互动叙事");
            let game_type_prompt = Self::get_game_type_prompt(game_type_str);

            // 获取文本 provider
            let text_provider = {
                let config_manager = config_manager.read().await;
                let config = config_manager.get_config();
                let provider_config = Self::find_provider_for_modality(config, &AIModality::Text).cloned();
                drop(config_manager);

                match provider_config {
                    Some(pc) => DeepSeekProvider::new(&pc, asset_manager.base_path()),
                    None => {
                        log::error!("后续章节生成失败：无可用的文本 AI Provider");
                        return;
                    }
                }
            };

            let text_provider = match text_provider {
                Ok(d) => d,
                Err(e) => {
                    log::error!("后续章节生成失败：创建 Provider 失败: {:?}", e);
                    return;
                }
            };

            let model_name = {
                let cm = config_manager.read().await;
                let config = cm.get_config();
                if let Some(pc) = Self::find_provider_for_modality(config, &AIModality::Text) {
                    let model = pc.models.iter().find(|m| m.is_default).map(|m| m.id.clone()).unwrap_or_default();
                    format!("{}/{}", pc.id, model)
                } else {
                    "未知".to_string()
                }
            };

            let mut messages = ctx.messages.clone();
            let total_chapters = ctx.total_chapters;
            let already_generated = ctx.chapter_details.len(); // 已生成的章节数

            // 逐章生成后续章节
            for ch_idx in (already_generated + 1)..=total_chapters {
                // 检查是否已取消
                if ctx.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                    log::info!("后续章节生成已取消: game_id={}", game_id);
                    if let Some(ref handle) = app_handle {
                        let _ = handle.emit(
                            "generation-step",
                            serde_json::json!({
                                "gameId": game_id,
                                "step": "cancelled",
                                "detail": "后续章节生成已取消",
                                "modelName": "",
                                "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                            }),
                        );
                    }
                    return;
                }

                let label = Self::get_chapter_label(ch_idx);

                // 发送进度事件
                if let Some(ref handle) = app_handle {
                    let _ = handle.emit(
                        "generation-step",
                        serde_json::json!({
                            "gameId": game_id,
                            "step": "generating_chapter",
                            "detail": format!("正在生成{}详细内容（{}/{}）", label, ch_idx, total_chapters),
                            "modelName": model_name,
                            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                        }),
                    );
                }

                // 生成章节详情
                let detail_user = format!(
                    "现在请根据核心要素，生成{}的详细内容。\n\n\
                     核心要素中关于{}的梗概是关键参考，请严格遵循。\
                     同时保持与前面章节的连续性和一致性。\
                     每个场景至少3-5个对话/旁白节点，选择节点至少2-3个选项。\
                     场景描写要细腻，为后续生成资源提供充分的提示词依据。\n\n\
                     {}",
                    label, label, game_type_prompt
                );

                messages.push(crate::providers::deepseek::ChatMessage {
                    role: "user".to_string(),
                    content: detail_user,
                });

                let chapter_detail = match text_provider.chat(messages.clone(), None).await {
                    Ok(d) => OutlineParser::strip_think_tags(&d),
                    Err(e) => {
                        log::error!("第{}章详情生成失败: {:?}", ch_idx, e);
                        // 移除失败的 user message，继续下一章
                        messages.pop();
                        continue;
                    }
                };

                log::info!("第{}章详情生成完成: 长度={}", ch_idx, chapter_detail.len());

                // 保存章节详情
                OutlineParser::save_raw_ai_response_sync(
                    &format!("chapter_{}_detail", ch_idx),
                    &chapter_detail,
                );

                messages.push(crate::providers::deepseek::ChatMessage {
                    role: "assistant".to_string(),
                    content: chapter_detail.clone(),
                });

                // 检查是否已取消
                if ctx.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                    log::info!("后续章节生成已取消: game_id={}", game_id);
                    return;
                }

                // 生成该章节的 GameScript JSON
                if let Some(ref handle) = app_handle {
                    let _ = handle.emit(
                        "generation-step",
                        serde_json::json!({
                            "gameId": game_id,
                            "step": "generating_chapter_script",
                            "detail": format!("正在生成{}游戏脚本", label),
                            "modelName": model_name,
                            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                        }),
                    );
                }

                let script_user = format!(
                    "根据核心要素和{}的详细内容，生成该章节的游戏脚本 JSON。\n\n\
                     == 核心要素 ==\n{}\n\n\
                     == {} 详细内容 ==\n{}\n\n\
                     要求：只生成{}的脚本，包含完整的 scenes 和 sequence。\n\
                     - 严格遵循核心要素中的世界观、角色设定和剧情走向\n\
                     - 每个场景至少3-5个对话/旁白节点\n\
                     - 选择节点至少2-3个选项\n\
                     - 场景描写要细腻，注重氛围\n\
                     - 角色对话要符合其性格\n\
                     - 为所有资源编写详细的生成 prompt\n\
                     {}",
                    label, ctx.core_elements, label, chapter_detail, label, game_type_prompt
                );

                let script_messages = vec![
                    crate::providers::deepseek::ChatMessage {
                        role: "system".to_string(),
                        content: PROMPT_COMBINED.to_string(),
                    },
                    crate::providers::deepseek::ChatMessage {
                        role: "user".to_string(),
                        content: script_user,
                    },
                ];

                let response = match text_provider.chat(script_messages, None).await {
                    Ok(r) => OutlineParser::strip_think_tags(&r),
                    Err(e) => {
                        log::error!("第{}章脚本生成失败: {:?}", ch_idx, e);
                        continue;
                    }
                };

                // 解析该章节的 JSON
                match Self::extract_json_from_response(&response) {
                    Ok(json_str) => {
                        let json_str = OutlineParser::normalize_json(&json_str);
                        match serde_json::from_str::<GameScript>(&json_str) {
                            Ok(mut partial_script) => {
                                Self::validate_and_fix_script(&mut partial_script).ok();

                                if !partial_script.chapters.is_empty() {
                                    // 将新章节追加到现有 GameScript
                                    let new_chapter = partial_script.chapters.remove(0);
                                    let chapter_id = new_chapter.id.clone();
                                    let chapter_title = new_chapter.title.clone();

                                    {
                                        let mut scripts = scripts.write().await;
                                        if let Some(existing_script) = scripts.get_mut(&game_id) {
                                            existing_script.chapters.push(new_chapter);
                                            let _ = asset_manager.save_game_script(&game_id, existing_script);
                                        }
                                    }

                                    // 更新生成状态：添加新章节
                                    {
                                        let mut s = statuses.write().await;
                                        if let Some(status) = s.get_mut(&game_id) {
                                            status.chapter_status.insert(
                                                chapter_id.clone(),
                                                ChapterStatus {
                                                    chapter_id: chapter_id.clone(),
                                                    chapter_title: chapter_title.clone(),
                                                    total_assets: 0,
                                                    completed_assets: 0,
                                                    status: "pending".to_string(),
                                                },
                                            );
                                        }
                                    }

                                    // 发送 chapter-ready 事件
                                    if let Some(ref handle) = app_handle {
                                        let _ = handle.emit(
                                            "chapter-ready",
                                            serde_json::json!({
                                                "gameId": game_id,
                                                "chapterIndex": ch_idx - 1,
                                                "totalChapters": total_chapters,
                                                "chapterId": chapter_id,
                                                "chapterTitle": chapter_title,
                                            }),
                                        );
                                    }

                                    // 收集该章节的 AssetRef 并启动资源生成
                                    let asset_refs = {
                                        let scripts = scripts.read().await;
                                        if let Some(script) = scripts.get(&game_id) {
                                            Self::extract_asset_refs_static(script)
                                                .into_iter()
                                                .filter(|(cid, _)| cid == &chapter_id)
                                                .collect::<Vec<_>>()
                                        } else {
                                            Vec::new()
                                        }
                                    };

                                    if !asset_refs.is_empty() {
                                        // 更新章节状态
                                        {
                                            let mut s = statuses.write().await;
                                            if let Some(status) = s.get_mut(&game_id) {
                                                if let Some(cs) = status.chapter_status.get_mut(&chapter_id) {
                                                    cs.total_assets = asset_refs.len();
                                                    cs.status = "generating".to_string();
                                                }
                                                status.total_assets += asset_refs.len();
                                            }
                                        }

                                        // 生成该章节的资源
                                        let results = Self::fetch_assets_static(
                                            &config_manager,
                                            &asset_manager,
                                            &call_history,
                                            &game_id,
                                            &asset_refs,
                                        ).await;

                                        // 处理结果
                                        let mut chap_completed = 0usize;
                                        let mut chap_failed = 0usize;

                                        for (i, result) in results.into_iter().enumerate() {
                                            let (_, asset_ref) = &asset_refs[i];
                                            match result {
                                                Ok(ref local_asset) => {
                                                    // 更新 GameScript
                                                    {
                                                        let mut scripts = scripts.write().await;
                                                        if let Some(script) = scripts.get_mut(&game_id) {
                                                            Self::update_script_assets(script, |ar: &mut AssetRef| {
                                                                if ar.id == asset_ref.id {
                                                                    ar.url = Some(local_asset.local_path.clone());
                                                                    ar.status = AssetStatus::Ready;
                                                                    ar.source = ScriptAssetSource::AiGenerated;
                                                                }
                                                            });
                                                            let _ = asset_manager.save_game_script(&game_id, script);
                                                        }
                                                    }
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

                                        // 更新章节和整体状态
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
                                            }
                                        }
                                    }

                                    log::info!("第{}章生成完成: game_id={}", ch_idx, game_id);
                                }
                            }
                            Err(e) => {
                                log::warn!("第{}章脚本解析失败: {}, 跳过", ch_idx, e);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("第{}章JSON提取失败: {:?}, 跳过", ch_idx, e);
                    }
                }
            }

            // 所有章节生成完成
            {
                let mut s = statuses.write().await;
                if let Some(status) = s.get_mut(&game_id) {
                    status.background_generation_active = false;
                }
            }

            if let Some(ref handle) = app_handle {
                let _ = handle.emit(
                    "all-chapters-ready",
                    serde_json::json!({
                        "gameId": game_id,
                    }),
                );
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let _ = handle.emit(
                    "generation-step",
                    serde_json::json!({
                        "gameId": game_id,
                        "step": "completed",
                        "detail": "所有章节生成完成",
                        "modelName": "",
                        "timestamp": timestamp,
                    }),
                );
                let _ = handle.emit(
                    "generation-complete",
                    serde_json::json!({ "gameId": game_id, "allChapters": true }),
                );
            }

            log::info!("所有后续章节生成完成: game_id={}", game_id);
        });

        Ok(())
    }

    /// 取消后续章节的后台生成
    pub async fn cancel_remaining_chapters(&self, game_id: &str) -> Result<(), ProviderError> {
        let contexts = self.generation_contexts.read().await;
        if let Some(ctx) = contexts.get(game_id) {
            ctx.cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
            log::info!("已请求取消后续章节生成: game_id={}", game_id);
            Ok(())
        } else {
            Err(ProviderError::NotFound(format!(
                "Generation context not found for game '{}'", game_id
            )))
        }
    }

    /// 静态版本的 extract_asset_refs，用于后台任务
    fn extract_asset_refs_static(script: &GameScript) -> Vec<(String, AssetRef)> {
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

    // --- Helper methods ---

    /// 获取当前配置的 Provider 信息（用于调用历史记录）
    async fn get_provider_info(&self, modality: &AIModality) -> (String, String, String) {
        let cm = self.config_manager.read().await;
        let config = cm.get_config();
        if let Some(pc) = Self::find_provider_for_modality(config, modality) {
            let model = pc.models.iter()
                .find(|m| m.is_default)
                .map(|m| m.id.clone())
                .unwrap_or_default();
            let endpoint = pc.models.iter()
                .find(|m| m.is_default)
                .map(|m| m.endpoint.clone())
                .unwrap_or_default();
            (pc.id.clone(), model, endpoint)
        } else {
            ("builtin".to_string(), "default".to_string(), String::new())
        }
    }

    /// AssetType → AIModality 映射
    fn asset_type_to_modality(asset_type: &ScriptAssetType) -> AIModality {
        match asset_type {
            ScriptAssetType::Image => AIModality::Image,
            ScriptAssetType::Video => AIModality::Video,
            ScriptAssetType::Audio => AIModality::Music,
            ScriptAssetType::Voice => AIModality::Voice,
        }
    }

    /// 资源类型的中文标签
    fn asset_type_label(asset_type: &ScriptAssetType) -> &'static str {
        match asset_type {
            ScriptAssetType::Image => "图片",
            ScriptAssetType::Video => "视频",
            ScriptAssetType::Audio => "音频",
            ScriptAssetType::Voice => "语音",
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
