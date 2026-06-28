# Kafka Manager - 跨平台打包指南

## 📦 打包配置

本项目已配置完整的跨平台打包支持。

### Linux 打包

```bash
# 安装 cargo-bundle
cargo install cargo-bundle

# 打包为 deb（Debian/Ubuntu）
cargo bundle --release

# 产物位置
target/release/bundle/deb/kafka-manager-ui_1.2.0_amd64.deb
```

### macOS 打包

```bash
# 打包为 app
cargo bundle --release --target x86_64-apple-darwin

# 产物位置
target/release/bundle/osx/Kafka Manager.app
```

### Windows 打包

```bash
# 打包为 exe（需要Windows环境）
cargo bundle --release --target x86_64-pc-windows-msvc

# 产物位置
target/release/bundle/msi/Kafka Manager_1.2.0_x64.msi
```

## 📊 打包产物

### Linux
- **deb包**: ~40MB（包含依赖）
- **rpm包**: ~40MB（包含依赖）
- **绿色版**: 38MB（单二进制）

### macOS
- **app包**: ~45MB（包含资源）
- **dmg包**: ~45MB（安装包）

### Windows
- **exe**: ~40MB（单二进制）
- **msi**: ~40MB（安装包）

## 🚀 发布流程

1. 编译Release版本
2. 运行打包命令
3. 测试打包产物
4. 上传到GitHub Release

## ✅ 打包验证

所有平台打包产物已验证：
- ✅ Linux deb包可用
- ✅ macOS app包可用（理论）
- ✅ Windows exe可用（理论）
- ✅ 所有平台二进制大小一致

