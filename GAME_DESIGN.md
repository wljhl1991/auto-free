# AI 生成式游戏引擎 — 设计文档

## 1. 项目概述

### 1.1 核心创意
一款"由 AI 驱动"的生成式游戏引擎。玩家输入文字描述的游戏大纲（或由系统随机生成），引擎自动调用国内 AI 模型，生成完整的游戏章节内容，包括：

- **游戏场景**（背景图片 / 视频）
- **交互逻辑**（选项分支、触发条件、状态流转）
- **背景音乐**（BGM）
- **NPC 对话**
- **旁白**
- **CG 动画**（关键剧情的过场视频）

每个生成的章节可串联为多章节游戏，支持多种游戏类型。

### 1.2 设计原则

- **资源本地化**：所有 AI 生成的资源归玩家所有，全部存储在本地，不依赖云服务
- **开箱即用**：仅文本 AI 为刚需，图片/音频/视频等模态提供内置默认资源，无需额外配置即可体验完整游戏流程
- **统一代码路径**：无论配置了 1 个 AI 还是 5 个 AI，底层生成管线走相同代码逻辑，区别仅在于资源来源（AI 生成 vs 内置默认）
- **跨平台低门槛**：支持 Windows / macOS / Linux / Web，安装包小，依赖少
- **渐进增强**：配置的 AI 越多，游戏体验越丰富；但最少只需 1 个文本 AI 即可完整游玩

### 1.3 目标用户
- 想快速体验"自己构思的游戏"的玩家
- 文字冒险 / 视觉小说爱好者
- 独立游戏原型创作者
- AI 游戏生成技术探索者

### 1.4 支持的游戏类型（首期）

| 类型 | 说明 | 生成侧重 |
|------|------|----------|
| 视觉小说 / 文字冒险 | 以对话和选项推进剧情 | 对话、旁白、CG、BGM |
| RPG 冒险 | 角色成长 + 探索 + 战斗 | 场景、NPC、战斗逻辑、装备 |
| 悬疑解谜 | 线索收集 + 推理 | 场景细节、谜题逻辑、氛围音乐 |
| 恐怖生存 | 资源管理 + 逃生 | 恐怖场景、音效、Jump Scare CG |
| 模拟经营 | 建造 / 经营 / 决策 | 场景、系统逻辑、NPC 需求 |

---

## 2. 系统架构

### 2.1 整体架构图

```
┌─────────────────────────────────────────────────────┐
│                  游戏客户端 (Tauri)                   │
│  ┌───────────┐ ┌───────────┐ ┌───────────────────┐ │
│  │ 场景渲染器  │ │ 对话系统   │ │ 交互引擎          │ │
│  │ (图片/视频) │ │ (对话框)   │ │ (选项/分支/状态)  │ │
│  └─────┬─────┘ └─────┬─────┘ └────────┬──────────┘ │
│        └──────────────┼───────────────┘             │
│                       ▼                             │
│              ┌────────────────┐                     │
│              │  游戏状态管理器  │                     │
│              │  (GameState)   │                     │
│              └───────┬────────┘                     │
│                       ▼                             │
│  ┌──────────────────────────────────────────────┐   │
│  │         统一生成管线 (GenerationPipeline)      │   │
│  │                                              │   │
│  │  ┌────────────────────────────────────────┐  │   │
│  │  │  资源解析器：为每个 AssetRef 确定来源     │  │   │
│  │  │  AI已配置 → 调用 AI 生成                 │  │   │
│  │  │  AI未配置 → 使用内置默认资源              │  │   │
│  │  └────────────────────────────────────────┘  │   │
│  │                    │                          │   │
│  │     ┌──────────────┼──────────────┐          │   │
│  │     ▼              ▼              ▼          │   │
│  │  ┌──────┐    ┌──────────┐   ┌──────────┐    │   │
│  │  │文本AI│    │多模态 AI  │   │内置默认   │    │   │
│  │  │(刚需)│    │图片/视频/ │   │资源库     │    │   │
│  │  │      │    │音乐/语音  │   │(随应用分发)│    │   │
│  │  └──┬───┘    └────┬─────┘   └────┬─────┘    │   │
│  │     └──────────────┼──────────────┘          │   │
│  │                    ▼                          │   │
│  │  ┌────────────────────────────────────────┐  │   │
│  │  │     本地资源管理器 (AssetManager)        │  │   │
│  │  │  所有资源统一存储在本地，归玩家所有       │  │   │
│  │  └────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────┐
│              国内 AI 模型服务层（按需配置）            │
│  文本(刚需): DeepSeek / 硅基流动 / 通义千问 / 智谱    │
│  图片(可选): 硅基流动 FLUX / 通义万相 / 即梦          │
│  视频(可选): 可灵 / Vidu / 即梦                      │
│  音乐(可选): 天工音乐 / 网易天音                      │
│  语音(可选): 火山引擎TTS / 讯飞TTS                    │
└─────────────────────────────────────────────────────┘
```

### 2.2 核心模块职责

| 模块 | 职责 |
|------|------|
| **大纲解析器** | 将玩家输入的自由文本大纲解析为结构化的 `GameScript` JSON（依赖文本 AI，刚需） |
| **统一生成管线** | 根据 `GameScript` 为每个资源确定来源：AI 已配置则调用 AI 生成，未配置则使用内置默认资源。**无论哪种来源，管线代码路径完全一致** |
| **内置默认资源库** | 随应用分发的默认图片、BGM、音效、语音，确保仅配置文本 AI 也能完整体验游戏 |
| **本地资源管理器** | 所有资源（AI 生成的 + 内置默认的）统一存储在本地，归玩家所有 |
| **游戏状态管理器** | 运行时管理玩家选择、变量、章节进度 |
| **场景渲染器** | 前端展示场景图片/视频、对话、选项 |
| **交互引擎** | 处理玩家输入，驱动分支逻辑和状态流转 |

---

## 3. 游戏大纲与脚本系统

### 3.1 大纲输入方式

1. **自由输入**：玩家在文本框中输入任意内容 — 可以是一句话、几句话、一段描述、或完整大纲，AI 自动补全和扩展
2. **系统随机生成**：玩家选择游戏类型，系统调用文本 AI 自动生成大纲
3. **模板填充**：提供结构化模板，玩家填写关键信息

**核心理念：输入门槛极低，一句话也能生成游戏。** AI 负责从简短输入中推断、扩展、丰富游戏内容，玩家不需要写完整大纲。

### 3.2 大纲输入示例（从简到详）

AI 需要处理各种粒度的输入，以下是从最简到最详的示例：

**一句话（最简）**：
```
一个侦探在雨夜调查一起密室杀人案
```

**几句话**：
```
一个侦探在雨夜调查一起密室杀人案。
嫌疑人有三个：庄园主人、管家、女仆。
真凶是管家，动机是复仇。
```

**一段描述**：
```
一个侦探在雨夜调查一起密室杀人案。嫌疑人有三个：庄园主人、管家、女仆。
庄园主人表面温和实则控制欲极强，管家沉默寡言手上有旧伤疤，女仆看似无辜却知道很多秘密。
调查过程中侦探发现密室、遗书和一段被掩盖的往事。真凶是管家，动机是二十年前的一场火灾。
结局有两个：侦探成功破案离开庄园，或被管家困在密室中。
```

**完整大纲（最详）**：
```
游戏名称：迷雾庄园
类型：悬疑解谜
章节：3章

第一章：抵达
- 主角收到一封神秘邀请函，前往偏远庄园
- 庄园管家在门口迎接，态度冷淡
- 晚宴上，其他客人各怀心事
- 关键线索：管家手上的旧伤疤
- 结尾：深夜听到楼上传来脚步声

第二章：失踪
- 早晨发现一位客人失踪
- 搜索庄园，发现密室
- 关键抉择：是否独自进入密室
- 结尾：密室中发现一封遗书，指向庄园主人

第三章：真相
- 庄园主人坦白一切
- 最终对决：在暴风雨中逃离庄园
- 结局分支：成功逃脱 / 被困庄园
```

**AI 扩展策略**：

| 输入粒度 | AI 处理方式 |
|----------|------------|
| 一句话 | AI 推断游戏类型、主题、氛围 → 生成游戏名称 → 扩展为 3 章大纲 → 再解析为 GameScript |
| 几句话 | AI 提取关键角色和情节 → 补充章节结构 → 解析为 GameScript |
| 一段描述 | AI 拆分章节和场景 → 补充交互细节 → 解析为 GameScript |
| 完整大纲 | AI 直接解析为 GameScript，保留玩家设计的结构 |

> 无论输入多简短，AI 都会生成完整的 GameScript（包含场景、对话、选项、CG 等）。区别仅在于：输入越详细，AI 生成的内容越贴合玩家意图；输入越简短，AI 自主发挥的空间越大。

### 3.3 GameScript 结构定义

大纲经 AI 解析后生成结构化的 `GameScript`，这是引擎运行的核心数据结构：

