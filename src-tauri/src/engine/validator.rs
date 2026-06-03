use crate::types::game_script::{AssetRef, GameScript, Scene, SceneNode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub fixed_script: Option<GameScript>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub severity: String, // "error", "warning"
    pub category: String, // "dead_link", "missing_id", "unreachable", "missing_field"
    pub message: String,
    pub location: String, // "chapter/{id}/scene/{id}/node/{id}"
    pub auto_fixed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub category: String,
    pub message: String,
    pub location: String,
}

pub struct GameScriptValidator;

impl GameScriptValidator {
    pub fn new() -> Self {
        Self
    }

    /// 校验 GameScript
    pub fn validate(&self, script: &GameScript) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        errors.extend(self.check_required_fields(script));
        errors.extend(self.check_dead_links(script));

        let reachability_issues = self.check_reachability(script);
        for issue in reachability_issues {
            if issue.severity == "warning" {
                warnings.push(ValidationWarning {
                    category: issue.category,
                    message: issue.message,
                    location: issue.location,
                });
            } else {
                errors.push(issue);
            }
        }

        let is_valid = errors.iter().all(|e| e.severity != "error");

        ValidationResult {
            is_valid,
            errors,
            warnings,
            fixed_script: None,
        }
    }

    /// 校验并自动修复
    pub fn validate_and_fix(&self, script: &mut GameScript) -> ValidationResult {
        let first_result = self.validate(script);
        let fixed_errors = self.auto_fix(script, &first_result.errors);

        let second_result = self.validate(script);

        let mut all_errors: Vec<ValidationError> = fixed_errors;
        all_errors.extend(second_result.errors);

        let is_valid = all_errors.iter().all(|e| e.severity != "error");

        ValidationResult {
            is_valid,
            errors: all_errors,
            warnings: second_result.warnings,
            fixed_script: Some(script.clone()),
        }
    }

    /// 检查节点可达性
    fn check_reachability(&self, script: &GameScript) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for chapter in &script.chapters {
            for scene in &chapter.scenes {
                if scene.sequence.is_empty() {
                    continue;
                }

                let node_ids = self.collect_node_ids(scene);
                let mut reachable = HashSet::new();

                // BFS from sequence[0]
                let first_id = get_node_id(&scene.sequence[0]);
                if !first_id.is_empty() {
                    let mut queue = vec![first_id.clone()];
                    reachable.insert(first_id);

                    while let Some(current_id) = queue.pop() {
                        // Find the node and collect its next references
                        if let Some(node) = scene.sequence.iter().find(|n| get_node_id(n) == current_id) {
                            let next_refs = get_node_next_refs(node);
                            for next_id in next_refs {
                                if !next_id.is_empty() && node_ids.contains(&next_id) && !reachable.contains(&next_id) {
                                    reachable.insert(next_id.clone());
                                    queue.push(next_id);
                                }
                            }
                        }
                    }
                }

                // Mark unreachable nodes as warnings
                for node in &scene.sequence {
                    let node_id = get_node_id(node);
                    if !node_id.is_empty() && !reachable.contains(&node_id) {
                        errors.push(ValidationError {
                            severity: "warning".to_string(),
                            category: "unreachable".to_string(),
                            message: format!("节点 '{}' 不可达（从场景起始节点无法到达）", node_id),
                            location: format!(
                                "chapter/{}/scene/{}/node/{}",
                                chapter.id, scene.id, node_id
                            ),
                            auto_fixed: false,
                        });
                    }
                }
            }
        }

        errors
    }

    /// 检查死链（nextNodeId 引用不存在的节点）
    fn check_dead_links(&self, script: &GameScript) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for chapter in &script.chapters {
            for scene in &chapter.scenes {
                let node_ids = self.collect_node_ids(scene);
                let refs_map = self.collect_next_node_refs(scene);

                for (node_id, referenced_by) in &refs_map {
                    if !node_id.is_empty() && !node_ids.contains(node_id) {
                        for ref_by in referenced_by {
                            errors.push(ValidationError {
                                severity: "error".to_string(),
                                category: "dead_link".to_string(),
                                message: format!(
                                    "节点 '{}' 引用了不存在的节点 '{}'",
                                    ref_by, node_id
                                ),
                                location: format!(
                                    "chapter/{}/scene/{}/node/{}",
                                    chapter.id, scene.id, ref_by
                                ),
                                auto_fixed: false,
                            });
                        }
                    }
                }
            }
        }

        errors
    }

    /// 检查必要字段完整性
    fn check_required_fields(&self, script: &GameScript) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check meta title
        if script.meta.title.is_empty() {
            errors.push(ValidationError {
                severity: "error".to_string(),
                category: "missing_field".to_string(),
                message: "GameScript.meta.title 为空".to_string(),
                location: "meta".to_string(),
                auto_fixed: false,
            });
        }

        for chapter in &script.chapters {
            if chapter.id.is_empty() {
                errors.push(ValidationError {
                    severity: "error".to_string(),
                    category: "missing_id".to_string(),
                    message: "Chapter 缺少 id".to_string(),
                    location: "chapter/<missing_id>".to_string(),
                    auto_fixed: false,
                });
            }
            if chapter.title.is_empty() {
                errors.push(ValidationError {
                    severity: "error".to_string(),
                    category: "missing_field".to_string(),
                    message: format!("Chapter '{}' 缺少 title", chapter.id),
                    location: format!("chapter/{}", chapter.id),
                    auto_fixed: false,
                });
            }

            for scene in &chapter.scenes {
                if scene.id.is_empty() {
                    errors.push(ValidationError {
                        severity: "error".to_string(),
                        category: "missing_id".to_string(),
                        message: format!("Chapter '{}' 中 Scene 缺少 id", chapter.id),
                        location: format!("chapter/{}/scene/<missing_id>", chapter.id),
                        auto_fixed: false,
                    });
                }
                if scene.title.is_empty() {
                    errors.push(ValidationError {
                        severity: "error".to_string(),
                        category: "missing_field".to_string(),
                        message: format!("Scene '{}' 缺少 title", scene.id),
                        location: format!("chapter/{}/scene/{}", chapter.id, scene.id),
                        auto_fixed: false,
                    });
                }

                for node in &scene.sequence {
                    let node_id = get_node_id(node);
                    if node_id.is_empty() {
                        errors.push(ValidationError {
                            severity: "error".to_string(),
                            category: "missing_id".to_string(),
                            message: "SceneNode 缺少 id".to_string(),
                            location: format!(
                                "chapter/{}/scene/{}/node/<missing_id>",
                                chapter.id, scene.id
                            ),
                            auto_fixed: false,
                        });
                    }

                    // Check ChoiceNode has options
                    if let SceneNode::Choice(c) = node {
                        if c.options.is_empty() {
                            errors.push(ValidationError {
                                severity: "error".to_string(),
                                category: "missing_field".to_string(),
                                message: format!("ChoiceNode '{}' 缺少 options", node_id),
                                location: format!(
                                    "chapter/{}/scene/{}/node/{}",
                                    chapter.id, scene.id, node_id
                                ),
                                auto_fixed: false,
                            });
                        }
                    }

                    // Check AssetRef has id and type
                    let asset_refs = get_node_asset_refs(node);
                    for asset in asset_refs {
                        if asset.id.is_empty() {
                            errors.push(ValidationError {
                                severity: "error".to_string(),
                                category: "missing_field".to_string(),
                                message: format!(
                                    "节点 '{}' 中的 AssetRef 缺少 id",
                                    node_id
                                ),
                                location: format!(
                                    "chapter/{}/scene/{}/node/{}",
                                    chapter.id, scene.id, node_id
                                ),
                                auto_fixed: false,
                            });
                        }
                    }
                }
            }
        }

        errors
    }

    /// 自动修复问题
    fn auto_fix(&self, script: &mut GameScript, _errors: &[ValidationError]) -> Vec<ValidationError> {
        let mut fixed = Vec::new();

        let id_fixes = self.fix_missing_ids(script);
        fixed.extend(id_fixes);

        let link_fixes = self.fix_dead_links(script);
        fixed.extend(link_fixes);

        // Mark errors that were auto-fixed
        for fixed_error in &mut fixed {
            fixed_error.auto_fixed = true;
        }

        fixed
    }

    /// 为死链添加兜底跳转
    fn fix_dead_links(&self, script: &mut GameScript) -> Vec<ValidationError> {
        let mut fixed = Vec::new();

        for chapter in &mut script.chapters {
            for scene in &mut chapter.scenes {
                let node_ids = self.collect_node_ids(scene);

                // Pre-collect next-in-sequence IDs to avoid borrow conflicts
                let next_in_seq_map: Vec<Option<String>> = scene
                    .sequence
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        if i + 1 < scene.sequence.len() {
                            Some(get_node_id(&scene.sequence[i + 1]))
                        } else {
                            None
                        }
                    })
                    .collect();

                for (i, node) in scene.sequence.iter_mut().enumerate() {
                    let next_in_seq = next_in_seq_map.get(i).cloned().flatten();

                    match node {
                        SceneNode::Action(a) => {
                            if let Some(ref next_id) = a.next_node_id {
                                if !next_id.is_empty() && !node_ids.contains(next_id) {
                                    let old = a.next_node_id.take();
                                    a.next_node_id = next_in_seq;
                                    fixed.push(ValidationError {
                                        severity: "warning".to_string(),
                                        category: "dead_link".to_string(),
                                        message: format!(
                                            "ActionNode '{}' 的 nextNodeId '{}' 不存在，已修复为{:?}",
                                            a.id,
                                            old.unwrap_or_default(),
                                            a.next_node_id
                                        ),
                                        location: format!(
                                            "chapter/{}/scene/{}/node/{}",
                                            chapter.id, scene.id, a.id
                                        ),
                                        auto_fixed: true,
                                    });
                                }
                            }
                        }
                        SceneNode::Cg(c) => {
                            if let Some(ref next_id) = c.next_node_id {
                                if !next_id.is_empty() && !node_ids.contains(next_id) {
                                    let old = c.next_node_id.take();
                                    c.next_node_id = next_in_seq;
                                    fixed.push(ValidationError {
                                        severity: "warning".to_string(),
                                        category: "dead_link".to_string(),
                                        message: format!(
                                            "CGNode '{}' 的 nextNodeId '{}' 不存在，已修复为{:?}",
                                            c.id,
                                            old.unwrap_or_default(),
                                            c.next_node_id
                                        ),
                                        location: format!(
                                            "chapter/{}/scene/{}/node/{}",
                                            chapter.id, scene.id, c.id
                                        ),
                                        auto_fixed: true,
                                    });
                                }
                            }
                        }
                        SceneNode::Choice(c) => {
                            for opt in &mut c.options {
                                if let Some(ref next_id) = opt.next_node_id {
                                    if !next_id.is_empty() && !node_ids.contains(next_id) {
                                        let old = opt.next_node_id.take();
                                        opt.next_node_id = next_in_seq.clone();
                                        fixed.push(ValidationError {
                                            severity: "warning".to_string(),
                                            category: "dead_link".to_string(),
                                            message: format!(
                                                "ChoiceNode '{}' 的选项 nextNodeId '{}' 不存在，已修复为{:?}",
                                                c.id,
                                                old.unwrap_or_default(),
                                                opt.next_node_id
                                            ),
                                            location: format!(
                                                "chapter/{}/scene/{}/node/{}",
                                                chapter.id, scene.id, c.id
                                            ),
                                            auto_fixed: true,
                                        });
                                    }
                                }
                            }
                        }
                        SceneNode::Condition(c) => {
                            if !c.true_branch.is_empty() && !node_ids.contains(&c.true_branch) {
                                let old = c.true_branch.clone();
                                c.true_branch = next_in_seq.clone().unwrap_or_default();
                                fixed.push(ValidationError {
                                    severity: "warning".to_string(),
                                    category: "dead_link".to_string(),
                                    message: format!(
                                        "ConditionNode '{}' 的 trueBranch '{}' 不存在，已修复为'{}'",
                                        c.id, old, c.true_branch
                                    ),
                                    location: format!(
                                        "chapter/{}/scene/{}/node/{}",
                                        chapter.id, scene.id, c.id
                                    ),
                                    auto_fixed: true,
                                });
                            }
                            if !c.false_branch.is_empty() && !node_ids.contains(&c.false_branch) {
                                let old = c.false_branch.clone();
                                c.false_branch = next_in_seq.clone().unwrap_or_default();
                                fixed.push(ValidationError {
                                    severity: "warning".to_string(),
                                    category: "dead_link".to_string(),
                                    message: format!(
                                        "ConditionNode '{}' 的 falseBranch '{}' 不存在，已修复为'{}'",
                                        c.id, old, c.false_branch
                                    ),
                                    location: format!(
                                        "chapter/{}/scene/{}/node/{}",
                                        chapter.id, scene.id, c.id
                                    ),
                                    auto_fixed: true,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        fixed
    }

    /// 为缺失 id 的节点生成 id
    fn fix_missing_ids(&self, script: &mut GameScript) -> Vec<ValidationError> {
        let mut fixed = Vec::new();

        for chapter in &mut script.chapters {
            if chapter.id.is_empty() {
                chapter.id = format!("chapter_{}", uuid::Uuid::new_v4());
                fixed.push(ValidationError {
                    severity: "warning".to_string(),
                    category: "missing_id".to_string(),
                    message: format!("Chapter 缺少 id，已生成为 '{}'", chapter.id),
                    location: format!("chapter/{}", chapter.id),
                    auto_fixed: true,
                });
            }

            for scene in &mut chapter.scenes {
                if scene.id.is_empty() {
                    scene.id = format!("scene_{}", uuid::Uuid::new_v4());
                    fixed.push(ValidationError {
                        severity: "warning".to_string(),
                        category: "missing_id".to_string(),
                        message: format!("Scene 缺少 id，已生成为 '{}'", scene.id),
                        location: format!("chapter/{}/scene/{}", chapter.id, scene.id),
                        auto_fixed: true,
                    });
                }

                for node in &mut scene.sequence {
                    let node_id = get_node_id(node);
                    if node_id.is_empty() {
                        let new_id = uuid::Uuid::new_v4().to_string();
                        set_node_id(node, new_id.clone());
                        fixed.push(ValidationError {
                            severity: "warning".to_string(),
                            category: "missing_id".to_string(),
                            message: format!("SceneNode 缺少 id，已生成为 '{}'", new_id),
                            location: format!(
                                "chapter/{}/scene/{}/node/{}",
                                chapter.id, scene.id, new_id
                            ),
                            auto_fixed: true,
                        });
                    }
                }
            }
        }

        fixed
    }

    /// 收集场景中所有节点 ID
    fn collect_node_ids(&self, scene: &Scene) -> HashSet<String> {
        scene
            .sequence
            .iter()
            .map(|n| get_node_id(n))
            .filter(|id| !id.is_empty())
            .collect()
    }

    /// 收集场景中所有 nextNodeId 引用
    fn collect_next_node_refs(&self, scene: &Scene) -> HashMap<String, Vec<String>> {
        let mut refs_map: HashMap<String, Vec<String>> = HashMap::new();

        for node in &scene.sequence {
            let source_id = get_node_id(node);
            let next_refs = get_node_next_refs(node);

            for next_id in next_refs {
                if !next_id.is_empty() {
                    refs_map
                        .entry(next_id)
                        .or_default()
                        .push(if source_id.is_empty() {
                            "<unknown>".to_string()
                        } else {
                            source_id.clone()
                        });
                }
            }
        }

        refs_map
    }
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

/// 设置节点的 id
fn set_node_id(node: &mut SceneNode, id: String) {
    match node {
        SceneNode::Narration(n) => n.id = id,
        SceneNode::Dialogue(d) => d.id = id,
        SceneNode::Choice(c) => c.id = id,
        SceneNode::Condition(c) => c.id = id,
        SceneNode::Action(a) => a.id = id,
        SceneNode::Cg(c) => c.id = id,
        SceneNode::SceneTransition(s) => s.id = id,
    }
}

/// 获取节点的所有 nextNodeId 引用
fn get_node_next_refs(node: &SceneNode) -> Vec<String> {
    match node {
        SceneNode::Action(a) => a
            .next_node_id
            .as_ref()
            .filter(|id| !id.is_empty())
            .cloned()
            .into_iter()
            .collect(),
        SceneNode::Cg(c) => c
            .next_node_id
            .as_ref()
            .filter(|id| !id.is_empty())
            .cloned()
            .into_iter()
            .collect(),
        SceneNode::Choice(c) => c
            .options
            .iter()
            .filter_map(|opt| opt.next_node_id.as_ref().filter(|id| !id.is_empty()).cloned())
            .collect(),
        SceneNode::Condition(c) => {
            let mut refs = Vec::new();
            if !c.true_branch.is_empty() {
                refs.push(c.true_branch.clone());
            }
            if !c.false_branch.is_empty() {
                refs.push(c.false_branch.clone());
            }
            refs
        }
        _ => Vec::new(),
    }
}

/// 获取节点中的所有 AssetRef
fn get_node_asset_refs(node: &SceneNode) -> Vec<&AssetRef> {
    let mut refs = Vec::new();
    match node {
        SceneNode::Narration(n) => {
            if let Some(ref voice) = n.voice_asset {
                refs.push(voice);
            }
        }
        SceneNode::Dialogue(d) => {
            if let Some(ref avatar) = d.speaker_avatar {
                refs.push(avatar);
            }
            if let Some(ref voice) = d.voice_asset {
                refs.push(voice);
            }
        }
        SceneNode::Cg(c) => {
            refs.push(&c.video_asset);
        }
        _ => {}
    }
    refs
}
