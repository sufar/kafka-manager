# Windows 打包指南

本文档说明如何将 Kafka Manager 打包成 Windows 可执行安装文件。

## 前置要求

### 在 Windows 系统上打包（推荐）

1. **安装 Node.js 18+**
   - 下载地址：https://nodejs.org/
   - 建议使用 LTS 版本

2. **安装 Rust 工具链**
   ```powershell
   winget install Rustlang.Rustup
   # 或者访问 https://rustup.rs/ 下载
   ```

3. **安装 Visual Studio Build Tools**
   - 下载地址：https://visualstudio.microsoft.com/downloads/
   - 安装时选择 **"Desktop development with C++"** 工作负载
   - 这包含了 Windows 编译所需的 MSVC 编译器

4. **安装 Webview2**（Windows 10 1803+ 通常已内置）
   - 下载地址：https://developer.microsoft.com/en-us/microsoft-edge/webview2/

## 打包步骤

### 1. 准备项目

```powershell
# 克隆或复制项目到 Windows 机器
cd C:\path\to\kafka-manager

# 安装前端依赖
cd ui
npm install

# 构建前端
npm run build
```

### 2. 安装 Tauri CLI

```powershell
cd ..\src-tauri
cargo install tauri-cli
```

### 3. 构建 Windows 安装包

```powershell
# 返回项目根目录
cd ..

# 执行构建
cargo tauri build
```

### 4. 获取安装包

构建完成后，安装包位于：
- **NSIS 安装程序**: `src-tauri\target\release\bundle\nsis\Kafka Manager_0.1.0_x64-setup.exe`
- **独立 EXE**: `src-tauri\target\release\bundle\nsis\Kafka Manager_0.1.0_x64.exe`

## 在 macOS 上交叉编译到 Windows（不推荐）

虽然可以在 macOS 上交叉编译 Windows 版本，但过程复杂且容易遇到问题：

```bash
# 添加 Windows 目标
rustup target add x86_64-pc-windows-msvc

# 安装交叉编译工具
brew install llvm mingw-w64

# 构建（需要 Windows 资源编译器等复杂配置）
cargo tauri build --target x86_64-pc-windows-msvc
```

**强烈建议在 Windows 原生环境或虚拟机中进行打包。**

## 使用虚拟机打包

如果你没有 Windows 机器，可以使用：

1. **Parallels Desktop** (macOS)
2. **VMware Fusion** (macOS/Linux)
3. **VirtualBox** (免费)

在虚拟机中安装 Windows 10/11，然后按照上述步骤进行打包。

## 故障排除

### 构建失败：找不到 MSVC 编译器
- 确保安装了 Visual Studio Build Tools
- 确保选择了 "Desktop development with C++" 工作负载
- 重启终端后重试

### 构建失败：Webview2 相关错误
- 下载并安装 Webview2 Bootstrapper
- https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### 前端构建失败
```bash
# 清理缓存
rm -rf node_modules package-lock.json
npm install
npm run build
```

## 配置文件说明

### tauri.conf.json

```json
{
  "bundle": {
    "targets": ["nsis", "msi"],  // NSIS 和 MSI 安装程序
    "windows": {
      "nsis": {
        "oneClick": false,  // 允许用户选择安装目录
        "allowToChangeInstallDirectory": true
      }
    }
  }
}
```

## 安装包大小优化

如需减小安装包大小：

1. 使用 `--target` 指定特定架构
2. 启用代码分割和 Tree-shaking
3. 压缩资源文件

```bash
# 仅构建当前架构
cargo tauri build --target x86_64-pc-windows-msvc
```

## 签名（可选）

如果需要代码签名证书：

```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "YOUR_CERT_THUMBPRINT",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

## 测试安装包

在另一台干净的 Windows 虚拟机中测试安装包，确保：
- 安装程序正常运行
- 应用程序正常启动
- 所有功能正常工作
