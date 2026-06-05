use crate::engine::validator::GameScriptValidator;
use crate::providers::deepseek::DeepSeekProvider;
use crate::providers::ProviderError;
use crate::types::game_script::{
    AssetRef, AssetStatus, GameScript, GameType, SceneAssets, SceneNode,
};

// 内嵌 Prompt 模板，编译时加载，不依赖运行时文件路径
const PROMPT_COMBINED: &str = include_str!("../../../prompts/outline-parser/combined.md");
const PROMPT_EXPAND: &str = include_str!("../../../prompts/outline-parser/expand.md");
const PROMPT_PARSE: &str = include_str!("../../../prompts/outline-parser/parse.md");

pub struct OutlineParser {
    text_provider: DeepSeekProvider,
}

impl OutlineParser {
    pub fn new(text_provider: DeepSeekProvider) -> Self {
        Self {
            text_provider,
        }
    }

    /// 解析大纲为 GameScript
    /// - 短文本（<50字或无章节结构）：先扩展再解析
    /// - 长文本：直接解析
    pub async fn parse(
        &self,
        input: &str,
        game_type: Option<GameType>,
    ) -> Result<GameScript, ProviderError> {
        if Self::needs_expansion(input) {
            let expanded = self.expand(input, game_type.clone()).await?;
            self.parse_to_script(&expanded, game_type).await
        } else {
            self.combined_parse(input, game_type).await
        }
    }

    /// 判断输入是否需要先扩展
    fn needs_expansion(input: &str) -> bool {
        let char_count = input.chars().count();
        if char_count < 50 {
            return true;
        }
        // 检查是否包含章节标记
        let chapter_patterns = ["第一章", "第二章", "第三章", "第1章", "第2章", "第3章", "Chapter"];
        !chapter_patterns.iter().any(|p| input.contains(p))
    }

    /// 扩展简短输入为完整大纲
    async fn expand(
        &self,
        input: &str,
        game_type: Option<GameType>,
    ) -> Result<String, ProviderError> {
        let system_prompt = Self::load_prompt("expand").to_string();

        let mut user_content = format!("玩家构想：{}", input);
        if let Some(gt) = game_type {
            user_content = format!(
                "{}\n期望的游戏类型：{}",
                user_content,
                Self::game_type_display(&gt)
            );
        }

        let messages = vec![
            crate::providers::deepseek::ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            crate::providers::deepseek::ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ];

        self.text_provider.chat(messages, None).await.map(|r| Self::strip_think_tags(&r))
    }

    /// 解析大纲为 GameScript JSON
    async fn parse_to_script(
        &self,
        outline: &str,
        game_type: Option<GameType>,
    ) -> Result<GameScript, ProviderError> {
        let system_prompt = Self::load_prompt("parse").to_string();

        let mut user_content = format!("请将以下大纲解析为 GameScript JSON：\n\n{}", outline);
        if let Some(gt) = game_type {
            user_content = format!(
                "{}\n\n游戏类型：{}",
                user_content,
                Self::game_type_display(&gt)
            );
        }

        let messages = vec![
            crate::providers::deepseek::ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            crate::providers::deepseek::ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ];

        let response = self.text_provider.chat(messages, None).await?;
        let response = Self::strip_think_tags(&response);
        self.save_raw_ai_response("parse", &response);
        let json_str = self.extract_json(&response).map_err(|e| {
            Self::save_raw_ai_response_sync("parse_error", &response);
            e
        })?;
        let json_str = Self::normalize_json(&json_str);
        let mut script: GameScript = serde_json::from_str(&json_str).map_err(|e| {
            Self::save_raw_ai_response_sync("parse_json_error", &response);
            ProviderError::GenerationFailed(format!("Failed to parse GameScript JSON: {}", e))
        })?;

        self.validate_and_fix(&mut script)?;
        Ok(script)
    }

    /// 合并单次调用（扩展+解析）
    async fn combined_parse(
        &self,
        input: &str,
        game_type: Option<GameType>,
    ) -> Result<GameScript, ProviderError> {
        let system_prompt = Self::load_prompt("combined").to_string();

        let mut user_content = format!("玩家描述：{}", input);
        if let Some(gt) = game_type {
            user_content = format!(
                "{}\n游戏类型：{}",
                user_content,
                Self::game_type_display(&gt)
            );
        }

        let messages = vec![
            crate::providers::deepseek::ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            crate::providers::deepseek::ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ];

        let response = self.text_provider.chat(messages, None).await?;
        let response = Self::strip_think_tags(&response);
        self.save_raw_ai_response("combined", &response);
        let json_str = self.extract_json(&response).map_err(|e| {
            Self::save_raw_ai_response_sync("combined_error", &response);
            e
        })?;
        let json_str = Self::normalize_json(&json_str);
        let mut script: GameScript = serde_json::from_str(&json_str).map_err(|e| {
            Self::save_raw_ai_response_sync("combined_json_error", &response);
            ProviderError::GenerationFailed(format!("Failed to parse GameScript JSON: {}", e))
        })?;

        self.validate_and_fix(&mut script)?;
        Ok(script)
    }

