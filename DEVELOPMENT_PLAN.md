# AI 自动化编程 — 开发计划与提交节点

> 基于 GAME_DESIGN.md 设计文档，拆分为可独立提交的开发节点。
> 每个节点是一个可编译、可运行的增量，AI 编程时按顺序逐节点实施。

---

## 节点总览

| # | 状态 | 节点名称 | 核心交付物 | 依赖 |
|---|------|---------|-----------|------|
| 01 | ☐ | 项目脚手架 | Tauri + React + TS 项目可运行 | 无 |
| 02 | ☐ | 共享类型定义 | GameScript / GameState / AI Provider 类型 | 01 |
| 03 | ☐ | 内置默认资源库 | 默认图片/BGM/头像占位资源 | 01 |
| 04 | ☐ | Rust 配置管理 | 配置管理器 + 加密存储 + 预设方案 | 02 |
| 05 | ☐ | AI Provider Trait + 内置 Provider | IAssetProvider trait + BuiltinAssetProvider | 02, 03 |
| 06 | ☐ | 文本 AI Provider（DeepSeek） | DeepSeek / OpenAI 兼容文本生成 | 04, 05 |
| 07 | ☐ | 大纲解析器 | 文本 → GameScript 解析管线 | 06 |
| 08 | ☐ | 统一生成管线 + 资源管理器 | GenerationPipeline + AssetManager | 05, 07 |
| 09 | ☐ | Tauri IPC 命令（游戏+配置） | game/config/asset 命令注册 | 04, 08 |
| 10 | ☐ | 前端：主菜单 + 创建游戏页 | 页面路由 + 大纲输入 UI | 09 |
| 11 | ☐ | 前端：AI 配置管理页 | 预设选择 + API Key 填写 + 连通检测 | 09 |
| 12 | ☐ | 前端：生成进度页 | 章节生成进度展示 + 事件监听 | 09, 10 |
| 13 | ☐ | 前端：场景渲染器 | 背景图 + 对话框 + 选项组件 | 10 |
| 14 | ☐ | 前端：游戏引擎核心 | SceneExecutor + StateManager + 存档 | 13 |
| 15 | ☐ | 前端：游戏主界面整合 | GamePlay 页面完整流程 | 13, 14 |
| 16 | ☐ | Edge TTS Provider | 免费语音生成 | 05 |
| 17 | ☐ | 图片 AI Provider（硅基流动 FLUX） | AI 图片生成 | 05 |
| 18 | ☐ | 音乐 AI Provider（天工音乐） | AI BGM 生成 | 05 |
| 19 | ☐ | 视频 AI Provider（可灵） | AI CG 视频生成 | 05 |
| 20 | ☐ | 渐进式加载 + 热替换 | 边生成边游玩 + 资源实时替换 | 15, 16-19 |
| 21 | ☐ | 讯飞星火 Lite Provider | 零成本文本 AI | 06 |
| 22 | ☐ | GameScript 校验器 | 节点可达性检查 + 死链修复 | 07 |
| 23 | ☐ | 随机大纲生成 | 按类型随机生成游戏大纲 | 07 |
| 24 | ☐ | 重新生成 + 多候选 | 单资源重新生成 + 候选选择 | 20 |
| 25 | ☐ | CG 回廊 + 资源导出 | CG 浏览 + 游戏包导出 | 15 |
| 26 | ☐ | 备选 Provider 集成 | 通义千问/智谱/即梦/Vidu/火山引擎等 | 05 |
| 27 | ☐ | Web 端适配 | IndexedDB + 前端 AI 调用适配层 | 全部 |

> 状态说明：☐ 未开始 | 🔲 进行中 | ☑ 已完成

---

## 节点详细说明

### 节点 01：项目脚手架

**目标**：Tauri + React + TypeScript 项目可编译运行

