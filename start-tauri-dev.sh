#!/bin/bash

# Kafka Manager Tauri 开发启动脚本

echo "Starting Kafka Manager in Tauri dev mode..."

# 获取脚本所在目录作为项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 检查是否安装了必要的工具
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed. Please install Rust."
    exit 1
fi

if ! command -v npm &> /dev/null; then
    echo "Error: npm is not installed. Please install Node.js."
    exit 1
fi

# 启动后端
echo "Starting backend server..."
cargo run --release &
BACKEND_PID=$!

# 等待后端启动
sleep 3

# 启动 Tauri 开发模式
echo "Starting Tauri dev mode..."
cd ui
npm run tauri dev

# 清理
kill $BACKEND_PID 2>/dev/null