    /// 加载 Prompt 模板（从编译时内嵌的常量中读取）
    fn load_prompt(name: &str) -> &'static str {
        match name {
            "combined" => PROMPT_COMBINED,
            "expand" => PROMPT_EXPAND,
            "parse" => PROMPT_PARSE,
            _ => PROMPT_COMBINED,
        }
    }

    /// 校验并修复 GameScript
    fn validate_and_fix(&self, script: &mut GameScript) -> Result<(), ProviderError> {
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
                    Self::fix_node_id(node, &node_id);
                }
            }
        }

        // 修复 AssetRef status（validator 不处理此逻辑）
        for chapter in &mut script.chapters {
            for scene in &mut chapter.scenes {
                Self::fix_scene_assets(&mut scene.assets);
                for node in &mut scene.sequence {
                    Self::fix_node_assets(node);
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

    /// 修复节点缺失的 id
    fn fix_node_id(node: &mut SceneNode, default_id: &str) {
        match node {
            SceneNode::Narration(n) => { if n.id.is_empty() { n.id = default_id.to_string(); } }
            SceneNode::Dialogue(d) => { if d.id.is_empty() { d.id = default_id.to_string(); } }
            SceneNode::Choice(c) => { if c.id.is_empty() { c.id = default_id.to_string(); } }
            SceneNode::Condition(c) => { if c.id.is_empty() { c.id = default_id.to_string(); } }
            SceneNode::Action(a) => { if a.id.is_empty() { a.id = default_id.to_string(); } }
            SceneNode::Cg(c) => { if c.id.is_empty() { c.id = default_id.to_string(); } }
            SceneNode::SceneTransition(t) => { if t.id.is_empty() { t.id = default_id.to_string(); } }
        }
    }

    /// 保存 AI 原始响应到文件
    pub fn save_raw_ai_response(&self, stage: &str, response: &str) {
        let dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("gen")
            .join("ai-responses");
        std::fs::create_dir_all(&dir).ok();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let path = dir.join(format!("outline_{}_{}.txt", stage, ts));
        std::fs::write(&path, response).ok();
        log::info!("AI 原始响应已保存: {}", path.display());
    }

    pub fn save_raw_ai_response_sync(stage: &str, response: &str) {
        let dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("gen")
            .join("ai-responses");
        std::fs::create_dir_all(&dir).ok();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let path = dir.join(format!("outline_{}_{}.txt", stage, ts));
        std::fs::write(&path, response).ok();
    }

    /// 从 AI 响应中去除 <think>...</think> 标签及其内容
    /// DeepSeek R1 等推理模型会在 content 中返回 <think>推理过程</think>实际回答
    pub fn strip_think_tags(response: &str) -> String {
        let mut result = response.to_string();
        // 循环移除所有 <think>...</think> 块（支持多行内容）
        loop {
            if let Some(start) = result.find("<think>") {
                let think_start = start;
                if let Some(end) = result[think_start..].find("</think>") {
                    let think_end = think_start + end + "</think>".len();
                    result = format!("{}{}", &result[..think_start], &result[think_end..]);
                } else {
                    // 没有闭合标签，移除从 <think> 到末尾的所有内容
                    result = result[..think_start].to_string();
                    break;
                }
            } else {
                break;
            }
        }
        result.trim().to_string()
    }

    /// 从 AI 响应中提取 JSON
    pub fn extract_json(&self, response: &str) -> Result<String, ProviderError> {
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
            // 跳过可能的语言标记行
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
            // 找到匹配的 }
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

    pub fn normalize_json(json_str: &str) -> String {
        let mut value: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => return json_str.to_string(),
        };

        let mut fixed = false;
        Self::normalize_value(&mut value, &mut fixed);

        if fixed {
            log::warn!("AI 返回的 JSON 包含非标准字段，已自动修正");
        }

        serde_json::to_string(&value).unwrap_or_else(|_| json_str.to_string())
    }

    fn normalize_value(value: &mut serde_json::Value, fixed: &mut bool) {
        match value {
            serde_json::Value::Object(map) => {
                // type: 修正 AI 常见的节点类型拼写错误
                if let Some(v) = map.get_mut("type") {
                    if let Some(s) = v.as_str() {
                        if let Some(corrected) = normalize_node_type(s) {
                            *v = serde_json::Value::String(corrected);
                            *fixed = true;
                        }
                    }
                }

                // gameType: camelCase → snake_case（如 "visualNovel" → "visual_novel"）
                if let Some(v) = map.get_mut("gameType") {
                    if let Some(s) = v.as_str() {
                        let normalized = camel_to_snake(s);
                        if normalized != s {
                            *v = serde_json::Value::String(normalized);
                            *fixed = true;
                        }
                    }
                }

                // transitionType: 非法值 → "fade"
                if let Some(v) = map.get_mut("transitionType") {
                    if let Some(s) = v.as_str() {
                        if !["fade", "dissolve", "slide", "instant"].contains(&s) {
                            *v = serde_json::Value::String("fade".to_string());
                            *fixed = true;
                        }
                    }
                }

                // effects: 字符串 → null
                if let Some(v) = map.get_mut("effects") {
                    if v.is_string() {
                        *v = serde_json::Value::Null;
                        *fixed = true;
                    }
                }

                // condition: 字符串 → null
                if let Some(v) = map.get_mut("condition") {
                    if v.is_string() {
                        *v = serde_json::Value::Null;
                        *fixed = true;
                    }
                }

                // 递归处理嵌套值
                for (_, v) in map.iter_mut() {
                    Self::normalize_value(v, fixed);
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr.iter_mut() {
                    Self::normalize_value(v, fixed);
                }
            }
            _ => {}
        }
    }

    /// 修复场景资源中的 AssetRef status
    fn fix_scene_assets(assets: &mut SceneAssets) {
        if let Some(ref mut bg) = assets.background_image {
            Self::fix_asset_ref(bg);
        }
        if let Some(ref mut video) = assets.background_video {
            Self::fix_asset_ref(video);
        }
        if let Some(ref mut bgm) = assets.bgm {
            Self::fix_asset_ref(bgm);
        }
        if let Some(ref mut ambient) = assets.ambient_sound {
            Self::fix_asset_ref(ambient);
        }
        if let Some(ref mut cg) = assets.cg_animation {
            Self::fix_asset_ref(cg);
        }
    }

    /// 修复节点中的 AssetRef
    fn fix_node_assets(node: &mut SceneNode) {
        match node {
            SceneNode::Narration(n) => {
                if let Some(ref mut voice) = n.voice_asset {
                    Self::fix_asset_ref(voice);
                }
            }
            SceneNode::Dialogue(d) => {
                if let Some(ref mut avatar) = d.speaker_avatar {
                    Self::fix_asset_ref(avatar);
                }
                if let Some(ref mut voice) = d.voice_asset {
                    Self::fix_asset_ref(voice);
                }
            }
            SceneNode::Cg(c) => {
                Self::fix_asset_ref(&mut c.video_asset);
            }
            _ => {}
        }
    }

    /// 确保 AssetRef 有正确的 status
    fn fix_asset_ref(asset: &mut AssetRef) {
        if asset.id.is_empty() {
            asset.id = uuid::Uuid::new_v4().to_string();
        }
        // 如果 status 是 Fallback 或 Ready 之外的状态，且没有 url，设为 Pending
        match asset.status {
            AssetStatus::Ready | AssetStatus::Fallback => {
                if asset.url.is_none() && asset.builtin_asset_id.is_none() {
                    asset.status = AssetStatus::Pending;
                }
            }
            _ => {}
        }
    }

    /// 获取游戏类型的显示名称
    fn game_type_display(game_type: &GameType) -> &'static str {
        match game_type {
            GameType::VisualNovel => "视觉小说",
            GameType::Rpg => "RPG",
            GameType::Mystery => "悬疑解谜",
            GameType::Horror => "恐怖生存",
            GameType::Simulation => "模拟经营",
        }
    }
}

/// camelCase 转 snake_case（如 "visualNovel" → "visual_novel"）
fn camel_to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// 修正 AI 常见的节点类型拼写/命名错误，返回修正后的值；若无需修正则返回 None
fn normalize_node_type(s: &str) -> Option<String> {
    // 合法值列表（与 SceneNode 枚举的 snake_case 标签一致）
    const VALID_TYPES: &[&str] = &[
        "narration",
        "dialogue",
        "choice",
        "condition",
        "action",
        "cg",
        "scene_transition",
    ];

    // 已经合法，无需修正
    if VALID_TYPES.contains(&s) {
        return None;
    }

    // camelCase → snake_case（如 "sceneTransition" → "scene_transition"）
    let snake = camel_to_snake(s);
    if VALID_TYPES.contains(&snake.as_str()) {
        return Some(snake);
    }

    // 常见拼写错误映射
    let corrected = match s {
        "dialog" => "dialogue",
        "narrate" | "narrative" => "narration",
        "conditional" => "condition",
        "transition" => "scene_transition",
        "sceneTransition" => "scene_transition",
        _ => return None,
    };

    Some(corrected.to_string())
}
