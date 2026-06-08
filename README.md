# auto-free

freestyle，梦到啥写啥

## 前置依赖

- **Node.js** (v18+)
- **Rust** (用于 Tauri 构建)

### 安装 Rust (Windows)

```powershell
# 方法一：使用 PowerShell 下载安装器
iwr https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe -OutFile rustup-init.exe
.\rustup-init.exe -y

# 安装完成后更新 PATH
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```

## 安装项目依赖

```bash
npm install
```

## 开发模式

```bash
npm run tauri dev
```

## 构建生产版本

```bash
npm run tauri build
```

## 项目结构

- `src/` - 前端源代码
- `src-tauri/` - Tauri Rust 后端代码
- `public/` - 静态资源