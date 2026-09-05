# =============================================================================
# MOX 微服务 Dockerfile（统一模板）
# 构建：docker build -t mox-mox-cloud-server-server --build-arg SERVICE=mox-mox-cloud-server-server .
# 运行：docker run -p 8102:8102 -v ./config:/etc/mox mox-mox-cloud-server-server
# =============================================================================

FROM rust:1.98-slim AS builder
ARG SERVICE=mox-kb-server
WORKDIR /workspace

# 安装系统依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 先复制 Cargo.toml 以利用 Docker 层缓存
COPY Cargo.toml Cargo.lock ./
COPY platform ./platform

# 构建指定服务
RUN cargo build --release -p ${SERVICE}

# ── 运行时镜像 ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim
ARG SERVICE=mox-kb-server
ENV SERVICE_NAME=${SERVICE}
ENV RUST_LOG=info
ENV MOX_SERVER_HOST=0.0.0.0

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r mox && useradd -r -g mox mox

WORKDIR /app
COPY --from=builder /workspace/target/release/${SERVICE} /app/server
COPY --from=builder /workspace/platform/domains/*/svc/${SERVICE}/config/*.toml /etc/mox/ 2>/dev/null || true

RUN chown -R mox:mox /app /etc/mox
USER mox

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${MOX_SERVER_PORT:-8080}/health/live || exit 1

ENTRYPOINT ["/app/server", "--config", "/etc/mox/server.toml"]