**具体任务**：
- 使用 `npm create tauri-app` 初始化项目（React + TS + Vite）
- 配置 `tauri.conf.json`（窗口大小、应用名称、权限）
- 安装前端依赖：`zustand`, `@tauri-apps/api`, `pixi.js`, `howler`
- 安装 Rust 依赖：`reqwest`, `tokio`, `serde_json`, `rusqlite`, `aes-gcm`
- 建立项目目录结构（按设计文档 §11）
- 配置 Vite 别名路径
- 验证：`npm run tauri dev` 可启动空白窗口

**提交信息**：`feat: init Tauri + React + TypeScript project scaffold`

---

### 节点 02：共享类型定义

**目标**：定义所有核心 TypeScript / Rust 共享类型

**具体任务**：
- `shared/types/game-script.ts`：GameScript, GameMeta, Chapter, Scene, SceneNode（Narration/Dialogue/Choice/Condition/Action/CG/SceneTransition）, AssetRef, VariableDef, Effect, Condition, Transition, GameType
- `shared/types/game-state.ts`：GameState, ChoiceRecord, GenerationProgress
- `shared/types/asset.ts`：LocalAsset, AIModality
- `shared/types/ai-provider.ts`：AIProviderConfig, AuthConfig, AIModelConfig, ConfigPreset, AppConfig, ConnectivityCheck, GenerationTask
- Rust 侧对应 struct 定义（`src-tauri/src/types/`），使用 serde Serialize/Deserialize
- 导出 TypeScript 类型供前端使用

**提交信息**：`feat: define shared types for GameScript, GameState, and AI providers`

---

### 节点 03：内置默认资源库

**目标**：提供开箱即用的默认资源，确保仅配置文本 AI 即可完整体验

**具体任务**：
- 创建 `builtin-assets/` 目录结构
- 按游戏类型放置默认场景图（visual_novel/mystery/horror/rpg/simulation），每类至少 3 张
- 默认 BGM：calm.mp3, tense.mp3, dark.mp3, happy.mp3, battle.mp3
- 默认 NPC 头像：male/ 和 female/ 各 5 张
- 默认音效：click.mp3, transition.mp3
- 在 Rust 侧实现 `BuiltinAssetRegistry`：按游戏类型和氛围匹配内置资源
- Tauri 配置中注册资源目录

**提交信息**：`feat: add builtin default assets and asset registry`

---

### 节点 04：Rust 配置管理

**目标**：实现 AI 服务配置的存储、加密、预设方案管理

**具体任务**：
- `src-tauri/src/config/manager.rs`：配置管理器（读写 config.json + secrets.enc）
- `src-tauri/src/config/encryption.rs`：AES-256-GCM 加密存储（machine-id 密钥派生）
- `src-tauri/src/config/presets/`：4 个预设方案定义（zero_cost, text_only, default, minimal）
- `src-tauri/src/config/providers/mod.rs`：BUILTIN_PROVIDERS 完整定义
- `src-tauri/src/config/providers/connectivity.rs`：连通性检测逻辑
- 配置导出/导入（脱敏处理）
- 单元测试

**提交信息**：`feat: implement config manager with encryption, presets, and connectivity check`

---

### 节点 05：AI Provider Trait + 内置 Provider

**目标**：定义统一资源获取接口，实现内置默认资源 Provider

**具体任务**：
- `src-tauri/src/providers/mod.rs`：定义 `IAssetProvider` trait
  ```rust
  trait IAssetProvider: Send + Sync {
      async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError>;
      async fn check_connectivity(&self) -> Result<ConnectivityCheck, ProviderError>;
  }
  ```
- `src-tauri/src/providers/builtin.rs`：BuiltinAssetProvider 实现
  - 根据 asset_ref.type 和场景特征匹配内置资源
  - 复制到本地资源目录，返回 LocalAsset
- `src-tauri/src/engine/asset_manager.rs`：AssetManager
  - 统一资源存储路径管理
  - 缓存键生成与查找
  - 资源落盘（保存到本地 + 更新 AssetRef.status）

**提交信息**：`feat: implement IAssetProvider trait and BuiltinAssetProvider`

---

### 节点 06：文本 AI Provider（DeepSeek）

