# =============================================================================
# MOX API 网关 - Dockerfile
# 多阶段构建：Rust 编译 → 轻量运行
# =============================================================================

# ── 阶段 1：Rust 编译环境 ──────────────────────────────────────────────────
FROM rust:1.75-bookworm AS builder

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 先复制 Cargo.toml 和 Cargo.lock，利用 Docker 缓存依赖
COPY Cargo.toml Cargo.lock ./

# 创建虚拟项目，先编译依赖
RUN mkdir -p platform/gateway/mox-api-gateway/src && \
    echo 'fn main() {}' > platform/gateway/mox-api-gateway/src/main.rs && \
    mkdir -p platform/shared/mox-unified-contract/src && \
    echo '' > platform/shared/mox-unified-contract/src/lib.rs && \
    mkdir -p platform/shared/mox-config-core/src && \
    echo '' > platform/shared/mox-config-core/src/lib.rs && \
    mkdir -p platform/shared/mox-auth-core/src && \
    echo '' > platform/shared/mox-auth-core/src/lib.rs && \
    mkdir -p platform/shared/mox-observability-core/src && \
    echo '' > platform/shared/mox-observability-core/src/lib.rs && \
    mkdir -p platform/shared/mox-unified-algo-core/src && \
    echo '' > platform/shared/mox-unified-algo-core/src/lib.rs

# 编译依赖（这一层会被缓存）
RUN cargo build --release -p mox-api-gateway 2>&1 || true

# 复制完整源代码
COPY . .

# 编译 release 版本
RUN cargo build --release -p mox-api-gateway

# ── 阶段 2：轻量运行环境 ────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN groupadd -r mox && useradd -r -g mox -d /app -s /sbin/nologin mox

# 设置工作目录
WORKDIR /app

# 从编译阶段复制二进制文件
COPY --from=builder /app/target/release/mox-api-gateway /app/mox-api-gateway

# 创建数据目录
RUN mkdir -p /app/data /app/logs && \
    chown -R mox:mox /app

# 切换到非 root 用户
USER mox

# 环境变量
ENV RUST_LOG=info
ENV MOX_GATEWAY_HOST=0.0.0.0
ENV MOX_GATEWAY_PORT=8080
ENV MOX_ENV=production

# 健康检查
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health/live || exit 1

# 暴露端口
EXPOSE 8080

# 启动命令
CMD ["./mox-api-gateway"]
