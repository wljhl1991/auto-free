# SOLO.md — autoFree 项目描述

> 本文件供 Trae SOLO（AI 编程助手）理解项目上下文、架构决策和开发规范。

## 项目简介

autoFree 是一款 AI 驱动的生成式游戏引擎。玩家输入文字描述的游戏大纲（或由系统随机生成），引擎自动调用国内 AI 模型，生成完整的游戏章节内容——包括场景、对话、选项分支、BGM、NPC 语音、CG 动画等。每个生成的章节可串联为多章节游戏，支持视觉小说、RPG、悬疑解谜、恐怖生存、模拟经营等多种类型。

**核心理念**：资源本地化、开箱即用、统一代码路径、渐进增强。最少只需 1 个文本 AI 即可完整游玩。

## 技术栈

| 层 | 技术 |
|---|---|
| 客户端框架 | Tauri（Rust 后端 + Web 前端） |
| 前端 | React + TypeScript + Vite |
| 状态管理 | Zustand |
| 场景渲染 | PixiJS |
| 音频 | Howler.js |
| 后端 | Rust（Tauri 进程内，无独立后端服务） |
| HTTP 客户端 | reqwest |
| 异步运行时 | tokio |
| 本地数据库 | rusqlite（SQLite） |
| JSON | serde_json |
| 加密 | aes-gcm（AES-256-GCM） |
| Web 端适配 | IndexedDB / OPFS，前端直接调 AI API |

## 项目结构

```
autoFree/
├── src-tauri/                  # Tauri Rust 后端
│   ├── src/
│   │   ├── main.rs             # Tauri 入口，注册 IPC 命令
│   │   ├── commands/           # Tauri IPC 命令
│   │   │   ├── game.rs         # 游戏管理
│   │   │   ├── generation.rs   # 生成调度
│   │   │   ├── config.rs       # 配置管理
│   │   │   └── asset.rs        # 资源管理
│   │   ├── engine/             # 生成引擎
│   │   │   ├── outline_parser.rs
│   │   │   ├── pipeline.rs
│   │   │   └── asset_manager.rs
│   │   ├── providers/          # AI 模型适配器（统一 IAssetProvider trait）
│   │   │   ├── mod.rs          # IAssetProvider trait
│   │   │   ├── builtin.rs      # 内置默认资源
│   │   │   ├── deepseek.rs
│   │   │   ├── xfyun_spark.rs
│   │   │   ├── edge_tts.rs
│   │   │   ├── siliconflow.rs
│   │   │   ├── kling.rs
│   │   │   └── ...             # 其他 Provider
│   │   ├── config/             # 配置管理
│   │   │   ├── manager.rs
│   │   │   ├── encryption.rs
│   │   │   ├── presets/
│   │   │   └── providers/
│   │   └── db/                 # SQLite 操作
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                        # 前端 React 应用
│   ├── components/             # UI 组件（Scene/Dialogue/Choice/CG/HUD/Config）
│   ├── engine/                 # 前端游戏引擎（SceneExecutor/StateManager/AssetLoader/AudioEngine）
│   ├── pages/                  # 页面（MainMenu/CreateGame/GenerationProgress/GamePlay/Settings）
│   ├── store/                  # Zustand 状态仓库
│   ├── hooks/                  # Tauri IPC 封装（useGame/useGeneration/useConfig）
│   └── App.tsx
├── shared/                     # 共享类型定义
│   └── types/                  # game-script.ts / game-state.ts / asset.ts / ai-provider.ts
├── builtin-assets/             # 内置默认资源（随应用分发）
├── prompts/                    # Prompt 模板库
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

- `AIAssetProvider`：调用配置的 AI 模型生成资源
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

### GameScript 核心数据结构

大纲经 AI 解析后生成 `GameScript`，是引擎运行的核心数据：

- `GameScript` → `Chapter[]` → `Scene[]` → `SceneNode[]`
- SceneNode 类型：Narration / Dialogue / Choice / Condition / Action / CG / SceneTransition
- 所有需要生成的资源通过 `AssetRef` 引用，包含 prompt、来源（ai_generated/builtin/local_file）、状态（pending/generating/ready/failed/fallback）

### 生成管线流程

1. 玩家输入大纲 → 文本 AI 解析为 GameScript JSON
2. 提取所有 AssetRef → 为每个确定来源（AI 已配置则生成，否则用内置默认）
3. 并行获取资源（AI 生成 + 内置资源复制）
4. 统一存入本地 AssetManager
5. 第一章就绪即可开始游玩，后续章节后台预生成

### 前后端通信

- **桌面端**：Tauri IPC（`invoke()` 调用 Rust 命令 + `listen()` 接收事件）
- **Web 端**：前端直接调 AI API（HTTP 请求），资源存 IndexedDB/OPFS
- 事件：generation-progress / asset-ready / generation-complete / generation-failed

## AI 服务配置

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

### 开发节奏

按 `DEVELOPMENT_PLAN.md` 中的 27 个节点顺序推进，每个节点独立提交，确保代码可编译运行。

当前阶段：**Phase 1 — MVP（节点 01-15）**，核心管线打通。

### 编码约定

- **类型先行**：`shared/types/` 中的 TypeScript 类型是前后端共享的基础，新增功能先定义类型
- **Rust 侧**：所有 struct 使用 `serde::{Serialize, Deserialize}`，IPC 命令使用 `#[tauri::command]`
- **前端**：组件使用函数式组件 + Hooks，状态管理用 Zustand，Tauri 调用封装在 `src/hooks/`
- **错误降级**：任何 AI 调用失败都应优雅降级到内置默认资源，不应阻塞游戏流程
- **事件驱动**：生成管线通过 Tauri 事件通知前端，前端不应轮询
- **Mock 友好**：AI Provider 应支持 mock 模式，方便前端开发时不依赖真实 API

