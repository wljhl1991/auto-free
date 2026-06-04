# SOLO.md — autoFree 项目描述

> 本文件供 Trae SOLO（AI 编程助手）理解项目上下文、架构决策和开发规范。

## 项目简介

autoFree 是一款 AI 驱动的生成式游戏引擎。玩家输入文字描述的游戏大纲（或由系统随机生成），引擎自动调用国内 AI 模型，生成完整的游戏章节内容——包括场景、对话、选项分支、BGM、NPC 语音、CG 动画等。每个生成的章节可串联为多章节游戏，支持视觉小说、RPG、悬疑解谜、恐怖生存、模拟经营等多种类型。

**核心理念**：资源本地化、开箱即用、统一代码路径、渐进增强。最少只需 1 个文本 AI 即可完整游玩。

## 当前进展

**27 个开发节点全部完成**，已推送到 GitHub。包括：
- Phase 1 MVP（节点 01-15）：核心管线打通，可完整游玩
- Phase 2（节点 16-20）：Edge TTS / 硅基流动 / 天工音乐 / 可灵视频 / 渐进式加载
- Phase 3（节点 21-27）：讯飞星火 / 校验器 / 随机大纲 / 多候选 / CG回廊 / 备选Provider / Web适配

当前状态：**功能开发完成，进入测试和打磨阶段**。

## 技术栈

| 层 | 技术 |
|---|---|
| 客户端框架 | Tauri v2（Rust 后端 + Web 前端） |
| 前端 | React + TypeScript + Vite |
| 音频 | Howler.js |
| 后端 | Rust（Tauri 进程内，无独立后端服务） |
| HTTP 客户端 | reqwest |
| 异步运行时 | tokio |
| JSON | serde_json |
| 加密 | aes-gcm（AES-256-GCM） |
| WebSocket | tokio-tungstenite（Edge TTS / 讯飞星火） |
| 签名 | hmac + sha2（可灵 API） |
| Web 端适配 | 前端适配层（src/adapters/tauri.ts），非 Tauri 环境降级为 HTTP API |

## 项目结构