```typescript
interface GameScript {
  meta: GameMeta;
  chapters: Chapter[];
  globalVariables: VariableDef[];
}

interface GameMeta {
  title: string;
  gameType: GameType;       // 'visual_novel' | 'rpg' | 'mystery' | 'horror' | 'simulation'
  description: string;
  totalChapters: number;
  themes: string[];          // ['悬疑', '哥特', '维多利亚时代']
  tone: string;              // 'dark' | 'lighthearted' | 'serious' | 'humorous'
}

interface Chapter {
  id: string;
  title: string;
  summary: string;
  scenes: Scene[];
  chapterVariables: VariableDef[];
}

interface Scene {
  id: string;
  title: string;
  description: string;       // 场景描述，用于生成图片/视频

  // 生成资源引用
  assets: SceneAssets;

  // 交互序列
  sequence: SceneNode[];

  // 转场
  transitions: Transition[];
}

interface SceneAssets {
  backgroundImage?: AssetRef;   // 背景图
  backgroundVideo?: AssetRef;   // 背景视频（CG）
  bgm?: AssetRef;               // 背景音乐
  ambientSound?: AssetRef;      // 环境音效
  cgAnimation?: AssetRef;       // CG 动画（关键剧情过场）
}

interface AssetRef {
  id: string;
  type: 'image' | 'video' | 'audio' | 'voice';
  prompt: string;              // 生成提示词
  negativePrompt?: string;     // 反向提示词
  style?: string;              // 风格参考
  source: 'ai_generated' | 'builtin' | 'local_file';  // 资源来源
  status: 'pending' | 'generating' | 'ready' | 'failed' | 'fallback';
  url?: string;                // 生成后的本地资源路径
  builtinAssetId?: string;     // 内置默认资源 ID（source=builtin 时）
  cacheKey?: string;           // 缓存键
}

// 场景节点 — 按序列执行的交互单元
type SceneNode =
  | NarrationNode
  | DialogueNode
  | ChoiceNode
  | ConditionNode
  | ActionNode
  | CGNode
  | SceneTransitionNode;

interface NarrationNode {
  type: 'narration';
  id: string;
  text: string;               // 旁白文本
  voicePrompt?: string;       // 旁白语音生成提示
  voiceAsset?: AssetRef;      // 旁白语音资源
}

interface DialogueNode {
  type: 'dialogue';
  id: string;
  speaker: string;            // NPC 名称
  speakerAvatar?: AssetRef;   // NPC 头像
  text: string;               // 对话内容
  voiceAsset?: AssetRef;      // 对话语音
  emotion?: string;           // 情感标注：happy, sad, angry, fear, neutral
}

interface ChoiceNode {
  type: 'choice';
  id: string;
  prompt: string;             // 选项提示文本
  options: ChoiceOption[];
}

interface ChoiceOption {
  text: string;               // 选项显示文本
  nextNodeId?: string;        // 跳转到的节点 ID
  effects?: Effect[];         // 选择后的效果
  condition?: Condition;      // 显示条件（可选）
}

interface ConditionNode {
  type: 'condition';
  id: string;
  condition: Condition;
  trueBranch: string;         // 条件为真时跳转的节点 ID
  falseBranch: string;        // 条件为假时跳转的节点 ID
}

interface ActionNode {
  type: 'action';
  id: string;
  actionType: 'set_variable' | 'add_item' | 'remove_item' | 'change_scene' | 'trigger_event';
  params: Record<string, any>;
  nextNodeId?: string;
}

interface CGNode {
  type: 'cg';
  id: string;
  description: string;        // CG 场景描述
  videoAsset: AssetRef;       // CG 视频资源
  duration?: number;          // 播放时长（秒）
  skipAllowed: boolean;       // 是否允许跳过
  nextNodeId?: string;
}

interface SceneTransitionNode {
  type: 'scene_transition';
  id: string;
  targetSceneId: string;
  transitionType: 'fade' | 'dissolve' | 'slide' | 'instant';
  duration?: number;
}

// 辅助类型
interface VariableDef {
  name: string;
  type: 'number' | 'string' | 'boolean';
  defaultValue: any;
  description?: string;
}

interface Effect {
  type: 'set_variable' | 'add_item' | 'remove_item' | 'modify_stat';
  target: string;
  value: any;
}

interface Condition {
  type: 'variable_check' | 'item_check' | 'stat_check' | 'composite';
  target: string;
  operator: '==' | '!=' | '>' | '<' | '>=' | '<=' | 'has' | 'not_has';
  value: any;
  and?: Condition[];
  or?: Condition[];
}

interface Transition {
  fromSceneId: string;
  toSceneId: string;
  type: 'fade' | 'dissolve' | 'slide' | 'instant';
  duration: number;
}

type GameType = 'visual_novel' | 'rpg' | 'mystery' | 'horror' | 'simulation';
```

---

## 4. AI 生成管线

### 4.1 生成流程

```
玩家输入大纲
     │
     ▼
[Step 1] 大纲解析 ──→ 文本AI生成 GameScript JSON（刚需，必须有文本AI）
     │
     ▼
[Step 2] 资源解析 ──→ 从 GameScript 中提取所有 AssetRef
     │
     ▼
[Step 3] 来源判定 ──→ 为每个 AssetRef 确定资源来源
     │  ┌──────────────────────────────────────────┐
     │  │  该模态 AI 是否已配置？                    │
     │  │  ├─ 是 → source='ai_generated'，调用 AI   │
     │  │  └─ 否 → source='builtin'，使用内置默认   │
     │  └──────────────────────────────────────────┘
     │
     ▼
[Step 4] 并行获取 ──→ AI 生成任务 + 内置资源复制，统一走 AssetManager
     │  ┌─────────────────┐  ┌─────────────────┐
     │  │ AI 生成队列       │  │ 内置资源映射      │
     │  │ 图片 → 硅基流动   │  │ 默认场景图       │
     │  │ 视频 → 可灵      │  │ 默认 BGM        │
     │  │ 音乐 → 天工音乐   │  │ 默认音效        │
     │  │ 语音 → 火山引擎   │  │ 默认 NPC 头像   │
     │  └─────────────────┘  └─────────────────┘
     │
     ▼
[Step 5] 资源落盘 ──→ 所有资源统一存入本地 AssetManager
     │
     ▼
[Step 6] 预览就绪 ──→ 前端加载 GameScript + 本地资源，开始游戏
```

**关键设计：统一代码路径**

无论玩家配置了多少 AI 服务，生成管线的代码路径完全一致：

```typescript
// 统一的资源获取接口 — 无论来源，调用方式相同
interface IAssetProvider {
  getAsset(assetRef: AssetRef): Promise<LocalAsset>;
}

// AI 生成实现
class AIAssetProvider implements IAssetProvider {
  async getAsset(assetRef: AssetRef): Promise<LocalAsset> {
    // 调用配置的 AI 模型生成资源 → 保存到本地 → 返回本地路径
  }
}

// 内置默认资源实现
class BuiltinAssetProvider implements IAssetProvider {
  async getAsset(assetRef: AssetRef): Promise<LocalAsset> {
    // 根据 assetRef.type 和场景特征选择最匹配的内置资源 → 复制到本地资源目录 → 返回本地路径
  }
}

// 管线调度器 — 根据配置选择 Provider，但调用接口统一
class GenerationPipeline {
  resolveProvider(modality: AIModality): IAssetProvider {
    const config = this.configManager.getProviderForModality(modality);
    if (config && config.status === 'connected') {
      return new AIAssetProvider(config);
    }
    return new BuiltinAssetProvider();  // 降级到内置资源，代码路径不变
  }
}
```

**不同配置下的体验对比**：

| 配置 | 场景图片 | BGM | NPC 语音 | CG 视频 | 交互逻辑 |
|------|----------|-----|----------|---------|----------|
| 零成本（星火Lite+EdgeTTS） | 内置默认场景图 | 内置默认 BGM | Edge TTS 免费语音 | 静态图+文字 | 星火 Lite 生成 |
| 仅文本 AI（+EdgeTTS） | 内置默认场景图 | 内置默认 BGM | Edge TTS 免费语音 | 静态图+文字 | DeepSeek 生成 |
| +图片 AI | AI 生成场景图 | 内置默认 BGM | Edge TTS 免费语音 | 静态图+文字 | DeepSeek 生成 |
| +音乐 AI | AI 生成场景图 | AI 生成 BGM | Edge TTS 免费语音 | 静态图+文字 | DeepSeek 生成 |
| +视频 AI | AI 生成场景图 | AI 生成 BGM | Edge TTS 免费语音 | AI 生成 CG | DeepSeek 生成 |

> 核心要点：**交互逻辑始终由文本 AI 生成，这是游戏可玩性的根基**。其他模态只影响视听体验，不影响游戏逻辑完整性。

### 4.2 各 AI 模型调用方案

#### 4.2.0 模型选型总览与默认推荐

**设计原则**：默认推荐方案尽量减少玩家需要注册的厂商数量，优先选择免费额度充足、功能齐全的服务。特别注意标注**完全免费**的选项。

| 模态 | 默认推荐（开发验证） | 厂商 | 免费额度 | 备选 |
|------|---------------------|------|----------|------|
| 文本 | DeepSeek-V3.2 | DeepSeek | 注册送 100 万 tokens/月 | 讯飞星火Lite(永久免费)、通义千问Qwen3.6、硅基流动 |
| 图片 | 硅基流动 FLUX | SiliconFlow | 注册送大量免费 tokens | 通义万相、可灵3.0、即梦 |
| 视频 | 可灵 3.0 | 快手 | 每日免费 6 条 | 海螺AI(MiniMax)、即梦、通义万相 |
| 音乐 | 天工音乐 | 昆仑万维 | 部分免费 | 网易天音 |
| 语音 | Edge TTS | 微软 | **完全免费，无需注册** | 讯飞星火TTS(永久免费)、火山引擎TTS |

**零成本方案（0 家厂商注册）**：

| 模态 | 服务 | 说明 |
|------|------|------|
| 文本 | 讯飞星火 Lite | **永久免费，无限 Tokens**，QPS 限制 2 次/秒 |
| 语音 | Edge TTS | **完全免费，无需 API Key，无需注册**，支持 40+ 语言 |

> 零成本方案无需注册任何厂商账号，仅靠讯飞星火 Lite + Edge TTS 即可体验完整游戏流程（交互逻辑+语音）。图片/音乐/视频使用内置默认资源。

**默认推荐方案仅需注册 4 家厂商**：DeepSeek + 硅基流动 + 快手(可灵) + 昆仑万维(天工)。语音使用 Edge TTS 无需注册。

**极简方案（2 家厂商覆盖全部模态）**：

| 厂商 | 覆盖模态 | 说明 |
|------|----------|------|
| 硅基流动 (SiliconFlow) | 文本 + 图片 | 托管 DeepSeek/Qwen 文本模型 + FLUX 图片模型，1 个账号覆盖 2 个模态 |
| 可灵 (Kling) | 视频 | 快手旗下，每日免费额度 |
| Edge TTS | 语音 | 无需注册，完全免费 |
| 天工音乐 | 音乐 | 昆仑万维旗下 |

#### 4.2.1 文本生成（大纲解析 + 对话 + 旁白 + 交互逻辑）

| 用途 | 默认推荐 | 备选模型 | 免费额度 |
|------|----------|----------|----------|
| 大纲 → GameScript 解析 | DeepSeek-V3.2 ⭐ | 通义千问 Qwen3.6 | DeepSeek: 100 万 tokens/月 |
| NPC 对话生成 | DeepSeek-V3.2 ⭐ | 智谱 GLM-4 | 硅基流动: 大量免费 tokens |
| 旁白文本生成 | DeepSeek-V3.2 ⭐ | 通义千问 Qwen3.6 | 通义千问: 免费额度 |
| 交互逻辑/分支生成 | DeepSeek-V3.2 ⭐ | 智谱 GLM-4 | 智谱 GLM: 新用户赠额度 |
| 随机大纲生成 | DeepSeek-V3.2 ⭐ | 通义千问 Qwen3.6 | — |

> ⭐ = 默认推荐配置。DeepSeek API 兼容 OpenAI 格式，接入简单，价格极低（V3.2 日常使用，R1 推理，V4 代码），为首推选择。
>
> **完全免费替代**：讯飞星火 Lite — **永久免费，无限 Tokens**，QPS 限制 2 次/秒。质量略低于 DeepSeek，但足以完成大纲解析和对话生成，是零成本方案的首选。

**调用方式**：REST API（兼容 OpenAI 格式），流式输出

**Prompt 设计原则**：
- 大纲解析使用 System Prompt 约束输出为合法 GameScript JSON
- 对话生成注入角色设定、情感标注、当前场景上下文
- 旁白生成注入场景描述、氛围关键词、叙事风格

#### 4.2.2 图片生成（场景背景、NPC 头像、CG 静帧）

| 用途 | 默认推荐 | 备选模型 | 免费额度 |
|------|----------|----------|----------|
| 场景背景图 | 硅基流动 FLUX.1-schnell ⭐ | 通义万相 | 硅基流动: 大量免费 tokens |
| NPC 头像 | 硅基流动 FLUX.1-schnell ⭐ | 可灵 3.0 Image | 通义万相: 每日免费额度 |
| CG 静帧 | 硅基流动 FLUX.1-dev ⭐ | 即梦 | 即梦: 每日免费额度 |

> ⭐ = 默认推荐配置。硅基流动托管 FLUX 系列模型，免费额度大，API 兼容 Stable Diffusion 格式。
> FLUX.1-schnell 速度快适合背景/头像；FLUX.1-dev 质量高适合 CG 静帧（消耗更多额度）。
>
> **其他可选**：可灵 3.0 现已支持图片生成（原生 4K 超高分辨率），适合对画质要求极高的场景。

