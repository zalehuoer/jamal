# JamalC2 Web Server
# 多阶段构建 - 最终镜像约 30MB

# 阶段1: 构建
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /app

# 安装构建依赖 (含 MinGW-w64 用于交叉编译 C Implant)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    mingw-w64 \
    && rm -rf /var/lib/apt/lists/*

# 复制源码
COPY server-web/ ./server-web/
COPY shared/ ./shared/
COPY implant/ ./implant/
COPY implant-c/ ./implant-c/

WORKDIR /app/server-web

# 构建 Release 版本
RUN cargo build --release

# 阶段2: 运行时
FROM debian:bookworm-slim

WORKDIR /app

# 安装运行时依赖 (含 MinGW-w64 用于运行时编译 C Implant)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    mingw-w64 \
    && rm -rf /var/lib/apt/lists/*

# 从构建阶段复制二进制
COPY --from=builder /app/server-web/target/release/jamalc2-web /app/jamalc2-web

# 复制静态文件 (如果有)
COPY --from=builder /app/server-web/static /app/static

# 复制 Implant 源码用于运行时编译
COPY --from=builder /app/implant /app/implant
COPY --from=builder /app/implant-c /app/implant-c
COPY --from=builder /app/shared /app/shared

# 创建数据目录
RUN mkdir -p /app/data

# 环境变量
ENV JAMAL_WEB_PORT=443
ENV JAMAL_DATA_DIR=/app/data

# 暴露端口 (Web UI + 默认 C2 Listener)
EXPOSE 443 80

# 启动
CMD ["./jamalc2-web"]
