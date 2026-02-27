#!/bin/bash
# JamalC2 Linux 快速部署脚本 (国内外自适应)

set -e

echo "========================================="
echo "  JamalC2 Linux 部署脚本"
echo "========================================="
echo ""

# 探测网络环境
USE_CN_MIRROR=false
echo "[*] 探测网络环境..."
if ! curl -sS --connect-timeout 3 https://www.google.com &> /dev/null; then
    USE_CN_MIRROR=true
    echo "[*] 检测到国内网络，使用镜像源"
else
    echo "[*] 检测到国际网络，使用官方源"
fi

# 安装 Docker
if ! command -v docker &> /dev/null; then
    if [ "$USE_CN_MIRROR" = true ]; then
        echo "[*] 安装 Docker (阿里云镜像)..."
        apt-get update && apt-get install -y ca-certificates curl gnupg
        install -m 0755 -d /etc/apt/keyrings
        curl -fsSL https://mirrors.aliyun.com/docker-ce/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
        chmod a+r /etc/apt/keyrings/docker.gpg
        echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://mirrors.aliyun.com/docker-ce/linux/ubuntu $(. /etc/os-release && echo "$VERSION_CODENAME") stable" > /etc/apt/sources.list.d/docker.list
        apt-get update && apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
    else
        echo "[*] 安装 Docker (官方源)..."
        curl -fsSL https://get.docker.com | sh
    fi
    systemctl enable --now docker
    echo "[+] Docker 安装完成"
fi

# 检查 Docker Compose
if ! docker compose version &> /dev/null; then
    echo "[*] 安装 Docker Compose 插件..."
    apt-get update && apt-get install -y docker-compose-plugin
fi

# 国内配置 Docker 镜像加速器
if [ "$USE_CN_MIRROR" = true ] && [ ! -f /etc/docker/daemon.json ]; then
    echo "[*] 配置 Docker 镜像加速器..."
    mkdir -p /etc/docker
    cat > /etc/docker/daemon.json <<EOF
{
  "registry-mirrors": ["https://mirror.ccs.tencentyun.com"]
}
EOF
    systemctl restart docker
fi

echo "[*] 构建镜像..."
docker compose build

echo "[*] 启动服务..."
docker compose up -d

echo ""
echo "========================================="
echo "[+] JamalC2 已启动!"
echo "    Web 面板: https://<IP>:443"
echo "    C2 监听:  默认端口 80 (可在面板中修改)"
echo ""
echo "    查看日志: docker compose logs -f"
echo "    停止服务: docker compose down"
echo "========================================="