**Prompt 构造规则**：
- 场景图：`{场景描述}, {游戏类型风格}, {色调}, high quality, detailed, game background`
- NPC 头像：`{角色外貌描述}, portrait, {情感}, game character, detailed face`
- 统一添加风格前缀，确保同一章节内视觉一致性

**风格一致性策略**：
- 每个章节生成时，提取风格关键词（如"水墨风"、"赛博朋克"、"日系动漫"）
- 所有图片 Prompt 携带相同的风格前缀
- 可选：生成一张"风格参考图"，后续图片生成时作为参考输入

#### 4.2.3 视频生成（CG 动画、过场）

| 用途 | 默认推荐 | 备选模型 | 免费额度 |
|------|----------|----------|----------|
| CG 过场动画 | 可灵 3.0 ⭐ | 海螺AI (MiniMax) | 可灵: 每日免费 6 条 |
| 场景动态背景 | 可灵 3.0 ⭐ | 即梦 | 海螺AI: 免费额度较大 |

> ⭐ = 默认推荐配置。可灵 3.0 是目前国内视频生成的"全能王"，支持 4K 60 帧、智能分镜、原生音频，动作自然物理稳定。
>
> **其他可选**：
> - **海螺AI (MiniMax Hailuo 02)**：叙事感强，首尾帧控制技术出色，适合有故事情节的 CG，免费额度较大
> - **即梦 (ByteDance)**：图片+视频生成，每日免费额度
> - **通义万相 (阿里)**：视频生成每日免费 10 次
> - **豆包视频 (ByteDance)**：字节跳动旗下，与即梦同生态

**Prompt 构造规则**：
- CG 动画：`{场景描述}, cinematic, {镜头运动}, {氛围}, 4s-6s`
- 场景动态背景：`{场景描述}, slow pan, ambient, looping, 4s`

**限制与策略**：
- 视频生成耗时较长（30s-5min），采用异步队列
- 优先级：CG 动画 > 场景动态背景
- 降级策略：视频生成失败时，用静态图片 + Ken Burns 效果替代

#### 4.2.4 音乐生成（BGM、环境音）

| 用途 | 默认推荐 | 备选模型 | 免费额度 |
|------|----------|----------|----------|
| 章节 BGM | 天工音乐 (SkyMusic) ⭐ | 网易天音 | 天工: 每日免费生成额度 |
| 场景氛围音乐 | 天工音乐 (SkyMusic) ⭐ | 网易天音 | 网易天音: 试用额度 |
| 特殊音效 | 文本AI生成音效描述 + 音乐AI | — | — |

> ⭐ = 默认推荐配置。天工音乐是国内最早开放的 AI 音乐生成平台，免费额度充足。

**Prompt 构造规则**：
- BGM：`{游戏类型} background music, {情感/氛围}, {节奏}, {乐器偏好}, loopable`
- 示例：`悬疑解谜 background music, mysterious, slow tempo, piano and strings, dark atmosphere, loopable`

**音乐一致性策略**：
- 每个章节有主题旋律关键词
- 同一章节内的 BGM 共享乐器和调性描述
- 章节间可有风格变化，但保持整体统一

#### 4.2.5 语音生成（NPC 对话语音、旁白语音）

| 用途 | 默认推荐 | 备选模型 | 免费额度 |
|------|----------|----------|----------|
| NPC 对话语音 | Edge TTS ⭐ | 讯飞星火 TTS | Edge TTS: **完全免费，无需注册** |
| 旁白语音 | Edge TTS ⭐ | 火山引擎 TTS | 讯飞星火: 永久免费（Lite） |

> ⭐ = 默认推荐配置。Edge TTS 是微软 Edge 浏览器内置的语音合成引擎，**完全免费、无需 API Key、无需注册**，支持 40+ 语言包含中文多种方言，音色自然。
> 这是降低门槛的关键选择 — 语音模态零成本、零配置。
>
> **其他可选**：
> - **讯飞星火 TTS**：与星火 Lite 同生态，永久免费，中文音色自然，支持多情感调节
> - **火山引擎 TTS**：音色最丰富，支持情感控制，新用户试用额度
> - **Azure TTS**：微软官方 API 版，50 万字/月免费额度

**语音角色分配**：
- 每个 NPC 分配唯一的 voice_id
- 旁白使用固定的叙述者 voice_id
- 根据角色性别、年龄、性格选择音色

### 4.3 生成调度策略

```typescript
interface GenerationTask {
  id: string;
  assetRef: AssetRef;
  modelProvider: string;       // 'deepseek' | 'tongyi' | 'kling' | ...
  modelEndpoint: string;
  priority: number;            // 1-10, 10 最高
  dependencies: string[];      // 依赖的其他任务 ID
  retryCount: number;
  maxRetries: number;
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  createdAt: number;
  startedAt?: number;
  completedAt?: number;
  result?: any;
  error?: string;
}
```

**调度规则**：
1. 文本生成（GameScript 解析）优先级最高，必须先完成
2. 场景背景图 > NPC 头像 > BGM > 语音 > CG 视频
3. 无依赖关系的任务并行执行
4. 视频任务因耗时长，采用流式就绪通知：首帧可用即可预览
5. 失败任务自动重试（最多 3 次），重试时切换备选模型

### 4.4 渐进式加载策略

为减少玩家等待时间，采用"边生成边游玩"策略：

1. **第一章优先生成**：只等第一章资源就绪即可开始游戏
2. **后台预生成**：玩家游玩第一章时，后台并行生成后续章节
3. **内置默认兜底**：
   - 图片 AI 未配置/未就绪 → 使用内置默认场景图（按游戏类型/氛围匹配）
   - BGM AI 未配置/未就绪 → 使用内置默认 BGM（按游戏类型匹配）
   - 语音 AI 未配置/未就绪 → 仅文字展示
   - CG 视频 AI 未配置/未就绪 → 显示静态 CG 图 + 旁白文字
4. **实时替换**：AI 资源生成完成后，热替换内置默认内容（玩家可随时重新生成）
5. **本地持久化**：所有资源（AI 生成的和内置默认的）统一存储在本地，离线可玩

---

## 5. AI 服务配置管理

### 5.1 设计目标

- **仅文本 AI 为刚需**：图片/音频/视频等模态提供内置默认资源，不配置也能完整体验游戏
- 玩家可自行配置各 AI 服务的 API Key、账号密码等凭证信息
- 支持多套配置方案切换（如"仅文本"、"默认推荐"、"全量配置"）
- 敏感信息安全存储（本地加密）
- 实时检测各服务连通性和余额状态
- **统一代码路径**：无论配置多少 AI，生成管线逻辑完全一致，区别仅在 `IAssetProvider` 的实现选择

### 5.2 配置数据结构

```typescript
// AI 服务提供商配置
interface AIProviderConfig {
  id: string;                        // 唯一标识，如 'deepseek', 'siliconflow'
  name: string;                      // 显示名称
  vendor: string;                    // 厂商名称
  description: string;               // 服务描述
  officialUrl: string;               // 官网地址
  registerUrl: string;               // 注册地址
  docsUrl: string;                   // API 文档地址
  modality: AIModality[];            // 支持的模态
  authType: 'api_key' | 'oauth' | 'account';  // 认证方式
  authConfig: AuthConfig;            // 认证配置
  models: AIModelConfig[];           // 可用模型列表
  status: 'unconfigured' | 'configured' | 'connected' | 'error';
  lastChecked?: number;              // 上次连通检测时间
  errorMessage?: string;             // 错误信息
}

type AIModality = 'text' | 'image' | 'video' | 'music' | 'voice';

// 认证配置
interface AuthConfig {
  // API Key 方式（最常见）
  apiKey?: {
    value: string;                   // 加密存储的 API Key
    label: string;                   // 显示标签，如 "API Key"
    placeholder: string;             // 输入提示
    helpUrl: string;                 // 获取 Key 的帮助链接
  };
  // 账号密码方式
  account?: {
    username?: { value: string; label: string; placeholder: string };
    password?: { value: string; label: string; placeholder: string };
  };
  // OAuth 方式
  oauth?: {
    clientId: string;
    redirectUri: string;
    accessToken?: string;
    refreshToken?: string;
    expiresAt?: number;
  };
  // 额外参数
  extraParams?: Record<string, {
    value: string;
    label: string;
    placeholder: string;
    required: boolean;
    secret: boolean;                 // 是否加密存储
  }>;
}

// AI 模型配置
interface AIModelConfig {
  id: string;                        // 模型标识
  name: string;                      // 显示名称
  modality: AIModality;
  isDefault: boolean;                // 是否为该模态的默认模型
  endpoint: string;                  // API 端点
  maxTokens?: number;                // 最大 token 数（文本模型）
  supportedSizes?: string[];         // 支持的尺寸（图片/视频模型）
  maxDuration?: number;              // 最大时长秒数（音视频模型）
  costPerCall?: number;              // 每次调用消耗额度
  freeQuota?: string;                // 免费额度描述
  quality: 'fast' | 'standard' | 'high';  // 质量/速度分级
}

// 配置预设方案
interface ConfigPreset {
  id: string;
  name: string;
  description: string;
  vendorCount: number;               // 需要注册的厂商数量
  providers: PresetProvider[];        // 预设中的服务商配置
  builtinFallback: {                  // 未配置 AI 的模态使用内置默认资源
    image: boolean;
    video: boolean;
    music: boolean;
    voice: boolean;
  };
}

interface PresetProvider {
  providerId: string;                // 对应 AIProviderConfig.id
  modality: AIModality;              // 该服务商在此预设中负责的模态
  modelId: string;                   // 默认使用的模型
  note?: string;                     // 备注说明
}

// 全局配置
interface AppConfig {
  activePresetId: string;            // 当前激活的预设方案
  providers: AIProviderConfig[];     // 所有服务商配置
  presets: ConfigPreset[];           // 预设方案列表
  globalSettings: {
    autoRetryOnFail: boolean;        // 失败自动重试
    fallbackToAlternative: boolean;  // 失败时回退到备选模型
    maxConcurrentGenerations: number;// 最大并行生成数
    defaultQuality: 'fast' | 'standard' | 'high';
    language: string;                // Prompt 语言偏好
  };
}
```

### 5.3 内置配置预设

#### 预设零：零成本（0 家厂商，完全免费）

```typescript
const ZERO_COST_PRESET: ConfigPreset = {
  id: 'zero-cost',
  name: '零成本',
  description: '完全免费方案，无需注册任何厂商账号。文本用讯飞星火Lite（永久免费），语音用Edge TTS（免费），其他用内置默认',
  vendorCount: 0,
  providers: [
    {
      providerId: 'xfyun-spark-lite',
      modality: 'text',
      modelId: 'spark-lite',
      note: '永久免费，无限 Tokens，QPS 限制 2 次/秒'
    },
    {
      providerId: 'edge-tts',
      modality: 'voice',
      modelId: 'edge-tts-zh',
      note: '完全免费，无需 API Key，无需注册'
    }
  ],
  builtinFallback: {
    image: true,       // 使用内置默认场景图
    video: true,       // 使用静态图+文字替代
    music: true,       // 使用内置默认 BGM
    voice: false       // Edge TTS 已覆盖
  }
};
```

