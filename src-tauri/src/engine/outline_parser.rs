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

        self.text_provider.chat(messages, None).await
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
        let json_str = self.extract_json(&response)?;
        let mut script: GameScript = serde_json::from_str(&json_str).map_err(|e| {
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
        // 先修复 AssetRef status（validator 不处理此逻辑）
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