**目标**：实现 DeepSeek/OpenAI 兼容的文本生成 Provider

**具体任务**：
- `src-tauri/src/providers/deepseek.rs`：DeepSeekProvider
  - OpenAI 兼容 API 调用（chat/completions）
  - 流式输出支持
  - System Prompt + User Prompt 构造
  - 错误处理 + 重试逻辑
- `src-tauri/src/providers/mod.rs`：注册 DeepSeek 为文本模态 Provider
- 连通性检测：发送 "hi" 检查响应
- 单元测试（mock HTTP）

**提交信息**：`feat: implement DeepSeek text AI provider with streaming support`

---

### 节点 07：大纲解析器

**目标**：将玩家输入的自由文本解析为结构化 GameScript JSON

**具体任务**：
- `src-tauri/src/engine/outline_parser.rs`：OutlineParser
  - 输入长度判断：短文本先扩展再解析，长文本直接解析
  - Prompt 模板管理（从 prompts/outline-parser/ 加载）
  - 调用文本 AI 生成 GameScript JSON
  - JSON 校验：确保输出符合 GameScript 类型定义
  - 自动修复：补充缺失字段、生成 ID
- `prompts/outline-parser/`：Prompt 模板文件
  - expand.md：简短输入扩展 Prompt
  - parse.md：大纲解析 Prompt
  - combined.md：合并单次调用 Prompt
- 集成测试：用示例大纲验证解析结果

**提交信息**：`feat: implement outline parser with AI-powered GameScript generation`

---

### 节点 08：统一生成管线 + 资源管理器

**目标**：实现完整的生成调度管线，串联大纲解析 → 资源解析 → 来源判定 → 并行获取 → 落盘

**具体任务**：
- `src-tauri/src/engine/pipeline.rs`：GenerationPipeline
  - `create_game(input, game_type)`：完整生成流程
  - `resolve_provider(modality)`：根据配置选择 AI/Builtin Provider
  - `extract_asset_refs(script)`：从 GameScript 提取所有 AssetRef
  - `resolve_sources(asset_refs)`：为每个 AssetRef 确定来源
  - `schedule_generation(tasks)`：生成任务调度（优先级排序、并行执行）
  - `on_asset_ready(asset_ref)`：资源就绪回调 → Tauri 事件通知前端
  - `on_asset_failed(asset_ref, error)`：失败处理（重试/降级）
- `src-tauri/src/engine/asset_manager.rs`：完善 AssetManager
  - 游戏资源目录创建
  - 跨游戏缓存共享
- Tauri 事件发射：generation-progress, asset-ready, generation-complete, generation-failed

**提交信息**：`feat: implement unified generation pipeline with task scheduling`

---

### 节点 09：Tauri IPC 命令注册

**目标**：注册所有 IPC 命令，前端可调用后端功能

**具体任务**：
- `src-tauri/src/commands/game.rs`：
  - create_game, random_outline, get_game, get_game_script, list_games, delete_game, save_game, load_save, list_saves
- `src-tauri/src/commands/generation.rs`：
  - get_generation_status, regenerate_asset, export_game
- `src-tauri/src/commands/config.rs`：
  - get_config, update_config, get_presets, apply_preset, get_providers, update_provider, check_provider, check_all_providers, export_config, import_config
- `src-tauri/src/commands/asset.rs`：
  - get_asset_path, list_builtin_assets
- `src-tauri/src/main.rs`：注册所有命令到 Tauri Builder
- 前端 `src/hooks/`：封装 invoke 调用
  - useGame.ts, useGeneration.ts, useConfig.ts

**提交信息**：`feat: register all Tauri IPC commands and frontend hooks`

---

### 节点 10：前端 — 主菜单 + 创建游戏页

**目标**：实现应用入口页面和大纲输入界面

**具体任务**：
- `src/App.tsx`：路由配置（React Router）
- `src/pages/MainMenu.tsx`：主菜单（新游戏/继续游戏/游戏列表/设置）
- `src/pages/CreateGame.tsx`：
  - 大纲文本输入框（多行，支持自由输入）
  - 游戏类型选择（可选，AI 自动推断）
  - 随机生成按钮
  - 模板填充（可选）
  - 提交创建 → 调用 create_game