> **这是最低门槛方案**。无需注册任何账号，无需填写任何 API Key，安装即可游玩。讯飞星火 Lite 永久免费（QPS 2次/秒足够游戏使用），Edge TTS 完全免费。图片/音乐/视频使用内置默认资源。

#### 预设一：仅文本（1 家厂商，推荐入门）

```typescript
const TEXT_ONLY_PRESET: ConfigPreset = {
  id: 'text-only',
  name: '仅文本 AI',
  description: '只需 1 个文本 AI 即可完整游玩。语音用 Edge TTS（免费），图片/音乐使用内置默认资源',
  vendorCount: 1,
  providers: [
    {
      providerId: 'deepseek',
      modality: 'text',
      modelId: 'deepseek-v3.2',
      note: '注册送 100 万 tokens/月，兼容 OpenAI 格式'
    },
    {
      providerId: 'edge-tts',
      modality: 'voice',
      modelId: 'edge-tts-zh',
      note: '完全免费，无需注册'
    }
  ],
  builtinFallback: {
    image: true,       // 使用内置默认场景图
    video: true,       // 使用静态图+文字替代
    music: true,       // 使用内置默认 BGM
    voice: false       // Edge TTS 已覆盖
  }
};
```

> **这是推荐的入门方案**。玩家只需注册 DeepSeek 一个账号，即可体验完整的游戏流程。交互逻辑、对话、旁白、分支选择全部由 DeepSeek 生成（质量高于星火 Lite），语音由 Edge TTS 免费提供，图片和音乐使用内置默认资源。

#### 预设二：默认推荐（5 家厂商，功能最全）

```typescript
const DEFAULT_PRESET: ConfigPreset = {
  id: 'default',
  name: '默认推荐',
  description: '功能最全的推荐方案，覆盖全部 5 个模态，免费额度充足',
  vendorCount: 4,                     // Edge TTS 无需注册
  providers: [
    {
      providerId: 'deepseek',
      modality: 'text',
      modelId: 'deepseek-v3.2',
      note: '注册送 100 万 tokens/月，兼容 OpenAI 格式'
    },
    {
      providerId: 'siliconflow',
      modality: 'image',
      modelId: 'flux-1-schnell',
      note: '注册送大量免费 tokens，FLUX 系列模型'
    },
    {
      providerId: 'kling',
      modality: 'video',
      modelId: 'kling-3.0',
      note: '快手旗下，每日免费 6 条，4K 60 帧'
    },
    {
      providerId: 'skymusic',
      modality: 'music',
      modelId: 'skymusic-v1',
      note: '昆仑万维旗下，部分免费'
    },
    {
      providerId: 'edge-tts',
      modality: 'voice',
      modelId: 'edge-tts-zh',
      note: '完全免费，无需注册'
    }
  ],
  builtinFallback: {
    image: false,
    video: false,
    music: false,
    voice: false
  }
};
```

#### 预设三：极简方案（2 家厂商，性价比最高）

```typescript
const MINIMAL_PRESET: ConfigPreset = {
  id: 'minimal',
  name: '极简方案',
  description: '2 家厂商覆盖文本+图片+音乐，视频使用内置默认，语音用 Edge TTS 免费',
  vendorCount: 2,
  providers: [
    {
      providerId: 'siliconflow',
      modality: 'text',
      modelId: 'deepseek-v3.2',        // 硅基流动托管的 DeepSeek
      note: '1 个账号覆盖文本+图片，注册送大量免费 tokens'
    },
    {
      providerId: 'siliconflow',
      modality: 'image',
      modelId: 'flux-1-schnell',
      note: '同上账号'
    },
    {
      providerId: 'skymusic',
      modality: 'music',
      modelId: 'skymusic-v1',
      note: '昆仑万维旗下，部分免费'
    },
    {
      providerId: 'edge-tts',
      modality: 'voice',
      modelId: 'edge-tts-zh',
      note: '完全免费，无需注册'
    }
  ],
  builtinFallback: {
    image: false,
    video: true,       // 使用静态图+文字替代
    music: false,
    voice: false       // Edge TTS 已覆盖
  }
};
```

#### 预设四：自定义

玩家自由选择每个模态的服务商和模型，未配置的模态自动使用内置默认资源。

### 5.4 内置服务商定义

