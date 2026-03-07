# Tauri 2 跨平台桌面应用

本项目已迁移到 Tauri 2，支持 Windows、macOS 和 Linux 桌面平台。

## 架构说明

当前采用 **Tauri + 独立后端** 的架构：
- **前端**: Vue 3 + Tauri 2
- **后端**: Rust + axum (独立进程)
- **通信**: HTTP (localhost:9732)

## 开发环境要求

1. **Rust** (1.70+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Node.js** (18+)
   ```bash
   # 使用 nvm 安装
   nvm install 18
   ```

3. **系统依赖**

   **Linux:**
   ```bash
   # Ubuntu/Debian
   sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
     libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev \
     libsoup2.4-dev libjavascriptcoregtk-4.0-dev libgtk-3-dev
   ```

   **macOS:**
   ```bash
   xcode-select --install
   ```

   **Windows:**
   - 安装 [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   - 安装 [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

## 开发模式

### 方式 1: 使用启动脚本（推荐）

```bash
chmod +x start-tauri-dev.sh
./start-tauri-dev.sh
```

### 方式 2: 手动启动

1. **启动后端** (终端 1):
   ```bash
   cd /data/github/coding/ai/kafka-manager
   cargo run
   ```

2. **启动 Tauri 开发模式** (终端 2):
   ```bash
   cd ui
   npm run tauri dev
   ```

## 构建发布版本

```bash
cd ui
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`

## 项目结构

```
kafka-manager/
├── src/                    # Rust 后端源码
├── ui/                     # Vue 前端源码
│   ├── src/
│   │   ├── api/           # API 客户端
│   │   ├── views/         # 视图组件
│   │   └── stores/        # Pinia 状态管理
│   └── dist/              # 构建产物
├── src-tauri/             # Tauri 配置和源码
│   ├── src/
│   │   └── lib.rs        # Tauri 入口
│   ├── icons/            # 应用图标
│   ├── tauri.conf.json   # Tauri 配置
│   └── Cargo.toml        # Tauri Rust 依赖
├── config.toml           # 后端配置
├── kafka_manager.db      # SQLite 数据库
└── start-tauri-dev.sh    # 开发启动脚本
```

## 下一步优化方向

### 当前架构（独立后端）
- 优点：代码改动最小，后端逻辑无需修改
- 缺点：需要运行两个进程

### 未来可优化为集成架构
- 将 axum 后端逻辑集成到 Tauri commands
- 使用 Tauri 的 IPC 通信替代 HTTP
- 单一进程，更轻量

## 故障排除

### 问题：Tauri 窗口无法加载
检查后端是否在 `http://localhost:9732` 运行

### 问题：构建失败
尝试清理缓存：
```bash
cd ui
rm -rf node_modules dist
npm install

cd ..
cargo clean
cargo build
```

### 问题：WebView2 错误 (Windows)
下载并安装 [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
