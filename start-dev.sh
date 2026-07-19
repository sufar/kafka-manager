#!/bin/bash

# Kafka Manager (GPUI) 开发启动脚本

echo "Starting Kafka Manager (GPUI)..."

# 获取脚本所在目录作为项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed. Please install Rust."
    exit 1
fi

# 业务逻辑内嵌于应用（kafka-manager-api 库），无需单独启动后端
cargo run -p kafka-manager-app
