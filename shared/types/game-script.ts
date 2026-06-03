// GameScript 类型定义 — 引擎运行的核心数据结构
// 对应 GAME_DESIGN.md §3.3

export type GameType = 'visual_novel' | 'rpg' | 'mystery' | 'horror' | 'simulation';

export interface GameMeta {
  title: string;
  gameType: GameType;
  description: string;
  totalChapters: number;
  themes: string[];
  tone: string;
}

export interface GameScript {
  meta: GameMeta;
  chapters: Chapter[];
  globalVariables: VariableDef[];
}

export interface Chapter {
  id: string;
  title: string;
  summary: string;
  scenes: Scene[];
  chapterVariables: VariableDef[];
}

export interface Scene {
  id: string;
  title: string;
  description: string;
  assets: SceneAssets;
  sequence: SceneNode[];
  transitions: Transition[];
}

export interface SceneAssets {
  backgroundImage?: AssetRef;
  backgroundVideo?: AssetRef;
  bgm?: AssetRef;
  ambientSound?: AssetRef;
  cgAnimation?: AssetRef;
}

export interface AssetRef {
  id: string;
  type: 'image' | 'video' | 'audio' | 'voice';
  prompt: string;
  negativePrompt?: string;
  style?: string;
  source: 'ai_generated' | 'builtin' | 'local_file';
  status: 'pending' | 'generating' | 'ready' | 'failed' | 'fallback';
  url?: string;
  builtinAssetId?: string;
  cacheKey?: string;
}

// 场景节点 — 按序列执行的交互单元
export type SceneNode =
  | NarrationNode
  | DialogueNode
  | ChoiceNode
  | ConditionNode
  | ActionNode
  | CGNode
  | SceneTransitionNode;

export interface NarrationNode {
  type: 'narration';
  id: string;
  text: string;
  voicePrompt?: string;
  voiceAsset?: AssetRef;
}

export interface DialogueNode {
  type: 'dialogue';
  id: string;
  speaker: string;
  speakerAvatar?: AssetRef;
  text: string;
  voiceAsset?: AssetRef;
  emotion?: string;
}

export interface ChoiceNode {
  type: 'choice';
  id: string;
  prompt: string;
  options: ChoiceOption[];
}

export interface ConditionNode {
  type: 'condition';
  id: string;
  condition: Condition;
  trueBranch: string;
  falseBranch: string;
}

export interface ActionNode {
  type: 'action';
  id: string;
  actionType: 'set_variable' | 'add_item' | 'remove_item' | 'change_scene' | 'trigger_event';
  params: Record<string, any>;
  nextNodeId?: string;
}

export interface CGNode {
  type: 'cg';
  id: string;
  description: string;
  videoAsset: AssetRef;
  duration?: number;
  skipAllowed: boolean;
  nextNodeId?: string;
}

export interface SceneTransitionNode {
  type: 'scene_transition';
  id: string;
  targetSceneId: string;
  transitionType: 'fade' | 'dissolve' | 'slide' | 'instant';
  duration?: number;
}

export interface ChoiceOption {
  text: string;
  nextNodeId?: string;
  effects?: Effect[];
  condition?: Condition;
}

export interface VariableDef {
  name: string;
  type: 'number' | 'string' | 'boolean';
  defaultValue: any;
  description?: string;
}

export interface Effect {
  type: 'set_variable' | 'add_item' | 'remove_item' | 'modify_stat';
  target: string;
  value: any;
}

export interface Condition {
  type: 'variable_check' | 'item_check' | 'stat_check' | 'composite';
  target: string;
  operator: '==' | '!=' | '>' | '<' | '>=' | '<=' | 'has' | 'not_has';
  value: any;
  and?: Condition[];
  or?: Condition[];
}

export interface Transition {
  fromSceneId: string;
  toSceneId: string;
  type: 'fade' | 'dissolve' | 'slide' | 'instant';
  duration: number;
}