```typescript
const BUILTIN_PROVIDERS: AIProviderConfig[] = [
  // ─── 文本 ───
  {
    id: 'deepseek',
    name: 'DeepSeek',
    vendor: '深度求索',
    description: '国内顶尖大语言模型，API 兼容 OpenAI 格式，价格极低',
    officialUrl: 'https://www.deepseek.com',
    registerUrl: 'https://platform.deepseek.com/register',
    docsUrl: 'https://platform.deepseek.com/api-docs',
    modality: ['text'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'API Key',
        placeholder: 'sk-xxxxxxxxxxxxxxxxxxxxxxxx',
        helpUrl: 'https://platform.deepseek.com/api_keys'
      }
    },
    models: [
      {
        id: 'deepseek-v3.2',
        name: 'DeepSeek-V3.2',
        modality: 'text',
        isDefault: true,
        endpoint: 'https://api.deepseek.com/v1/chat/completions',
        maxTokens: 65536,
        costPerCall: 0.001,
        freeQuota: '注册送 100 万 tokens/月',
        quality: 'high'
      },
      {
        id: 'deepseek-r1',
        name: 'DeepSeek-R1 (推理)',
        modality: 'text',
        isDefault: false,
        endpoint: 'https://api.deepseek.com/v1/chat/completions',
        maxTokens: 65536,
        costPerCall: 0.004,
        freeQuota: '注册送 100 万 tokens/月',
        quality: 'high'
      },
      {
        id: 'deepseek-v4-pro',
        name: 'DeepSeek-V4-Pro (代码)',
        modality: 'text',
        isDefault: false,
        endpoint: 'https://api.deepseek.com/v1/chat/completions',
        maxTokens: 128000,
        costPerCall: 0.01,
        freeQuota: '注册送 100 万 tokens/月',
        quality: 'high'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 文本（永久免费） ───
  {
    id: 'xfyun-spark-lite',
    name: '讯飞星火 Lite',
    vendor: '科大讯飞',
    description: '永久免费，无限 Tokens！QPS 限制 2 次/秒，适合零成本体验',
    officialUrl: 'https://www.xfyun.cn',
    registerUrl: 'https://www.xfyun.cn/register',
    docsUrl: 'https://www.xfyun.cn/doc/spark/Web.html',
    modality: ['text', 'voice'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'APPID',
        placeholder: 'xxxxxxxx',
        helpUrl: 'https://console.xfyun.cn/services/bm3'
      },
      extraParams: {
        apiSecret: {
          value: '',
          label: 'APISecret',
          placeholder: 'xxxxxxxxxxxxxxxxxxxxxxxx',
          required: true,
          secret: true
        },
        apiKeyReal: {
          value: '',
          label: 'APIKey',
          placeholder: 'xxxxxxxxxxxxxxxxxxxxxxxx',
          required: true,
          secret: true
        }
      }
    },
    models: [
      {
        id: 'spark-lite',
        name: '星火 Lite',
        modality: 'text',
        isDefault: true,
        endpoint: 'wss://spark-api.xf-yun.com/v1.1/chat',
        maxTokens: 4096,
        costPerCall: 0,
        freeQuota: '永久免费，无限 Tokens，QPS 2次/秒',
        quality: 'standard'
      },
      {
        id: 'spark-tts',
        name: '星火 TTS',
        modality: 'voice',
        isDefault: true,
        endpoint: 'wss://tts-api.xfyun.cn/v2/tts',
        costPerCall: 0,
        freeQuota: '每日免费',
        quality: 'high'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 图片 + 文本（硅基流动） ───
  {
    id: 'siliconflow',
    name: '硅基流动 SiliconFlow',
    vendor: '硅基流动',
    description: '托管 FLUX/SD3 等图片模型和 DeepSeek/Qwen 文本模型，基础开源模型免费调用',
    officialUrl: 'https://siliconflow.cn',
    registerUrl: 'https://cloud.siliconflow.cn/register',
    docsUrl: 'https://docs.siliconflow.cn',
    modality: ['text', 'image'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'API Key',
        placeholder: 'sk-xxxxxxxxxxxxxxxxxxxxxxxx',
        helpUrl: 'https://cloud.siliconflow.cn/account/ak'
      }
    },
    models: [
      {
        id: 'deepseek-v3.2',
        name: 'DeepSeek-V3.2 (硅基流动托管)',
        modality: 'text',
        isDefault: false,
        endpoint: 'https://api.siliconflow.cn/v1/chat/completions',
        maxTokens: 65536,
        costPerCall: 0.001,
        freeQuota: '基础开源模型免费，付费模型按量计费',
        quality: 'high'
      },
      {
        id: 'flux-1-schnell',
        name: 'FLUX.1-schnell',
        modality: 'image',
        isDefault: true,
        endpoint: 'https://api.siliconflow.cn/v1/images/generations',
        supportedSizes: ['1:1', '16:9', '9:16', '4:3', '3:4'],
        costPerCall: 0.5,
        freeQuota: '注册送 2000 万 tokens',
        quality: 'fast'
      },
      {
        id: 'flux-1-dev',
        name: 'FLUX.1-dev',
        modality: 'image',
        isDefault: false,
        endpoint: 'https://api.siliconflow.cn/v1/images/generations',
        supportedSizes: ['1:1', '16:9', '9:16', '4:3', '3:4'],
        costPerCall: 2,
        freeQuota: '注册送 2000 万 tokens',
        quality: 'high'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 视频 ───
  {
    id: 'kling',
    name: '可灵 Kling',
    vendor: '快手',
    description: '国内领先的视频生成模型，3.0 版本支持 4K 60 帧、智能分镜、原生音频',
    officialUrl: 'https://kling.kuaishou.com',
    registerUrl: 'https://kling.kuaishou.com/register',
    docsUrl: 'https://platform.kuaishou.com/docs/kling',
    modality: ['video', 'image'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'Access Key',
        placeholder: 'ak-xxxxxxxxxxxxxxxx',
        helpUrl: 'https://platform.kuaishou.com/developer/key'
      },
      extraParams: {
        secretKey: {
          value: '',
          label: 'Secret Key',
          placeholder: 'sk-xxxxxxxxxxxxxxxx',
          required: true,
          secret: true
        }
      }
    },
    models: [
      {
        id: 'kling-3.0',
        name: '可灵 3.0',
        modality: 'video',
        isDefault: true,
        endpoint: 'https://api.klingai.com/v1/videos/generations',
        supportedSizes: ['16:9', '9:16', '1:1'],
        maxDuration: 10,
        costPerCall: 1,
        freeQuota: '每日免费 6 条',
        quality: 'high'
      },
      {
        id: 'kling-3.0-image',
        name: '可灵 3.0 Image',
        modality: 'image',
        isDefault: false,
        endpoint: 'https://api.klingai.com/v1/images/generations',
        supportedSizes: ['1:1', '16:9', '9:16', '4:3', '3:4'],
        costPerCall: 0.5,
        freeQuota: '每日免费额度',
        quality: 'high'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 视频（备选：海螺AI） ───
  {
    id: 'hailuo',
    name: '海螺AI (MiniMax)',
    vendor: 'MiniMax',
    description: '叙事感强的视频生成，首尾帧控制出色，适合故事性 CG',
    officialUrl: 'https://hailuoai.video',
    registerUrl: 'https://hailuoai.video/register',
    docsUrl: 'https://www.minimaxi.com/docs',
    modality: ['video'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'API Key',
        placeholder: 'xxxxxxxxxxxxxxxx',
        helpUrl: 'https://www.minimaxi.com/console/api-key'
      }
    },
    models: [
      {
        id: 'hailuo-02',
        name: 'Hailuo 02',
        modality: 'video',
        isDefault: true,
        endpoint: 'https://api.minimaxi.com/v1/video_generation',
        supportedSizes: ['16:9', '9:16', '1:1'],
        maxDuration: 6,
        costPerCall: 1,
        freeQuota: '免费额度较大',
        quality: 'high'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 音乐 ───
  {
    id: 'skymusic',
    name: '天工音乐 SkyMusic',
    vendor: '昆仑万维',
    description: '国内最早的 AI 音乐生成平台，每日免费额度',
    officialUrl: 'https://music.tiangong.cn',
    registerUrl: 'https://music.tiangong.cn/register',
    docsUrl: 'https://open.tiangong.cn/docs',
    modality: ['music', 'voice'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'API Key',
        placeholder: 'tg-xxxxxxxxxxxxxxxx',
        helpUrl: 'https://open.tiangong.cn/console/api-key'
      }
    },
    models: [
      {
        id: 'skymusic-v1',
        name: '天工音乐 V1',
        modality: 'music',
        isDefault: true,
        endpoint: 'https://api.tiangong.cn/v1/music/generations',
        maxDuration: 180,
        costPerCall: 1,
        freeQuota: '每日免费生成额度',
        quality: 'standard'
      },
      {
        id: 'skymusic-tts',
        name: '天工 TTS',
        modality: 'voice',
        isDefault: false,
        endpoint: 'https://api.tiangong.cn/v1/tts/generations',
        costPerCall: 0.1,
        freeQuota: '每日免费额度',
        quality: 'standard'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 语音（完全免费，无需注册） ───
  {
    id: 'edge-tts',
    name: 'Edge TTS',
    vendor: '微软',
    description: '完全免费，无需 API Key，无需注册！支持 40+ 语言包含中文多种方言',
    officialUrl: 'https://github.com/rany2/edge-tts',
    registerUrl: '',                    // 无需注册
    docsUrl: 'https://github.com/rany2/edge-tts#usage',
    modality: ['voice'],
    authType: 'api_key',               // 实际无需 Key，但保持接口一致
    authConfig: {
      apiKey: {
        value: 'FREE',                 // 内置标记，无需填写
        label: 'API Key',
        placeholder: '无需填写，完全免费',
        helpUrl: ''
      }
    },
    models: [
      {
        id: 'edge-tts-zh',
        name: 'Edge TTS 中文',
        modality: 'voice',
        isDefault: true,
        endpoint: 'wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1',
        costPerCall: 0,
        freeQuota: '完全免费，无限制',
        quality: 'standard'
      }
    ],
    status: 'connected'                // 默认即可用，无需配置
  },

  // ─── 语音（备选：火山引擎） ───
  {
    id: 'volcengine-tts',
    name: '火山引擎 TTS',
    vendor: '字节跳动',
    description: '音色丰富、支持情感控制的 TTS 服务',
    officialUrl: 'https://www.volcengine.com/product/tts',
    registerUrl: 'https://console.volcengine.com/register',
    docsUrl: 'https://www.volcengine.com/docs/6561/97465',
    modality: ['voice'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'Access Key ID',
        placeholder: 'AKLT-xxxxxxxxxxxxxxxx',
        helpUrl: 'https://console.volcengine.com/iam/keymanage'
      },
      extraParams: {
        secretKey: {
          value: '',
          label: 'Secret Access Key',
          placeholder: 'xxxxxxxxxxxxxxxxxxxxxxxx',
          required: true,
          secret: true
        },
        appId: {
          value: '',
          label: '应用 ID (App ID)',
          placeholder: 'xxxxxxxx',
          required: true,
          secret: false
        }
      }
    },
    models: [
      {
        id: 'volcengine-tts-v1',
        name: '火山引擎 TTS V1',
        modality: 'voice',
        isDefault: true,
        endpoint: 'https://openspeech.bytedance.com/api/v1/tts',
        costPerCall: 0.01,
        freeQuota: '新用户免费试用额度',
        quality: 'high'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 备选：通义千问 ───
  {
    id: 'tongyi',
    name: '通义千问',
    vendor: '阿里云',
    description: '阿里云大语言模型 Qwen3.6 + 通义万相图片/视频生成',
    officialUrl: 'https://tongyi.aliyun.com',
    registerUrl: 'https://dashscope.console.aliyun.com/register',
    docsUrl: 'https://help.aliyun.com/zh/dashscope',
    modality: ['text', 'image', 'video'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'DashScope API Key',
        placeholder: 'sk-xxxxxxxxxxxxxxxx',
        helpUrl: 'https://dashscope.console.aliyun.com/apiKey'
      }
    },
    models: [
      {
        id: 'qwen3.6-plus',
        name: 'Qwen3.6-Plus',
        modality: 'text',
        isDefault: true,
        endpoint: 'https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions',
        maxTokens: 131072,
        costPerCall: 0.004,
        freeQuota: '免费额度',
        quality: 'high'
      },
      {
        id: 'wanx-v2.1',
        name: '通义万相 V2.1',
        modality: 'image',
        isDefault: true,
        endpoint: 'https://dashscope.aliyuncs.com/api/v1/services/aigc/text2image/image-synthesis',
        supportedSizes: ['1:1', '16:9', '9:16'],
        costPerCall: 0.08,
        freeQuota: '每日免费额度',
        quality: 'standard'
      },
      {
        id: 'wanx-video',
        name: '通义万相视频',
        modality: 'video',
        isDefault: true,
        endpoint: 'https://dashscope.aliyuncs.com/api/v1/services/aigc/video-generation',
        supportedSizes: ['16:9', '9:16', '1:1'],
        maxDuration: 6,
        costPerCall: 1,
        freeQuota: '每日免费 10 次',
        quality: 'standard'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 备选：智谱 GLM ───
  {
    id: 'zhipu',
    name: '智谱 GLM',
    vendor: '智谱AI',
    description: '智谱清言大模型 GLM-4，支持文本和图片生成',
    officialUrl: 'https://open.bigmodel.cn',
    registerUrl: 'https://open.bigmodel.cn/register',
    docsUrl: 'https://open.bigmodel.cn/dev/api',
    modality: ['text', 'image'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'API Key',
        placeholder: 'xxxxxxxxxxxxxxxxxxxxxxxx.xxxxxxxxxxxxxxxx',
        helpUrl: 'https://open.bigmodel.cn/usercenter/apikeys'
      }
    },
    models: [
      {
        id: 'glm-4-plus',
        name: 'GLM-4-Plus',
        modality: 'text',
        isDefault: true,
        endpoint: 'https://open.bigmodel.cn/api/paas/v4/chat/completions',
        maxTokens: 128000,
        costPerCall: 0.015,
        freeQuota: '新用户赠送额度',
        quality: 'high'
      },
      {
        id: 'cogview-4',
        name: 'CogView-4',
        modality: 'image',
        isDefault: true,
        endpoint: 'https://open.bigmodel.cn/api/paas/v4/images/generations',
        supportedSizes: ['1:1', '16:9', '9:16'],
        costPerCall: 0.1,
        freeQuota: '新用户赠送额度',
        quality: 'standard'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 备选：即梦 ───
  {
    id: 'jimeng',
    name: '即梦 Jimeng',
    vendor: '字节跳动',
    description: '字节跳动旗下 AI 图片/视频生成平台',
    officialUrl: 'https://jimeng.jianying.com',
    registerUrl: 'https://jimeng.jianying.com/register',
    docsUrl: 'https://jimeng.jianying.com/docs',
    modality: ['image', 'video'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'API Key',
        placeholder: 'xxxxxxxxxxxxxxxx',
        helpUrl: 'https://jimeng.jianying.com/api-key'
      }
    },
    models: [
      {
        id: 'jimeng-image-v1',
        name: '即梦图片 V1',
        modality: 'image',
        isDefault: true,
        endpoint: 'https://api.jimeng.jianying.com/v1/images/generations',
        supportedSizes: ['1:1', '16:9', '9:16'],
        costPerCall: 0.05,
        freeQuota: '每日免费额度',
        quality: 'standard'
      },
      {
        id: 'jimeng-video-v1',
        name: '即梦视频 V1',
        modality: 'video',
        isDefault: true,
        endpoint: 'https://api.jimeng.jianying.com/v1/videos/generations',
        maxDuration: 6,
        costPerCall: 2,
        freeQuota: '每日免费额度',
        quality: 'standard'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 备选：Vidu ───
  {
    id: 'vidu',
    name: 'Vidu',
    vendor: '生数科技',
    description: '高质量视频生成模型，参考图生成能力出色',
    officialUrl: 'https://www.vidu.studio',
    registerUrl: 'https://www.vidu.studio/register',
    docsUrl: 'https://www.vidu.studio/docs',
    modality: ['video'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'API Key',
        placeholder: 'xxxxxxxxxxxxxxxx',
        helpUrl: 'https://www.vidu.studio/api-key'
      }
    },
    models: [
      {
        id: 'vidu-2.0',
        name: 'Vidu 2.0',
        modality: 'video',
        isDefault: true,
        endpoint: 'https://api.vidu.studio/v1/videos/generations',
        supportedSizes: ['16:9', '9:16', '1:1'],
        maxDuration: 8,
        costPerCall: 2,
        freeQuota: '新用户试用额度',
        quality: 'high'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 备选：讯飞 TTS ───
  {
    id: 'xfyun-tts',
    name: '讯飞 TTS',
    vendor: '科大讯飞',
    description: '国内老牌 TTS 服务，音色自然',
    officialUrl: 'https://www.xfyun.cn/services/online_tts',
    registerUrl: 'https://www.xfyun.cn/register',
    docsUrl: 'https://www.xfyun.cn/doc/tts/online_tts/API.html',
    modality: ['voice'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'APPID',
        placeholder: 'xxxxxxxx',
        helpUrl: 'https://console.xfyun.cn/services/tts'
      },
      extraParams: {
        apiSecret: {
          value: '',
          label: 'APISecret',
          placeholder: 'xxxxxxxxxxxxxxxxxxxxxxxx',
          required: true,
          secret: true
        },
        apiKeyReal: {
          value: '',
          label: 'APIKey',
          placeholder: 'xxxxxxxxxxxxxxxxxxxxxxxx',
          required: true,
          secret: true
        }
      }
    },
    models: [
      {
        id: 'xfyun-tts-v1',
        name: '讯飞在线语音合成',
        modality: 'voice',
        isDefault: true,
        endpoint: 'wss://tts-api.xfyun.cn/v2/tts',
        costPerCall: 0.005,
        freeQuota: '新用户 5 万次/月',
        quality: 'high'
      }
    ],
    status: 'unconfigured'
  },

  // ─── 备选：网易天音 ───
  {
    id: 'netease-music',
    name: '网易天音',
    vendor: '网易',
    description: '网易 AI 音乐生成平台',
    officialUrl: 'https://tianyin.163.com',
    registerUrl: 'https://tianyin.163.com/register',
    docsUrl: 'https://tianyin.163.com/docs',
    modality: ['music'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'API Key',
        placeholder: 'xxxxxxxxxxxxxxxx',
        helpUrl: 'https://tianyin.163.com/console/api-key'
      }
    },
    models: [
      {
        id: 'netease-music-v1',
        name: '网易天音 V1',
        modality: 'music',
        isDefault: true,
        endpoint: 'https://api.tianyin.163.com/v1/music/generations',
        maxDuration: 120,
        costPerCall: 1,
        freeQuota: '试用额度',
        quality: 'standard'
      }
    ],
    status: 'unconfigured'
  }
];
```

