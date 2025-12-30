# JamalC2 Web Server
# 多阶段构建 - 最终镜像约 30MB

# 阶段1: 构建
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制源码
COPY server-web/ ./server-web/
COPY shared/ ./shared/

WORKDIR /app/server-web

# 构建 Release 版本
RUN cargo build --release

# 阶段2: 运行时
FROM debian:bookworm-slim

WORKDIR /app

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 从构建阶段复制二进制
COPY --from=builder /app/server-web/target/release/jamalc2-web /app/jamalc2-web

# 复制静态文件 (如果有)
COPY --from=builder /app/server-web/static /app/static

# 创建数据目录
RUN mkdir -p /app/data

# 环境变量
ENV JAMAL_PORT=4444
ENV JAMAL_DATA_DIR=/app/data

# 暴露端口
EXPOSE 4444

# 启动
CMD ["./jamalc2-web"]
