#!/bin/bash

# Kafka Manager 服务器启动脚本
# 用于在无 Tauri 环境的服务器上同时启动后端和前端
# 用法：./start-server.sh [start|stop|restart|status]

COMMAND="${1:-start}"

echo "=========================================="
echo "  Kafka Manager 服务器管理脚本"
echo "=========================================="

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 默认端口
BACKEND_PORT="${BACKEND_PORT:-9732}"
FRONTEND_PORT="${FRONTEND_PORT:-9733}"

# PID 文件
BACKEND_PID_FILE="/tmp/kafka-manager-backend.pid"
FRONTEND_PID_FILE="/tmp/kafka-manager-frontend.pid"

# 清理函数
cleanup() {
    echo -e "\n${YELLOW}正在停止服务...${NC}"

    if [ -f "$BACKEND_PID_FILE" ]; then
        kill $(cat "$BACKEND_PID_FILE") 2>/dev/null
        rm -f "$BACKEND_PID_FILE"
        echo -e "${GREEN}后端服务已停止${NC}"
    fi

    if [ -f "$FRONTEND_PID_FILE" ]; then
        kill $(cat "$FRONTEND_PID_FILE") 2>/dev/null
        rm -f "$FRONTEND_PID_FILE"
        echo -e "${GREEN}前端服务已停止${NC}"
    fi

    exit 0
}

# 停止服务函数
stop_services() {
    echo -e "${YELLOW}正在停止服务...${NC}"

    if [ -f "$BACKEND_PID_FILE" ] && kill -0 $(cat "$BACKEND_PID_FILE") 2>/dev/null; then
        kill $(cat "$BACKEND_PID_FILE") 2>/dev/null
        rm -f "$BACKEND_PID_FILE"
        echo -e "${GREEN}后端服务已停止${NC}"
    else
        echo -e "${YELLOW}后端服务未运行${NC}"
    fi

    if [ -f "$FRONTEND_PID_FILE" ] && kill -0 $(cat "$FRONTEND_PID_FILE") 2>/dev/null; then
        kill $(cat "$FRONTEND_PID_FILE") 2>/dev/null
        rm -f "$FRONTEND_PID_FILE"
        echo -e "${GREEN}前端服务已停止${NC}"
    else
        echo -e "${YELLOW}前端服务未运行${NC}"
    fi
}

# 状态检查函数
check_status() {
    echo -e "\n${YELLOW}服务状态:${NC}"

    if [ -f "$BACKEND_PID_FILE" ] && kill -0 $(cat "$BACKEND_PID_FILE") 2>/dev/null; then
        echo -e "${GREEN}后端服务：运行中 (PID: $(cat "$BACKEND_PID_FILE"))${NC}"
    else
        echo -e "${RED}后端服务：未运行${NC}"
    fi

    if [ -f "$FRONTEND_PID_FILE" ] && kill -0 $(cat "$FRONTEND_PID_FILE") 2>/dev/null; then
        echo -e "${GREEN}前端服务：运行中 (PID: $(cat "$FRONTEND_PID_FILE"))${NC}"
    else
        echo -e "${RED}前端服务：未运行${NC}"
    fi
    echo ""
}