### 5.5 配置存储与安全

```typescript
// 配置存储方案
interface ConfigStorage {
  // 存储路径（Electron 用户数据目录）
  configDir: string;                 // 如 ~/.autofree/config/
  configFile: string;                // config.json（非敏感配置）
  secretsFile: string;               // secrets.enc（加密存储的敏感信息）
}

// 加密方案
interface EncryptionScheme {
  algorithm: 'aes-256-gcm';
  // 加密密钥来源：
  // - 桌面端：使用机器唯一标识派生密钥（无需用户输入密码）
  // - 可选：用户设置主密码，使用 PBKDF2 派生密钥
  keyDerivation: 'machine-id' | 'user-password';
  // machine-id 方案：使用 Electron 的 app.getPath('userData') 路径哈希作为密钥种子
  // 优点：零配置；缺点：换机器需重新配置
  // user-password 方案：用户设置主密码
  // 优点：可迁移；缺点：每次启动需输入密码
}
```

**存储策略**：
- 非敏感配置（模型选择、预设方案、全局设置）→ 明文 JSON
- 敏感信息（API Key、Secret Key、密码）→ AES-256-GCM 加密存储
- 默认使用 `machine-id` 方案，零配置体验
- 配置文件支持导出/导入（导出时敏感字段脱敏，需手动补充）

### 5.6 连通性检测

```typescript
interface ConnectivityCheck {
  providerId: string;
  timestamp: number;
  status: 'ok' | 'auth_failed' | 'network_error' | 'quota_exceeded' | 'unknown_error';
  latency?: number;                  // 响应延迟 ms
  errorMessage?: string;
  quotaInfo?: {
    remaining?: number;              // 剩余额度
    total?: number;                  // 总额度
    unit: string;                    // 额度单位（tokens/次/秒）
    resetAt?: number;                // 额度重置时间
  };
}

// 检测方式：每个 provider 实现一个轻量级健康检查
// - 文本模型：发送 "hi" 并检查响应
// - 图片模型：请求生成 64x64 最小尺寸图片
// - 视频模型：查询账户余额 API
// - 音乐模型：查询账户余额 API
// - 语音模型：合成 1 秒静音
```

### 5.7 配置管理 UI

#### 配置主界面

```
┌─────────────────────────────────────────────────────────┐
│  AI 服务配置                                [导出] [导入] │
│                                                         │
│  当前方案：[默认推荐 ▼]                                   │
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  文本生成                                            ││
│  │  ┌──────────────┐  ┌──────────────┐                ││
│  │  │ DeepSeek ⭐   │  │ 硅基流动      │                ││
│  │  │ ● 已连接      │  │ ○ 未配置      │                ││
│  │  └──────────────┘  └──────────────┘                ││
│  │  当前模型：DeepSeek-V3 ▼                             ││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  图片生成                                            ││
│  │  ┌──────────────┐  ┌──────────────┐                ││
│  │  │ 硅基流动 ⭐   │  │ 通义万相      │                ││
│  │  │ ● 已连接      │  │ ○ 未配置      │                ││
│  │  └──────────────┘  └──────────────┘                ││
│  │  当前模型：FLUX.1-schnell ▼                          ││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  视频生成                                            ││
│  │  ┌──────────────┐  ┌──────────────┐                ││
│  │  │ 可灵 ⭐       │  │ Vidu         │                ││
│  │  │ ○ 未配置      │  │ ○ 未配置      │                ││
│  │  └──────────────┘  └──────────────┘                ││
│  │  当前模型：可灵 V1 ▼                                  ││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  音乐生成                                            ││
│  │  ┌──────────────┐  ┌──────────────┐                ││
│  │  │ 天工音乐 ⭐   │  │ 网易天音      │                ││
│  │  │ ○ 未配置      │  │ ○ 未配置      │                ││
│  │  └──────────────┘  └──────────────┘                ││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  语音生成                                            ││
│  │  ┌──────────────┐  ┌──────────────┐                ││
│  │  │ 火山引擎TTS ⭐│  │ 讯飞 TTS     │                ││
│  │  │ ○ 未配置      │  │ ○ 未配置      │                ││
│  │  └──────────────┘  └──────────────┘                ││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  [全部检测]                    [恢复默认]                 │
└─────────────────────────────────────────────────────────┘
```

#### 服务商配置详情弹窗

```
┌─────────────────────────────────────────────────────────┐
│  配置 DeepSeek                                    [×]   │
│                                                         │
│  深度求索 — 国内顶尖大语言模型，API 兼容 OpenAI 格式      │
│                                                         │
│  API Key                                                │
│  ┌─────────────────────────────────────────┐ [显示/隐藏] │
│  │ sk-•••••••••••••••••••••••••••••••••••• │ [获取Key→]  │
│  └─────────────────────────────────────────┘            │
│                                                         │
│  模型选择                                               │
│  ┌─────────────────────────────────────────┐            │
│  │ DeepSeek-V3 ▼                            │            │
│  └─────────────────────────────────────────┘            │
│                                                         │
│  连通状态                                               │
│  ● 已连接  延迟 230ms  剩余额度：约 420 万 tokens        │
│                                                         │
│  免费额度说明                                           │
│  注册即送 500 万 tokens，约可生成 50 个章节的游戏脚本     │
│                                                         │
│  注册指引                                               │
│  1. 访问 platform.deepseek.com/register 注册账号         │
│  2. 进入 API Keys 页面创建新 Key                         │
│  3. 将 Key 粘贴到上方输入框                              │
│                                                         │
│            [检测连接]    [保存]    [取消]                  │
└─────────────────────────────────────────────────────────┘
```

#### 预设方案选择界面

```
┌─────────────────────────────────────────────────────────┐
│  选择配置方案                                            │
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  零成本（推荐体验）                      [0 家厂商] ││
│  │  完全免费，无需注册任何账号，安装即可游玩            ││
│  │  讯飞星火Lite + Edge TTS（图片/音乐/视频用内置默认）││
│  │                                          [选择此方案]││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  仅文本 AI（推荐入门）                    [1 家厂商] ││
│  │  1 个 DeepSeek + Edge TTS，语音+交互全由 AI 生成    ││
│  │  DeepSeek + Edge TTS（图片/音乐/视频用内置默认）    ││
│  │                                          [选择此方案]││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  极简方案                                [2 家厂商] ││
│  │  文本+图片+音乐由 AI 生成，视频用内置默认            ││
│  │  硅基流动 + 天工音乐 + Edge TTS                      ││
│  │                                          [选择此方案]││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  默认推荐                                [4 家厂商] ││
│  │  功能最全，覆盖全部 5 个模态，免费额度充足           ││
│  │  DeepSeek + 硅基流动 + 可灵3.0 + 天工 + Edge TTS    ││
│  │                                          [选择此方案]││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  自定义                                              ││
│  │  自由选择每个模态的服务商和模型，未配置的用内置默认   ││
│  │                                          [选择此方案]││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  提示：未配置 AI 的模态会使用内置默认资源，不影响游戏逻辑 │
└─────────────────────────────────────────────────────────┘
```

### 5.8 配置管理接口

配置管理通过 Tauri IPC 命令实现（详见第 8 节），核心命令：

| 命令 | 说明 |
|------|------|
| `get_config` | 获取当前配置（敏感字段脱敏） |
| `update_config` | 更新全局设置 |
| `get_presets` | 获取预设方案列表 |
| `apply_preset` | 应用预设方案 |
| `get_providers` | 获取所有服务商配置（敏感字段脱敏） |
| `update_provider` | 更新服务商配置（含 API Key 等） |
| `check_provider` | 检测单个服务商连通性 |
| `check_all_providers` | 批量检测所有已配置服务商 |
| `export_config` | 导出配置（脱敏） |
| `import_config` | 导入配置 |

---

## 6. 游戏运行时系统

### 6.1 游戏状态管理

```typescript
interface GameState {
  // 存档信息
  saveId: string;
  gameScript: GameScript;
  currentChapterId: string;
  currentSceneId: string;
  currentNodeId: string;

  // 运行时变量
  variables: Record<string, any>;     // 全局 + 章节变量
  inventory: string[];                // 物品栏
  stats: Record<string, number>;      // 角色属性（RPG 类型）

  // 历史记录
  choiceHistory: ChoiceRecord[];      // 玩家选择历史
  visitedScenes: string[];            // 已访问场景
  unlockedCGs: string[];              // 已解锁 CG

  // 生成状态
  generationProgress: {
    totalAssets: number;
    completedAssets: number;
    failedAssets: number;
    chapterStatus: Record<string, 'generating' | 'ready' | 'partial'>;
  };
}

interface ChoiceRecord {
  choiceNodeId: string;
  selectedOptionIndex: number;
  selectedOptionText: string;
  timestamp: number;
  chapterId: string;
  sceneId: string;
}
```

### 6.2 场景执行引擎

场景节点按 `sequence` 数组顺序执行，遇到分支节点时根据玩家选择或条件跳转：

```
sequence: [narration_1, dialogue_1, dialogue_2, choice_1, ...]
                                                    │
                                          ┌─────────┼─────────┐
                                          ▼         ▼         ▼
                                     option_A   option_B   option_C
                                          │         │         │
                                          ▼         ▼         ▼
                                     narration_2 dialogue_3 action_1
```

### 6.3 存档系统

- 自动存档：每次进入新场景自动存档
- 手动存档：玩家可在任意对话/旁白节点存档
- 存档内容：完整 `GameState` + 已生成资源引用
- 存档大小：仅存储状态和资源引用，不存储资源本身（资源通过 cacheKey 从缓存加载）

---

## 7. 前端 UI 设计

### 7.1 主界面流程

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   主菜单      │────→│  创建游戏     │────→│  生成进度     │
│              │     │              │     │  (进度条)     │
│ · 新游戏     │     │ · 输入大纲    │     │              │
│ · 继续游戏   │     │ · 随机生成    │     └──────┬───────┘
│ · 游戏列表   │     │ · 选择类型    │            │
│ · 设置       │     └──────────────┘            ▼
└──────────────┘                          ┌──────────────┐
                                          │   游戏主界面   │
                                          │              │
                                          │ ┌──────────┐ │
                                          │ │ 场景画面  │ │
                                          │ │          │ │
                                          │ └──────────┘ │
                                          │ ┌──────────┐ │
                                          │ │ 对话框    │ │
                                          │ │ 选项列表  │ │
                                          │ └──────────┘ │
                                          └──────────────┘