```
autoFree/
├── src-tauri/                  # Tauri Rust 后端
│   ├── src/
│   │   ├── lib.rs              # Tauri 入口，注册 IPC 命令和状态
│   │   ├── commands/           # Tauri IPC 命令
│   │   │   ├── game.rs         # 游戏管理（创建/存档/读档）
│   │   │   ├── generation.rs   # 生成调度（状态/重新生成/导出）
│   │   │   ├── config.rs       # 配置管理（预设/服务商/连通性）
│   │   │   └── asset.rs        # 资源管理
│   │   ├── engine/             # 生成引擎
│   │   │   ├── outline_parser.rs  # 大纲→GameScript 解析
│   │   │   ├── pipeline.rs     # 统一生成管线（渐进式加载）
│   │   │   ├── asset_manager.rs   # 资源文件管理
│   │   │   └── validator.rs    # GameScript 校验器（死链检测/自动修复）
│   │   ├── providers/          # AI 模型适配器（统一 IAssetProvider trait）
│   │   │   ├── mod.rs          # IAssetProvider trait + ProviderFactory
│   │   │   ├── builtin.rs      # 内置默认资源
│   │   │   ├── deepseek.rs     # DeepSeek 文本（OpenAI 兼容格式）
│   │   │   ├── xfyun_spark.rs  # 讯飞星火 Lite（WebSocket）
│   │   │   ├── qwen.rs         # 通义千问（OpenAI 兼容格式）
│   │   │   ├── zhipu.rs        # 智谱 GLM（OpenAI 兼容格式）
│   │   │   ├── edge_tts.rs     # Edge TTS 语音（WebSocket，免费）
│   │   │   ├── volcengine_tts.rs # 火山引擎 TTS
│   │   │   ├── siliconflow.rs  # 硅基流动 FLUX 图片
│   │   │   ├── hailuo.rs       # 海螺 AI 图片
│   │   │   ├── jimeng.rs       # 即梦图片
│   │   │   ├── kling.rs        # 可灵视频（HMAC 签名认证）
│   │   │   ├── vidu.rs         # Vidu 视频
│   │   │   ├── skymusic.rs     # 天工音乐 BGM
│   │   │   └── netease_music.rs # 网易天音音乐
│   │   ├── config/             # 配置管理
│   │   │   ├── manager.rs      # ConfigManager（加载/保存/脱敏/还原）
│   │   │   ├── encryption.rs   # AES-256-GCM 加密
│   │   │   ├── presets/        # 预设方案定义
│   │   │   └── providers/      # 内置服务商定义 + 连通性检测
│   │   └── types/              # Rust 类型定义
│   │       ├── game_script.rs  # GameScript / Chapter / Scene / SceneNode
│   │       ├── game_state.rs   # GameState / SaveData
│   │       ├── asset.rs        # LocalAsset / AIModality
│   │       └── ai_provider.rs  # AIProviderConfig / AppConfig / Preset
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                        # 前端 React 应用
│   ├── adapters/
│   │   └── tauri.ts            # Tauri/Web 双端适配层
│   ├── components/             # UI 组件
│   │   ├── Scene/              # SceneRenderer
│   │   ├── Dialogue/           # DialogueBox / NarrationBox
│   │   ├── Choice/             # ChoicePanel / CandidateSelector
│   │   ├── CG/                 # CGPlayer / CGGallery
│   │   ├── HUD/                # GameMenu / InventoryPanel / StatsPanel
│   │   └── Config/             # PresetSelector / ModalitySection / ProviderConfigModal / PromptEditor
│   ├── engine/                 # 前端游戏引擎
│   │   ├── SceneExecutor.ts    # 场景执行器（事件驱动）
│   │   ├── StateManager.ts     # 游戏状态管理
│   │   ├── AssetLoader.ts      # 资源加载器
│   │   └── AudioEngine.ts      # 音频引擎（Howler.js）
│   ├── pages/                  # 页面
│   │   ├── MainMenu.tsx
│   │   ├── CreateGame.tsx
│   │   ├── GenerationProgress.tsx
│   │   ├── GamePlay.tsx
│   │   └── Settings.tsx
│   ├── hooks/                  # Tauri IPC 封装
│   │   ├── useGame.ts
│   │   ├── useGeneration.ts
│   │   └── useConfig.ts
│   ├── types/                  # TypeScript 类型定义
│   │   └── index.ts
│   ├── App.tsx
│   └── index.css
├── builtin-assets/             # 内置默认资源（随应用分发）
├── prompts/                    # Prompt 模板库
│   └── outline-parser/         # 大纲解析 Prompt（expand.md / parse.md / combined.md / random.md）
├── GAME_DESIGN.md              # 完整设计文档
├── DEVELOPMENT_PLAN.md         # 开发计划与提交节点
└── package.json
```

## 核心架构概念

### 统一代码路径

无论玩家配置了 1 个 AI 还是 5 个 AI，底层生成管线走相同代码逻辑。关键接口：

```rust
trait IAssetProvider: Send + Sync {
    async fn get_asset(&self, asset_ref: &AssetRef) -> Result<LocalAsset, ProviderError>;
    async fn check_connectivity(&self) -> Result<ConnectivityCheck, ProviderError>;
}
```

- AI Provider：调用配置的 AI 模型生成资源
- `BuiltinAssetProvider`：使用内置默认资源（降级方案）

`GenerationPipeline.resolve_provider(modality)` 根据配置选择 Provider，但调用接口统一。

### 渐进增强

