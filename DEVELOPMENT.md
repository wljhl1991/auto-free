# AutoFree 开发指南

## 环境准备

1. **Node.js** (建议 18+)
2. **Rust** (通过 [rustup](https://rustup.rs/) 安装)
3. **Tauri v2 依赖** — Windows 上需要：
   - [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   - [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (Windows 10/11 通常已内置)

安装前端依赖：

```bash
npm install
```

## 开发模式

```bash
# 启动 Tauri 开发模式（同时启动 Vite 前端 + Rust 后端热重载）
npm run tauri dev
```

该命令会：
1. 执行 `npm run dev` 启动 Vite 开发服务器（端口 1420）
2. 编译并运行 Rust 后端
3. 打开桌面窗口

## 测试

### Rust 后端测试

项目后端包含单元测试（位于 `src-tauri/src/` 下的 `engine/asset_manager.rs` 和 `config/encryption.rs`）。

```bash
# 运行所有 Rust 测试
cd src-tauri
cargo test

# 运行特定模块的测试
cargo test --lib engine::asset_manager
cargo test --lib config::encryption

# 查看测试输出（println!）
cargo test -- --nocapture
```

### 前端测试

项目当前未配置前端测试框架。如需添加，推荐步骤：

```bash
# 安装 Vitest
npm install -D vitest @testing-library/react @testing-library/jest-dom
```

然后在 `package.json` 中添加脚本：

```json
"test": "vitest run",
"test:watch": "vitest"
```

## 打包构建

### 构建发布版本

```bash
npm run tauri build
```

该命令会：
1. 执行 `npm run build`（即 `tsc && vite build`）编译前端到 `dist/` 目录
2. 编译 Rust 后端（release 模式）
3. 生成安装包

### 构建产物位置

打包产物位于 `src-tauri/target/release/bundle/`，按平台不同包含：

| 平台 | 产物路径 |
|------|---------|
| Windows | `src-tauri/target/release/bundle/msi/*.msi` |
| Windows | `src-tauri/target/release/bundle/nsis/*.exe` |

### 仅构建前端

```bash
npm run build
```

产物输出到 `dist/` 目录。

### 仅构建 Rust 后端

```bash
cd src-tauri
cargo build --release
```

可执行文件位于 `src-tauri/target/release/autofree.exe`。

## 项目结构

```
autoFree/
├── src/                  # 前端源码 (React + TypeScript)
│   ├── components/       # UI 组件
│   ├── engine/           # 前端引擎（音频、场景、状态管理）
│   ├── hooks/            # React Hooks
│   ├── pages/            # 页面组件
│   └── main.tsx          # 入口
├── src-tauri/            # 后端源码 (Rust + Tauri)
│   ├── src/
│   │   ├── commands/     # Tauri 命令（前后端通信接口）
│   │   ├── config/       # 配置管理
│   │   ├── engine/       # 游戏引擎核心
│   │   ├── providers/    # AI 服务提供商
│   │   └── types/        # 类型定义
│   ├── Cargo.toml
│   └── tauri.conf.json   # Tauri 配置
├── builtin-assets/       # 内置资源（图片、音乐、音效）
├── prompts/              # AI 提示词模板
├── shared/types/         # 前后端共享类型定义
└── package.json
```

## 常用命令速查

| 命令 | 说明 |
|------|------|
| `npm install` | 安装前端依赖 |
| `npm run tauri dev` | 开发模式 |
| `cd src-tauri && cargo test` | 运行 Rust 测试 |
| `npm run tauri build` | 打包发布版本 |
| `npm run build` | 仅构建前端 |
