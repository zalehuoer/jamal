#!/bin/bash
# JamalC2 Linux 快速部署脚本

set -e

echo "========================================="
echo "  JamalC2 Linux 部署脚本"
echo "========================================="
echo ""

# 检查 Docker
if ! command -v docker &> /dev/null; then
    echo "[*] 安装 Docker..."
    curl -fsSL https://get.docker.com | sh
    sudo usermod -aG docker $USER
    echo "[+] Docker 安装完成，请重新登录后再运行此脚本"
    exit 0
fi

# 检查 Docker Compose
if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "[*] 安装 Docker Compose..."
    sudo apt-get update && sudo apt-get install -y docker-compose-plugin
fi

echo "[*] 构建镜像..."
docker compose build

echo "[*] 启动服务..."
docker compose up -d

echo ""
echo "========================================="
echo "[+] JamalC2 已启动!"
echo "    监听端口: 4444"
echo ""
echo "    查看日志: docker compose logs -f"
echo "    停止服务: docker compose down"
echo "========================================="