- 基础样式框架（CSS Modules / Tailwind）

**提交信息**：`feat: implement main menu and game creation page`

---

### 节点 11：前端 — AI 配置管理页

**目标**：实现 AI 服务配置的完整 UI

**具体任务**：
- `src/pages/Settings.tsx`：设置主页面
- `src/components/Config/`：
  - PresetSelector.tsx：预设方案选择卡片
  - ProviderCard.tsx：单个服务商状态卡片（连接状态/模型选择）
  - ProviderConfigModal.tsx：服务商配置弹窗（API Key 输入/显示隐藏/获取链接/检测连接）
  - ModalitySection.tsx：按模态分组的服务商列表
  - ConnectivityBadge.tsx：连通状态徽章
- 调用 useConfig hook 实现完整交互
- 导出/导入配置按钮

**提交信息**：`feat: implement AI provider configuration management UI`

---

### 节点 12：前端 — 生成进度页

**目标**：展示游戏资源生成进度，支持第一章就绪后开始游玩

**具体任务**：
- `src/pages/GenerationProgress.tsx`：
  - 章节进度卡片（进度条 + 资源状态列表）
  - 第一章就绪后显示"开始游玩"按钮
  - 整体进度概览
- 监听 Tauri 事件：generation-progress, asset-ready, generation-complete, generation-failed
- useGeneration hook 集成

**提交信息**：`feat: implement generation progress page with real-time event listening`

---

### 节点 13：前端 — 场景渲染器

**目标**：实现游戏场景的视觉呈现组件

**具体任务**：
- `src/components/Scene/SceneRenderer.tsx`：场景容器
  - 背景图展示（PixiJS 渲染或 CSS）
  - 背景视频播放
  - 转场动画（fade/dissolve/slide/instant）
- `src/components/Dialogue/DialogueBox.tsx`：
  - NPC 头像 + 名称 + 对话文本
  - 打字机效果
  - 情感标注（可选视觉反馈）
- `src/components/Dialogue/NarrationBox.tsx`：旁白文本展示
- `src/components/Choice/ChoicePanel.tsx`：
  - 选项按钮列表
  - 条件选项（灰色/隐藏不可选项）
  - 选择后的视觉反馈
- `src/components/CG/CGPlayer.tsx`：CG 视频播放器（可跳过控制）

**提交信息**：`feat: implement scene renderer, dialogue, choice, and CG components`

---

### 节点 14：前端 — 游戏引擎核心

**目标**：实现前端游戏逻辑引擎

**具体任务**：
- `src/engine/SceneExecutor.ts`：
  - 按 sequence 顺序执行 SceneNode
  - 分支跳转逻辑（ChoiceNode → nextNodeId）
  - 条件判断（ConditionNode）
  - 动作执行（ActionNode → set_variable/add_item/change_scene）
  - 场景切换（SceneTransitionNode）
  - CG 播放控制
- `src/engine/StateManager.ts`：
  - 全局/章节变量管理
  - 物品栏管理
  - 角色属性管理
  - 选择历史记录
- `src/engine/AssetLoader.ts`：
  - 根据 AssetRef 加载本地资源
  - 资源预加载
  - 加载失败降级处理
- `src/engine/AudioEngine.ts`：
  - BGM 播放/切换/淡入淡出
  - 语音播放
  - 音效播放
  - 基于 Howler.js
- 存档/读档逻辑（调用 IPC save_game/load_save）

**提交信息**：`feat: implement frontend game engine with scene executor and state manager`

---

### 节点 15：前端 — 游戏主界面整合

**目标**：整合所有游戏组件为完整的 GamePlay 页面

**具体任务**：
- `src/pages/GamePlay.tsx`：
  - 整合 SceneRenderer + DialogueBox + ChoicePanel + CGPlayer
  - 顶部 HUD：章节标题 + 菜单/存档/设置按钮
  - 底部工具栏：物品栏/状态/CG回廊
  - 点击/空格推进对话
  - 自动播放模式（可选）