# 主逻辑
case "$COMMAND" in
    start)
        # 检查是否已运行
        if [ -f "$BACKEND_PID_FILE" ] && kill -0 $(cat "$BACKEND_PID_FILE") 2>/dev/null; then
            echo -e "${RED}错误：后端服务已在运行 (PID: $(cat "$BACKEND_PID_FILE"))${NC}"
            echo "如需重启，请先运行：$0 stop"
            exit 1
        fi

        if [ -f "$FRONTEND_PID_FILE" ] && kill -0 $(cat "$FRONTEND_PID_FILE") 2>/dev/null; then
            echo -e "${RED}错误：前端服务已在运行 (PID: $(cat "$FRONTEND_PID_FILE"))${NC}"
            echo "如需重启，请先运行：$0 stop"
            exit 1
        fi

        # 检查 Rust 环境
        if ! command -v cargo &> /dev/null; then
            echo -e "${RED}错误：未找到 cargo，请先安装 Rust${NC}"
            exit 1
        fi

        # 检查 Node.js 环境
        if ! command -v npm &> /dev/null; then
            echo -e "${RED}错误：未找到 npm，请先安装 Node.js${NC}"
            exit 1
        fi

        echo -e "${GREEN}环境检查通过${NC}"

        # 启动后端
        echo -e "\n${YELLOW}正在启动后端服务...${NC}"
        cd "$SCRIPT_DIR"
        nohup cargo run --release > /tmp/kafka-manager-backend.log 2>&1 &
        BACKEND_PID=$!
        echo $BACKEND_PID > "$BACKEND_PID_FILE"
        echo -e "${GREEN}后端服务已启动 (PID: $BACKEND_PID)${NC}"
        echo -e "日志文件：/tmp/kafka-manager-backend.log"

        # 等待后端启动
        echo "等待后端服务就绪..."
        for i in {1..30}; do
            if curl -s http://localhost:$BACKEND_PORT/health > /dev/null 2>&1; then
                echo -e "${GREEN}后端服务已就绪${NC}"
                break
            fi
            if [ $i -eq 30 ]; then
                echo -e "${YELLOW}后端服务启动中，请查看日志：/tmp/kafka-manager-backend.log${NC}"
            fi
            sleep 1
        done

        # 启动前端
        echo -e "\n${YELLOW}正在启动前端服务...${NC}"
        cd "$SCRIPT_DIR/ui"
        # 启动前端服务 (配置文件已设置 host 为 127.0.0.1 避免 PRoot 网络接口检测错误)
        nohup npm run dev -- --port $FRONTEND_PORT > /tmp/kafka-manager-frontend.log 2>&1 &
        FRONTEND_PID=$!
        echo $FRONTEND_PID > "$FRONTEND_PID_FILE"
        echo -e "${GREEN}前端服务已启动 (PID: $FRONTEND_PID)${NC}"
        echo -e "日志文件：/tmp/kafka-manager-frontend.log"

        # 等待前端启动
        sleep 3

        echo -e "\n=========================================="
        echo -e "${GREEN}Kafka Manager 启动完成！${NC}"
        echo "=========================================="
        echo -e "${GREEN}前端访问地址：http://localhost:$FRONTEND_PORT${NC}"
        echo -e "${GREEN}后端 API 地址：http://localhost:$BACKEND_PORT${NC}"
        echo -e "\n按 Ctrl+C 停止所有服务"
        echo -e "或运行：$0 stop"
        echo "==========================================\n"

        # 显示日志（可选）
        tail -f /tmp/kafka-manager-backend.log /tmp/kafka-manager-frontend.log 2>/dev/null &
        TAIL_PID=$!

        # 等待用户中断
        wait
        ;;

    stop)
        stop_services
        ;;

    restart)
        echo -e "${YELLOW}正在重启服务...${NC}"
        stop_services
        sleep 2
        echo ""
        $0 start
        ;;

    status)
        check_status
        ;;

    *)
        echo -e "${RED}用法：$0 {start|stop|restart|status}${NC}"
        echo ""
        echo "命令说明:"
        echo "  start   - 启动后端和前端服务"
        echo "  stop    - 停止所有服务"
        echo "  restart - 重启所有服务"
        echo "  status  - 查看服务运行状态"
        echo ""
        echo "环境变量:"
        echo "  BACKEND_PORT   - 后端端口 (默认：3000)"
        echo "  FRONTEND_PORT  - 前端端口 (默认：5173)"
        echo ""
        echo "示例:"
        echo "  $0 start          # 启动服务"
        echo "  $0 stop           # 停止服务"
        echo "  FRONTEND_PORT=8080 $0 start  # 自定义端口启动"
        exit 1
        ;;
esac