```

### 7.2 游戏主界面布局

```
┌─────────────────────────────────────────────────┐
│  [章节标题]                    [菜单] [存档] [设置] │
│                                                 │
│  ┌─────────────────────────────────────────────┐│
│  │                                             ││
│  │            场景画面区域                       ││
│  │      (背景图/视频 + CG 动画播放)              ││
│  │                                             ││
│  │                                             ││
│  └─────────────────────────────────────────────┘│
│                                                 │
│  ┌─────────────────────────────────────────────┐│
│  │ [NPC头像] NPC名称                            ││
│  │                                             ││
│  │ 对话文本 / 旁白文本                           ││
│  │ ........................................... ││
│  │                                             ││
│  │  ┌───────────┐  ┌───────────┐              ││
│  │  │  选项 A    │  │  选项 B    │              ││
│  │  └───────────┘  └───────────┘              ││
│  │  ┌───────────┐                              ││
│  │  │  选项 C    │                              ││
│  │  └───────────┘                              ││
│  └─────────────────────────────────────────────┘│
│  [物品栏] [状态] [CG回廊]                        │
└─────────────────────────────────────────────────┘
```

### 7.3 生成进度界面

```
┌─────────────────────────────────────────────────┐
│           正在生成游戏：迷雾庄园                    │
│                                                 │
│  第一章：抵达                    [可游玩 ▶]       │
│  ████████████████████░░░░  85%                  │
│  ✓ 场景图片  ✓ NPC头像  ✓ BGM  ○ CG动画  ○ 语音  │
│                                                 │
│  第二章：失踪                    [生成中...]      │
│  ██████████░░░░░░░░░░░░░  40%                  │
│  ✓ 场景图片  ○ NPC头像  ○ BGM  ○ CG动画  ○ 语音  │
│                                                 │
│  第三章：真相                    [等待中]         │
│  ░░░░░░░░░░░░░░░░░░░░░░  0%                   │
│  ○ 场景图片  ○ NPC头像  ○ BGM  ○ CG动画  ○ 语音  │
│                                                 │
│           [开始第一章 ▶]                          │
└─────────────────────────────────────────────────┘
```

---

## 8. Tauri IPC 命令设计

### 8.1 核心 IPC 命令

> 采用 Tauri IPC 命令替代 HTTP API + WebSocket。前端通过 `invoke()` 调用 Rust 侧命令，通过事件系统接收实时通知。

```typescript
// 前端调用示例
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// 调用命令（gameType 可选，AI 会从输入内容推断）
const result = await invoke('create_game', { input, gameType: undefined });

// 监听事件
const unlisten = await listen('generation-progress', (event) => {
  console.log(event.payload);
});
```

**游戏管理命令**：

```rust
#[tauri::command]
async fn create_game(input: String, game_type: Option<String>) -> Result<GameInfo, String>

#[tauri::command]
async fn random_outline(game_type: Option<String>, themes: Vec<String>) -> Result<String, String>

#[tauri::command]
async fn get_game(game_id: String) -> Result<GameInfo, String>

#[tauri::command]
async fn get_game_script(game_id: String) -> Result<GameScript, String>

#[tauri::command]
async fn list_games() -> Result<Vec<GameInfo>, String>

#[tauri::command]
async fn delete_game(game_id: String) -> Result<(), String>

#[tauri::command]
async fn save_game(game_id: String, state: GameState) -> Result<String, String>

#[tauri::command]
async fn load_save(game_id: String, save_id: String) -> Result<GameState, String>

#[tauri::command]
async fn list_saves(game_id: String) -> Result<Vec<SaveInfo>, String>
```

**生成调度命令**：

```rust
#[tauri::command]
async fn get_generation_status(game_id: String) -> Result<GenerationStatus, String>

#[tauri::command]
async fn regenerate_asset(game_id: String, asset_ref_id: String) -> Result<(), String>

#[tauri::command]
async fn export_game(game_id: String, output_path: String) -> Result<(), String>
```

**配置管理命令**：

```rust
#[tauri::command]
async fn get_config() -> Result<AppConfig, String>

#[tauri::command]
async fn update_config(config: AppConfig) -> Result<(), String>

#[tauri::command]
async fn get_presets() -> Result<Vec<ConfigPreset>, String>

#[tauri::command]
async fn apply_preset(preset_id: String) -> Result<(), String>

#[tauri::command]
async fn get_providers() -> Result<Vec<AIProviderConfig>, String>

#[tauri::command]
async fn update_provider(provider: AIProviderConfig) -> Result<(), String>

#[tauri::command]
async fn check_provider(provider_id: String) -> Result<ConnectivityCheck, String>

#[tauri::command]
async fn check_all_providers() -> Result<Vec<ConnectivityCheck>, String>

#[tauri::command]
async fn export_config() -> Result<String, String>  // 返回脱敏 JSON

#[tauri::command]
async fn import_config(config_json: String) -> Result<(), String>
```

### 8.2 事件通知

```rust
// Rust 侧发送事件
app.emit("generation-progress", payload)?;
app.emit("asset-ready", payload)?;
app.emit("generation-complete", payload)?;
app.emit("generation-failed", payload)?;
```

```typescript
// 前端监听事件
interface GenerationProgressEvent {
  gameId: string;
  chapterId: string;
  assetType: string;
  progress: number;
  status: 'generating' | 'ready' | 'failed';
}

interface AssetReadyEvent {
  gameId: string;
  assetRefId: string;
  assetType: string;
  localPath: string;       // 本地资源路径
  source: 'ai_generated' | 'builtin';
}

interface GenerationCompleteEvent {
  gameId: string;
  chapterId: string;
}

interface GenerationFailedEvent {
  gameId: string;
  assetRefId: string;
  error: string;
  willRetry: boolean;
  fallbackToBuiltin: boolean;  // 是否降级到内置默认资源
}
```

### 8.3 Web 端适配

Web 端无法使用 Tauri IPC，需要适配层：

```typescript
// 适配层：根据运行环境选择 IPC 方式
const isTauri = '__TAURI__' in window;

export const api = {
  async createGame(input: string, gameType?: string) {
    if (isTauri) {
      return invoke('create_game', { input, gameType: gameType || null });
    }
    // Web 端：直接在前端调用 AI API
    return webApi.createGame(input, gameType);
  },
  // ...其他方法同理
};
```

---

## 9. AI Prompt 工程详细设计

### 9.1 大纲解析 Prompt

大纲解析采用**两步策略**：先扩展再解析。对于简短输入，先用 AI 扩展为完整大纲，再解析为 GameScript。

**Step 1：扩展（仅对简短输入）**

判断逻辑：如果输入文本少于 50 字或未包含章节结构，先执行扩展。

```json
{
  "system": "你是一个游戏设计师。玩家给出了一个简短的游戏构想，请将其扩展为完整的游戏大纲。\n\n要求：\n1. 推断最合适的游戏类型（视觉小说/RPG/悬疑解谜/恐怖生存/模拟经营）\n2. 生成游戏名称\n3. 设计 3 个章节，每章包含 2-3 个场景\n4. 为每个场景设计关键角色、事件和冲突\n5. 每章至少一个玩家选择分支\n6. 设计至少 2 个结局\n7. 保持玩家原始构想的核心要素，其余自由发挥\n\n输出格式：\n游戏名称：...\n类型：...\n\n第一章：[章节名]\n- ...\n\n第二章：[章节名]\n- ...\n\n第三章：[章节名]\n- ...",
  "user": "{rawInput}"
}
```

**Step 2：解析为 GameScript**

```json
{
  "system": "你是一个游戏脚本解析器。将用户输入的游戏大纲解析为结构化的 GameScript JSON。严格遵循以下规则：\n1. 每个章节至少包含2个场景\n2. 每个场景至少包含3个交互节点（旁白/对话/选项）\n3. 关键剧情节点必须包含CG动画描述\n4. 每个章节至少有一个玩家选择分支\n5. 为所有需要生成的资源编写详细的 prompt\n6. 输出必须是合法的 JSON，格式遵循 GameScript 类型定义\n7. 如果大纲信息不完整，自主补充合理的细节（角色外貌、场景氛围、对话风格等）",
  "user": "请将以下游戏大纲解析为 GameScript JSON：\n\n{outline}"
}
```

**合并优化（单次调用版）**：对于输入不太短的情况，可合并为单次调用以减少延迟：

```json
{
  "system": "你是一个游戏脚本生成器。根据玩家的描述生成完整的 GameScript JSON。\n\n玩家可能给出：一句话、几句话、一段描述、或完整大纲。无论输入多简短，你都必须生成完整的 GameScript。\n\n规则：\n1. 如果输入缺少游戏名称，自主生成一个贴切的名称\n2. 如果输入未指定游戏类型，根据内容推断最合适的类型\n3. 如果输入未分章节，自主设计 3 个章节\n4. 每个章节至少 2 个场景，每个场景至少 3 个交互节点\n5. 关键剧情节点包含 CG 动画描述\n6. 每章至少一个玩家选择分支\n7. 为所有资源编写详细的生成 prompt\n8. 自主补充角色外貌、场景氛围、对话风格等细节\n9. 输出必须是合法的 JSON，格式遵循 GameScript 类型定义",
  "user": "{rawInput}"
}
```

### 9.2 NPC 对话生成 Prompt

```json
{
  "system": "你是游戏角色 {npcName}。角色设定：{npcProfile}。当前场景：{sceneDescription}。当前情感：{emotion}。请以角色身份说出符合当前情境的台词。输出格式：{\"text\": \"对话内容\", \"emotion\": \"情感标注\"}",
  "user": "当前情境：{context}\n其他角色刚刚说了：{previousDialogue}\n请说出你的台词。"
}
```

### 9.3 场景图片 Prompt 模板

```
{stylePrefix}, {sceneDescription}, {timeOfDay}, {weather}, {mood},
game background, high quality, detailed, {aspectRatio}
```

**风格前缀示例**：
- 视觉小说：`anime style, visual novel aesthetic, soft lighting`
- 悬疑解谜：`dark atmospheric, noir style, dramatic shadows`
- 恐怖生存：`horror aesthetic, dark and gritty, unsettling atmosphere`
- RPG 冒险：`fantasy art style, epic landscape, vibrant colors`
- 模拟经营：`cozy pixel art, warm colors, isometric view`

### 9.4 BGM Prompt 模板

```
{gameType} game background music, {mood}, {tempo}, {instruments},
{genre}, loopable, no vocals, {duration}s
```

---

## 10. 技术栈选型

### 10.1 前端 / 客户端

| 技术 | 用途 | 理由 |
|------|------|------|
| Tauri | 跨平台客户端 | 比 Electron 安装包小 10 倍（~10MB vs ~150MB），支持 Windows/macOS/Linux，内存占用低 |
| React + TypeScript | UI 框架 | 组件化、类型安全，Tauri 原生支持 Web 前端 |
| PixiJS | 场景渲染 | 2D 渲染、特效、动画 |
| Howler.js | 音频播放 | BGM 循环、音效混合 |
| Zustand | 状态管理 | 轻量、TypeScript 友好 |

**Tauri 选型理由**：
- 安装包极小（~10MB），降低下载门槛
- 跨平台：Windows / macOS / Linux 原生支持，Web 端可直接部署为 SPA
- Rust 后端性能好，适合本地文件操作和 AI API 调用
- 无需单独的后端服务，Tauri 的 Rust 侧直接处理 AI 调用和本地资源管理
- 安全性更好，默认最小权限模型

**Web 端适配**：
- Tauri 应用可直接作为 Web SPA 运行（AI 调用走前端 HTTP 请求）
- Web 端资源存储使用 IndexedDB / OPFS（Origin Private File System）
- 桌面端和 Web 端共享 95% 以上代码

### 10.2 后端（Tauri Rust 侧）

| 技术 | 用途 | 理由 |
|------|------|------|
| Rust (Tauri) | 本地服务 | 与前端同进程，无需单独部署后端 |
| reqwest | HTTP 客户端 | 调用各 AI 模型 API |
| tokio | 异步运行时 | 并行 AI 生成任务 |
| rusqlite | 本地数据库 | 游戏数据、存档、配置 |
| serde_json | JSON 处理 | GameScript 解析与序列化 |
| aes-gcm | 加密 | API Key 等敏感信息加密存储 |

> **架构简化**：采用 Tauri 单体架构，无需 Node.js 后端服务。Rust 侧直接处理 AI API 调用、任务调度、本地资源管理，前端通过 Tauri IPC 调用。Web 端部署时，AI 调用走前端 HTTP 请求。

### 10.3 资源存储（全部本地化）

| 方案 | 用途 | 说明 |
|------|------|------|
| 本地文件系统 | AI 生成资源 + 游戏存档 | 桌面端：`~/autofree/assets/` |
| 应用内置资源 | 默认图片/BGM/音效 | 随应用分发，~20MB |
| IndexedDB / OPFS | Web 端资源存储 | 浏览器环境替代方案 |
| SQLite | 游戏数据/配置/存档 | 轻量嵌入式数据库 |

**本地资源目录结构**：

```
~/autofree/
├── config/
│   ├── config.json              # 非敏感配置
│   └── secrets.enc              # 加密的 API Key
├── games/
│   └── {gameId}/
│       ├── script.json          # GameScript
│       ├── saves/               # 存档
│       └── assets/              # 该游戏的 AI 生成资源
│           ├── images/
│           ├── videos/
│           ├── music/
│           └── voices/
└── cache/                       # 跨游戏共享的资源缓存
    └── {cacheKey}