| 配置级别 | 场景图片 | BGM | NPC 语音 | CG 视频 | 交互逻辑 |
|---------|---------|-----|---------|--------|---------|
| 零成本（星火Lite+EdgeTTS） | 内置默认 | 内置默认 | Edge TTS | 静态图+文字 | 星火 Lite |
| 仅文本 AI | 内置默认 | 内置默认 | Edge TTS | 静态图+文字 | DeepSeek |
| +图片 AI | AI 生成 | 内置默认 | Edge TTS | 静态图+文字 | DeepSeek |
| 全量配置 | AI 生成 | AI 生成 | AI 生成 | AI 生成 | DeepSeek |

**交互逻辑始终由文本 AI 生成，这是游戏可玩性的根基。** 其他模态只影响视听体验。

### 无 AI 配置也可游玩

即使未配置任何 AI 服务，也能创建游戏——使用本地模板生成基础 GameScript，所有资源使用内置默认。确保开箱即用。

### GameScript 核心数据结构

大纲经 AI 解析后生成 `GameScript`，是引擎运行的核心数据：

- `GameScript` → `Chapter[]` → `Scene[]` → `SceneNode[]`
- SceneNode 类型：Narration / Dialogue / Choice / Condition / Action / CG / SceneTransition
- 所有需要生成的资源通过 `AssetRef` 引用，包含 prompt、来源（ai_generated/builtin/local_file）、状态（pending/generating/ready/failed/fallback）

### 生成管线流程

1. 玩家输入大纲 → 文本 AI 解析为 GameScript JSON（无 AI 时用本地模板）
2. 提取所有 AssetRef → 为每个确定来源（AI 已配置则生成，否则用内置默认）
3. 并行获取资源（AI 生成 + 内置资源复制）
4. 统一存入本地 AssetManager
5. 第一章就绪即可开始游玩，后续章节后台预生成
6. AI 资源就绪后热替换（淡入淡出动画）

### 前后端通信

- **桌面端**：Tauri IPC（`invoke()` 调用 Rust 命令 + `listen()` 接收事件）
- **Web 端**：通过适配层（`src/adapters/tauri.ts`）降级为 HTTP API 调用
- 事件：generation-progress / asset-ready / generation-complete / generation-failed

## AI 服务配置

### 已集成 Provider（14 个）

| 模态 | Provider | 认证方式 | 备注 |
|------|----------|---------|------|
| 文本 | DeepSeek | API Key | OpenAI 兼容格式，支持流式 |
| 文本 | 讯飞星火 Lite | WebSocket + HMAC 签名 | 永久免费 |
| 文本 | 通义千问 | API Key | OpenAI 兼容格式 |
| 文本 | 智谱 GLM | API Key | OpenAI 兼容格式 |
| 图片 | 硅基流动 FLUX | API Key | 支持 negative_prompt |
| 图片 | 海螺 AI | API Key | OpenAI 兼容格式 |
| 图片 | 即梦 | API Key | OpenAI 兼容格式 |
| 视频 | 可灵 | Access Key + Secret Key | HMAC-SHA256 签名，异步轮询 |
| 视频 | Vidu | API Key | 异步轮询 |
| 音乐 | 天工音乐 | API Key | 异步轮询 |
| 音乐 | 网易天音 | API Key | 异步轮询 |
| 语音 | Edge TTS | 无需认证（WebSocket） | 免费，8 种中文音色 |
| 语音 | 火山引擎 TTS | AppID + Access Token | - |

### 预设方案

| 预设 | 厂商数 | 文本 | 图片 | 视频 | 音乐 | 语音 |
|-----|-------|------|------|------|------|------|
| 零成本 | 0 | 讯飞星火Lite（永久免费） | 内置默认 | 内置默认 | 内置默认 | Edge TTS（免费） |
| 仅文本 | 1 | DeepSeek | 内置默认 | 内置默认 | 内置默认 | Edge TTS |
| 极简 | 2 | 硅基流动(DeepSeek) | 硅基流动(FLUX) | 内置默认 | 天工音乐 | Edge TTS |
| 默认推荐 | 4 | DeepSeek | 硅基流动(FLUX) | 可灵3.0 | 天工音乐 | Edge TTS |