- `src/components/HUD/`：
  - InventoryPanel.tsx：物品栏
  - StatsPanel.tsx：角色属性
  - GameMenu.tsx：游戏内菜单（存档/读档/返回主菜单）
- 存档弹窗 UI
- 完整游玩流程验证：从创建到游玩到存档

**提交信息**：`feat: integrate gameplay page with all game components and save system`

---

### 节点 16：Edge TTS Provider

**目标**：实现完全免费的语音生成 Provider

**具体任务**：
- `src-tauri/src/providers/edge_tts.rs`：EdgeTTSProvider
  - WebSocket 连接到 Edge TTS 服务
  - SSML 构造（voice_id, rate, pitch）
  - 音频数据接收并保存为本地文件
  - 支持中文多方言音色选择
  - 无需 API Key，默认 connected 状态
- 语音角色分配逻辑：NPC → voice_id 映射
- 集成到生成管线

**提交信息**：`feat: implement Edge TTS provider for free voice generation`

---

### 节点 17：图片 AI Provider（硅基流动 FLUX）

**目标**：实现 AI 图片生成

**具体任务**：
- `src-tauri/src/providers/siliconflow.rs`：SiliconFlowProvider
  - 图片生成 API 调用（FLUX.1-schnell / FLUX.1-dev）
  - Prompt 构造：场景图 / NPC 头像 / CG 静帧
  - 风格一致性：章节风格前缀
  - 图片下载保存到本地
  - 尺寸选择（16:9 背景 / 1:1 头像）
- 连通性检测：生成 64x64 最小图片
- 集成到生成管线

**提交信息**：`feat: implement SiliconFlow FLUX image generation provider`

---

### 节点 18：音乐 AI Provider（天工音乐）

**目标**：实现 AI BGM 生成

**具体任务**：
- `src-tauri/src/providers/skymusic.rs`：SkyMusicProvider
  - 音乐生成 API 调用
  - Prompt 构造：BGM / 氛围音乐
  - 异步轮询等待生成完成
  - 音频文件下载保存
- 音乐一致性：章节主题旋律关键词共享
- 集成到生成管线

**提交信息**：`feat: implement SkyMusic AI BGM generation provider`

---

### 节点 19：视频 AI Provider（可灵）

**目标**：实现 AI CG 视频生成

**具体任务**：
- `src-tauri/src/providers/kling.rs`：KlingProvider
  - 视频生成 API 调用（可灵 3.0）
  - Prompt 构造：CG 过场 / 动态背景
  - 异步轮询等待生成完成（30s-5min）
  - 视频文件下载保存
  - 降级策略：视频失败时用静态图 + Ken Burns 效果
- 集成到生成管线

**提交信息**：`feat: implement Kling AI video generation provider`

---

### 节点 20：渐进式加载 + 热替换

**目标**：实现边生成边游玩，AI 资源就绪后实时替换内置默认资源

**具体任务**：
- 生成管线：第一章优先生成，后续章节后台预生成
- 前端监听 asset-ready 事件，热替换当前展示资源
- 内置默认资源兜底展示逻辑
- 资源加载状态指示器（生成中/已就绪/使用默认）
- 重新生成按钮（调用 regenerate_asset）

**提交信息**：`feat: implement progressive loading and hot-swap for AI-generated assets`

---

### 节点 21：讯飞星火 Lite Provider

**目标**：实现零成本文本 AI Provider

**具体任务**：
- `src-tauri/src/providers/xfyun_spark.rs`：XfyunSparkProvider
  - WebSocket 连接讯飞星火 API
  - 鉴权签名（APPID + APISecret + APIKey）
  - 流式响应解析
  - 永久免费模式适配
- 集成到零成本预设方案

**提交信息**：`feat: implement Xfyun Spark Lite provider for zero-cost text generation`

---

### 节点 22：GameScript 校验器