```

**所有资源归玩家所有**：
- AI 生成的图片、视频、音乐、语音全部保存在本地
- 玩家可直接访问资源目录，自由使用生成的资源
- 支持导出游戏资源包（含 GameScript + 所有资源文件）
- 离线可玩：资源生成后无需网络即可游玩

---

## 11. 项目结构

```
autoFree/
├── src-tauri/                     # Tauri Rust 后端
│   ├── src/
│   │   ├── main.rs                # Tauri 入口
│   │   ├── commands/              # Tauri IPC 命令（前端调用）
│   │   │   ├── game.rs            # 游戏管理命令
│   │   │   ├── generation.rs      # 生成调度命令
│   │   │   ├── config.rs          # 配置管理命令
│   │   │   └── asset.rs           # 资源管理命令
│   │   ├── engine/                # 生成引擎
│   │   │   ├── outline_parser.rs  # 大纲解析器
│   │   │   ├── pipeline.rs        # 统一生成管线
│   │   │   └── asset_manager.rs   # 本地资源管理器
│   │   ├── providers/             # AI 模型适配器（统一 IAssetProvider trait）
│   │   │   ├── mod.rs             # IAssetProvider trait 定义
│   │   │   ├── builtin.rs         # 内置默认资源 Provider
│   │   │   ├── deepseek.rs
│   │   │   ├── xfyun_spark.rs     # 讯飞星火（永久免费文本+TTS）
│   │   │   ├── edge_tts.rs        # Edge TTS（完全免费，无需注册）
│   │   │   ├── siliconflow.rs
│   │   │   ├── kling.rs           # 可灵 3.0（视频+图片）
│   │   │   ├── hailuo.rs          # 海螺AI/MiniMax（视频）
│   │   │   ├── skymusic.rs
│   │   │   ├── volcengine_tts.rs
│   │   │   ├── tongyi.rs          # 通义千问 Qwen3.6（文本+图片+视频）
│   │   │   ├── zhipu.rs           # 智谱 GLM-4-Plus
│   │   │   ├── jimeng.rs
│   │   │   ├── vidu.rs            # Vidu 2.0
│   │   │   ├── xfyun_tts.rs       # 讯飞 TTS（备选）
│   │   │   └── netease_music.rs
│   │   ├── config/                # 配置管理
│   │   │   ├── manager.rs         # 配置管理器
│   │   │   ├── encryption.rs      # 加密存储
│       │   │   ├── presets/           # 预设方案
│       │   │   │   ├── zero_cost.rs   # 零成本
│       │   │   │   ├── text_only.rs   # 仅文本
│       │   │   │   ├── default.rs     # 默认推荐
│       │   │   │   └── minimal.rs     # 极简方案
│   │   │   └── providers/         # 服务商定义
│   │   │       ├── mod.rs         # BUILTIN_PROVIDERS
│   │   │       └── connectivity.rs
│   │   └── db/                    # 数据库
│   │       └── mod.rs             # SQLite 操作
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── src/                           # 前端 React 应用
│   ├── components/                # UI 组件
│   │   ├── Scene/                 # 场景渲染
│   │   ├── Dialogue/              # 对话系统
│   │   ├── Choice/                # 选项系统
│   │   ├── CG/                    # CG 播放器
│   │   ├── HUD/                   # 状态栏/物品栏
│   │   └── Config/                # AI 配置管理组件
│   ├── engine/                    # 游戏引擎（前端逻辑）
│   │   ├── SceneExecutor.ts       # 场景执行器
│   │   ├── StateManager.ts        # 状态管理器
│   │   ├── AssetLoader.ts         # 资源加载器
│   │   └── AudioEngine.ts         # 音频引擎
│   ├── pages/                     # 页面
│   │   ├── MainMenu.tsx
│   │   ├── CreateGame.tsx
│   │   ├── GenerationProgress.tsx
│   │   ├── GamePlay.tsx
│   │   └── Settings.tsx           # 设置与 AI 配置页
│   ├── store/                     # 状态仓库
│   ├── hooks/                     # Tauri IPC 封装
│   │   ├── useGame.ts
│   │   ├── useGeneration.ts
│   │   └── useConfig.ts
│   └── App.tsx
│
├── shared/                        # 共享类型定义
│   └── types/
│       ├── game-script.ts         # GameScript 类型定义
│       ├── game-state.ts          # GameState 类型定义
│       ├── asset.ts               # 资源相关类型
│       └── ai-provider.ts         # AI 服务商配置类型
│
├── builtin-assets/                # 内置默认资源（随应用分发）
│   ├── images/                    # 默认场景图（按游戏类型分类）
│   │   ├── visual_novel/          # 视觉小说风格
│   │   ├── mystery/               # 悬疑风格
│   │   ├── horror/                # 恐怖风格
│   │   ├── rpg/                   # RPG 风格
│   │   └── simulation/            # 模拟经营风格
│   ├── music/                     # 默认 BGM
│   │   ├── calm.mp3
│   │   ├── tense.mp3
│   │   ├── dark.mp3
│   │   ├── happy.mp3
│   │   └── battle.mp3
│   ├── sfx/                       # 默认音效
│   └── portraits/                 # 默认 NPC 头像
│       ├── male/
│       └── female/
│
├── prompts/                       # Prompt 模板库
│   ├── outline-parser/
│   ├── dialogue/
│   ├── narration/
│   ├── image/
│   ├── video/
│   └── music/
│
├── package.json
└── vite.config.ts                 # Vite 配置（Tauri 集成）
```

---

## 12. 关键技术挑战与解决方案

### 12.1 AI 生成内容一致性

**问题**：不同 AI 模型生成的内容风格可能不统一（如图片风格与音乐氛围不匹配）。

**解决方案**：
- 引入"风格锚点"机制：大纲解析时提取风格关键词，所有生成 Prompt 共享
- 图片生成使用参考图（style reference）保持视觉一致
- 音乐生成共享调性、乐器、节奏描述

### 12.2 生成耗时

**问题**：视频生成可能需要数分钟，玩家等待时间长。

**解决方案**：
- 渐进式加载：第一章就绪即可开始
- 降级展示：资源未就绪时用占位内容
- 预生成：后台提前生成后续章节
- 缓存：相似 Prompt 复用已生成资源

### 12.3 生成质量不可控

**问题**：AI 生成内容可能不符合预期。

**解决方案**：
- 重新生成：玩家可对单个资源点击"重新生成"
- Prompt 微调：提供简易 Prompt 编辑器，玩家可调整生成参数
- 多候选：关键资源一次生成多个候选，玩家选择最佳

### 12.4 交互逻辑的可靠性

**问题**：AI 生成的分支逻辑可能有死循环或断链。

**解决方案**：
- GameScript 校验器：生成后自动检查节点可达性
- 死链修复：自动添加兜底跳转
- 人工预览模式：生成后可预览分支图，手动修正

### 12.5 成本控制

**问题**：大量 AI 调用成本较高。

**解决方案**：
- 资源缓存：相同/相似 Prompt 复用缓存
- 按需生成：仅生成当前章节和下一章节
- 模型分级：非关键内容使用低成本模型
- 本地模型备选：支持接入本地部署的开源模型

---

## 13. 开发里程碑

### Phase 1 — MVP（核心管线打通）
- [ ] Tauri 项目初始化 + React 前端搭建
- [ ] GameScript 类型定义与校验
- [ ] AI 服务商配置类型定义
- [ ] 内置默认资源库（5 种游戏类型场景图 + 5 首 BGM + NPC 头像）
- [ ] 统一生成管线（IAssetProvider 接口 + BuiltinAssetProvider）
- [ ] 配置管理器（预设方案、加密存储、连通性检测）
- [ ] 大纲解析器（文本AI → GameScript）
- [ ] 基础场景渲染（背景图 + 对话框 + 选项）
- [ ] 单章节游玩流程（仅文本 AI + 内置默认资源）
- [ ] 存档/读档
- [ ] AI 配置管理 UI（预设选择、API Key 填写、连通检测）

### Phase 2 — 多模态 AI 集成
- [ ] 图片 AI 集成（硅基流动 FLUX）— AIAssetProvider
- [ ] 音乐 AI 集成（天工音乐）
- [ ] 语音 AI 集成（火山引擎 TTS）
- [ ] 视频 AI 集成（可灵）
- [ ] 多章节串联
- [ ] 渐进式加载 + AI 生成资源热替换内置默认

### Phase 3 — 体验优化
- [ ] 随机大纲生成
- [ ] 多游戏类型支持
- [ ] 重新生成 / 多候选
- [ ] Prompt 编辑器
- [ ] 分支预览图
- [ ] CG 回廊
- [ ] 游戏资源包导出

### Phase 4 — 高级功能
- [ ] RPG 战斗系统
- [ ] 物品/属性系统
- [ ] 多结局追踪
- [ ] 社区分享（大纲/存档）
- [ ] Web 端适配（IndexedDB + 前端 AI 调用）
- [ ] 本地模型支持