### 配置存储

- 非敏感配置 → `config.json`（明文）
- API Key 等敏感信息 → `secrets.enc`（AES-256-GCM 加密，默认 machine-id 密钥派生）

## 开发规范

### 编码约定

- **类型先行**：`src/types/` 中的 TypeScript 类型是前端的基础，新增功能先定义类型
- **Rust 侧**：所有 struct 使用 `serde::{Serialize, Deserialize}`，IPC 命令使用 `#[tauri::command]`
- **前端**：组件使用函数式组件 + Hooks，Tauri 调用封装在 `src/hooks/`，通过 `src/adapters/tauri.ts` 适配层确保双端兼容
- **Hooks 稳定性**：所有自定义 Hook 必须用 `useMemo` 包裹返回对象，避免无限重渲染
- **错误降级**：任何 AI 调用失败都应优雅降级到内置默认资源，不应阻塞游戏流程
- **事件驱动**：生成管线通过 Tauri 事件通知前端，前端不应轮询
- **Mock 友好**：AI Provider 应支持 mock 模式，方便前端开发时不依赖真实 API

### 提交规范

提交信息使用**中文**，遵循 Conventional Commits 格式：

```
<类型>(<范围>): <简短描述>
```

#### 类型

| 类型 | 说明 |
|------|------|
| `feat` | 新功能 |
| `fix` | 修复缺陷 |
| `refactor` | 重构 |
| `docs` | 文档变更 |
| `style` | 代码风格调整 |
| `perf` | 性能优化 |
| `test` | 测试 |
| `chore` | 构建/工具/依赖变更 |

### 资源管理

- AI 生成的所有资源存储在本地 `~/autofree/games/{gameId}/assets/`
- 内置默认资源随应用分发，位于 `builtin-assets/`
- 跨游戏共享缓存：`~/autofree/cache/{cacheKey}`
- 所有资源归玩家所有，支持导出（zip 打包）

## 关键设计文档索引

| 文档 | 内容 |
|------|------|
| `GAME_DESIGN.md` | 完整设计文档：架构、GameScript 定义、AI 管线、配置系统、UI 设计、IPC 命令、Prompt 工程 |
| `DEVELOPMENT_PLAN.md` | 27 个开发节点的详细任务拆分和提交规范 |

开发时请先阅读这两个文档了解完整上下文。

## 常见开发任务指引

### 新增 AI Provider

1. 在 `src-tauri/src/providers/` 下创建新文件，实现 `IAssetProvider` trait
2. 在 `src-tauri/src/providers/mod.rs` 的 `ProviderFactory::create` 中添加分支
3. 在 `src-tauri/src/config/providers/` 中添加服务商定义
4. 添加连通性检测逻辑
5. 确保 `cargo check` 通过

### 新增前端页面/组件

1. 页面放 `src/pages/`，组件放 `src/components/`
2. 通过 `src/hooks/` 封装的 Hook 调用 Tauri IPC
3. 确保桌面端和 Web 端兼容（通过 `src/adapters/tauri.ts` 适配层）
4. Hook 返回对象必须用 `useMemo` 包裹

### 修改 GameScript 结构

1. 先更新 `src/types/index.ts` 中的 TypeScript 类型
2. 同步更新 Rust 侧 `src-tauri/src/types/game_script.rs` 中的 struct
3. 更新大纲解析器的 Prompt 模板（`prompts/outline-parser/`）
4. 更新 GameScript 校验器（`src-tauri/src/engine/validator.rs`）
5. 确保向后兼容（已有存档能正常加载）

### 编译验证

- Rust：`cd src-tauri && cargo check`
- 前端：`npx tsc --noEmit && npx vite build`
- 注意：运行 cargo 前需刷新 PATH：`$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")`