**目标**：确保 AI 生成的 GameScript 结构正确、无死链

**具体任务**：
- 节点可达性检查：从 sequence[0] 出发，所有节点是否可达
- 死链检测：nextNodeId 引用不存在的节点
- 必要字段完整性检查
- 自动修复：为死链添加兜底跳转
- 生成校验报告

**提交信息**：`feat: implement GameScript validator with dead-link detection and auto-fix`

---

### 节点 23：随机大纲生成

**目标**：按游戏类型随机生成游戏大纲

**具体任务**：
- 实现 random_outline IPC 命令
- 按游戏类型构造 Prompt（含主题/氛围/角色模板）
- 前端：创建游戏页的"随机生成"按钮交互
- 生成结果预览 + 确认

**提交信息**：`feat: implement random outline generation by game type`

---

### 节点 24：重新生成 + 多候选

**目标**：支持对单个资源重新生成，关键资源提供多候选选择

**具体任务**：
- regenerate_asset 命令完善
- 多候选生成：关键资源一次生成 2-3 个候选
- 前端：资源右键/长按菜单 → 重新生成
- 候选选择弹窗 UI
- Prompt 微调编辑器（简易版）

**提交信息**：`feat: implement asset regeneration and multi-candidate selection`

---

### 节点 25：CG 回廊 + 资源导出

**目标**：浏览已解锁 CG，导出游戏资源包

**具体任务**：
- `src/components/CG/CGGallery.tsx`：CG 浏览界面
- 游戏资源包导出：GameScript + 所有资源文件打包为 zip
- 前端 CG 回廊入口
- 导出进度提示

**提交信息**：`feat: implement CG gallery and game resource export`

---

### 节点 26：备选 Provider 集成

**目标**：集成所有备选 AI 服务商

**具体任务**：
- tongyi.rs：通义千问（文本+图片+视频）
- zhipu.rs：智谱 GLM-4（文本+图片）
- jimeng.rs：即梦（图片+视频）
- vidu.rs：Vidu 2.0（视频）
- hailuo.rs：海螺AI/MiniMax（视频）
- volcengine_tts.rs：火山引擎 TTS
- xfyun_tts.rs：讯飞 TTS
- netease_music.rs：网易天音
- 各 Provider 连通性检测
- 注册到 Provider 工厂

**提交信息**：`feat: integrate all alternative AI providers (Tongyi, Zhipu, Jimeng, Vidu, etc.)`

---

### 节点 27：Web 端适配

**目标**：支持浏览器端运行

**具体任务**：
- IPC 适配层：检测运行环境，Web 端走前端 HTTP 请求
- IndexedDB / OPFS 资源存储
- 前端直接调用 AI API（需处理 CORS）
- 资源管理器 Web 适配
- 构建配置：Vite SPA 模式

**提交信息**：`feat: adapt for web platform with IndexedDB and frontend AI calls`

---

## 开发阶段划分

### Phase 1 — MVP（节点 01-15）
核心管线打通：从输入大纲到单章节完整游玩，仅依赖文本 AI + 内置默认资源。

### Phase 2 — 多模态 AI（节点 16-20）
集成图片/音乐/语音/视频 AI，实现渐进式加载和热替换。

### Phase 3 — 体验优化（节点 21-25）
零成本方案、校验器、随机生成、重新生成、CG 回廊。

### Phase 4 — 高级功能（节点 26-27）
备选 Provider 全覆盖、Web 端适配。

---

## AI 编程注意事项

1. **每个节点独立提交**：确保每个节点完成后代码可编译运行
2. **类型先行**：节点 02 的类型定义是后续所有节点的基础
3. **Mock 友好**：AI Provider 应支持 mock 模式，方便前端开发时不依赖真实 API
4. **渐进增强**：先实现 BuiltinAssetProvider 跑通全流程，再逐步添加 AI Provider
5. **事件驱动**：生成管线通过 Tauri 事件通知前端，前端不应轮询
6. **错误降级**：任何 AI 调用失败都应优雅降级到内置默认资源
