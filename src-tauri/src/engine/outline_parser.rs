use crate::providers::deepseek::DeepSeekProvider;
use crate::providers::ProviderError;
use crate::types::game_script::{
    ActionNode, AssetRef, AssetSource, AssetStatus, AssetType, CGNode, Chapter, ChoiceNode,
    ChoiceOption, ConditionNode, DialogueNode, GameScript, GameType, NarrationNode, Scene,
    SceneAssets, SceneNode, SceneTransitionNode, TransitionKind, VariableDef, VariableType,
};
use std::collections::HashSet;
use std::path::PathBuf;

pub struct OutlineParser {
    text_provider: DeepSeekProvider,
    prompts_dir: PathBuf,
}

impl OutlineParser {
    pub fn new(text_provider: DeepSeekProvider, prompts_dir: PathBuf) -> Self {
        Self {
            text_provider,
            prompts_dir,
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
        let system_prompt = self.load_prompt("expand")?;

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

        self.text_provider.chat(messages, None).await
    }

    /// 解析大纲为 GameScript JSON
    async fn parse_to_script(
        &self,
        outline: &str,
        game_type: Option<GameType>,
    ) -> Result<GameScript, ProviderError> {
        let system_prompt = self.load_prompt("parse")?;

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
        let json_str = self.extract_json(&response)?;
        let mut script: GameScript = serde_json::from_str(&json_str).map_err(|e| {
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
        let system_prompt = self.load_prompt("combined")?;

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
        let json_str = self.extract_json(&response)?;
        let mut script: GameScript = serde_json::from_str(&json_str).map_err(|e| {
            ProviderError::GenerationFailed(format!("Failed to parse GameScript JSON: {}", e))
        })?;

        self.validate_and_fix(&mut script)?;
        Ok(script)
    }

    /// 加载 Prompt 模板
    fn load_prompt(&self, name: &str) -> Result<String, ProviderError> {
        let path = self.prompts_dir.join(format!("{}.md", name));
        std::fs::read_to_string(&path).map_err(|e| {
            ProviderError::InvalidConfig(format!(
                "Failed to load prompt template '{}': {}",
                name, e
            ))
        })
    }

    /// 校验并修复 GameScript
    fn validate_and_fix(&self, script: &mut GameScript) -> Result<(), ProviderError> {
        // 收集所有已有 id
        let mut all_ids: HashSet<String> = HashSet::new();
        let mut missing_id_nodes: Vec<(String, String)> = Vec::new(); // (chapter_id, scene_id) for context

        // 第一遍：收集所有已有 id，找出缺失 id 的节点
        for chapter in &script.chapters {
            all_ids.insert(chapter.id.clone());
            for scene in &chapter.scenes {
                all_ids.insert(scene.id.clone());
                for node in &scene.sequence {
                    let node_id = Self::get_node_id(node);
                    if node_id.is_empty() {
                        missing_id_nodes.push((chapter.id.clone(), scene.id.clone()));
                    } else {
                        all_ids.insert(node_id);
                    }
                }
            }
        }

        // 第二遍：为缺失 id 的节点生成 UUID，修复 AssetRef status
        for chapter in &mut script.chapters {
            for scene in &mut chapter.scenes {
                Self::fix_scene_assets(&mut scene.assets);
                for node in &mut scene.sequence {
                    Self::fix_node_id(node);
                    Self::fix_node_assets(node);
                }
            }
        }

        // 第三遍：验证 nextNodeId 引用
        let mut valid_ids: HashSet<String> = HashSet::new();
        for chapter in &script.chapters {
            valid_ids.insert(chapter.id.clone());
            for scene in &chapter.scenes {
                valid_ids.insert(scene.id.clone());
                for node in &scene.sequence {
                    let node_id = Self::get_node_id(node);
                    if !node_id.is_empty() {
                        valid_ids.insert(node_id);
                    }
                }
            }
        }

        // 移除无效的 nextNodeId 引用（设为 None）
        for chapter in &mut script.chapters {
            for scene in &mut chapter.scenes {
                for node in &mut scene.sequence {
                    Self::fix_node_next_refs(node, &valid_ids);
                }
            }
        }

        Ok(())
    }

    /// 从 AI 响应中提取 JSON
    fn extract_json(&self, response: &str) -> Result<String, ProviderError> {
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

    /// 获取节点的 id
    fn get_node_id(node: &SceneNode) -> String {
        match node {
            SceneNode::Narration(n) => n.id.clone(),
            SceneNode::Dialogue(d) => d.id.clone(),
            SceneNode::Choice(c) => c.id.clone(),
            SceneNode::Condition(c) => c.id.clone(),
            SceneNode::Action(a) => a.id.clone(),
            SceneNode::Cg(c) => c.id.clone(),
            SceneNode::SceneTransition(s) => s.id.clone(),
        }
    }

    /// 为缺失 id 的节点生成 UUID
    fn fix_node_id(node: &mut SceneNode) {
        let id = Self::get_node_id(node);
        if id.is_empty() {
            let new_id = uuid::Uuid::new_v4().to_string();
            match node {
                SceneNode::Narration(n) => n.id = new_id,
                SceneNode::Dialogue(d) => d.id = new_id,
                SceneNode::Choice(c) => c.id = new_id,
                SceneNode::Condition(c) => c.id = new_id,
                SceneNode::Action(a) => a.id = new_id,
                SceneNode::Cg(c) => c.id = new_id,
                SceneNode::SceneTransition(s) => s.id = new_id,
            }
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

    /// 修复节点中无效的 nextNodeId 引用
    fn fix_node_next_refs(node: &mut SceneNode, valid_ids: &HashSet<String>) {
        match node {
            SceneNode::Action(a) => {
                if let Some(ref next_id) = a.next_node_id {
                    if !valid_ids.contains(next_id) {
                        a.next_node_id = None;
                    }
                }
            }
            SceneNode::Cg(c) => {
                if let Some(ref next_id) = c.next_node_id {
                    if !valid_ids.contains(next_id) {
                        c.next_node_id = None;
                    }
                }
            }
            SceneNode::Choice(c) => {
                for opt in &mut c.options {
                    if let Some(ref next_id) = opt.next_node_id {
                        if !valid_ids.contains(next_id) {
                            opt.next_node_id = None;
                        }
                    }
                }
            }
            SceneNode::Condition(c) => {
                if !valid_ids.contains(&c.true_branch) {
                    c.true_branch = String::new();
                }
                if !valid_ids.contains(&c.false_branch) {
                    c.false_branch = String::new();
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