### 提交规范

格式：`feat: <描述>` / `fix: <描述>` / `refactor: <描述>`

每个节点完成后提交，提交信息参考 `DEVELOPMENT_PLAN.md` 中各节点的建议提交信息。

### 资源管理

- AI 生成的所有资源存储在本地 `~/autofree/games/{gameId}/assets/`
- 内置默认资源随应用分发，位于 `builtin-assets/`
- 跨游戏共享缓存：`~/autofree/cache/{cacheKey}`
- 所有资源归玩家所有，支持导出

## 关键设计文档索引

| 文档 | 内容 |
|------|------|
| `GAME_DESIGN.md` | 完整设计文档：架构、GameScript 定义、AI 管线、配置系统、UI 设计、IPC 命令、Prompt 工程 |
| `DEVELOPMENT_PLAN.md` | 27 个开发节点的详细任务拆分和提交规范 |

开发时请先阅读这两个文档了解完整上下文。

## 常见开发任务指引

### 新增 AI Provider

1. 在 `src-tauri/src/providers/` 下创建新文件，实现 `IAssetProvider` trait
2. 在 `src-tauri/src/config/providers/mod.rs` 的 `BUILTIN_PROVIDERS` 中添加服务商定义
3. 在 `GenerationPipeline` 中注册新 Provider
4. 添加连通性检测逻辑
5. 编写单元测试（mock HTTP）

### 新增前端页面/组件

1. 页面放 `src/pages/`，组件放 `src/components/`
2. 通过 `src/hooks/` 封装的 Hook 调用 Tauri IPC
3. 状态管理用 Zustand（`src/store/`）
4. 确保桌面端和 Web 端兼容（通过 IPC 适配层）

### 修改 GameScript 结构

1. 先更新 `shared/types/game-script.ts` 中的 TypeScript 类型
2. 同步更新 Rust 侧 `src-tauri/src/types/` 中的 struct
3. 更新大纲解析器的 Prompt 模板（`prompts/outline-parser/`）
4. 更新 GameScript 校验器
5. 确保向后兼容（已有存档能正常加载）
