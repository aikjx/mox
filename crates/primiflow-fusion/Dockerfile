# PrimiFlow-Fusion 企业级服务镜像
# 多阶段构建：构建阶段编译 release 二进制，运行阶段仅携带产物。
FROM rust:1.81-slim AS builder
WORKDIR /app

# 依赖缓存：先拷贝 workspace 清单，仅当依赖变化时重新编译
COPY Cargo.toml Cargo.lock ./
COPY crates/primiflow-fusion/Cargo.toml crates/primiflow-fusion/Cargo.toml
COPY crates/primiflow/Cargo.toml crates/primiflow/Cargo.toml
COPY crates/flow-ai/Cargo.toml crates/flow-ai/Cargo.toml

# 占位 src 触发依赖编译（避免拷贝全部源码导致缓存失效）
RUN mkdir -p crates/primiflow-fusion/src crates/primiflow/src crates/flow-ai/src \
    && echo "fn main() {}" > crates/primiflow-fusion/src/main.rs \
    && echo "" > crates/primiflow-fusion/src/lib.rs \
    && echo "" > crates/primiflow/src/lib.rs \
    && echo "" > crates/flow-ai/src/lib.rs \
    && cargo build --release -p primiflow-fusion || true

# 拷贝真实源码并构建
COPY . .
RUN cargo build --release -p primiflow-fusion

# ── 运行阶段 ──
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/primiflow-fusion /usr/local/bin/primiflow-fusion

# 运行时数据卷（注册表持久化 / PT-DOC 导出）
VOLUME ["/data"]
ENV OUS_FUSION_DOCS_DIR=/data/fusion_docs \
    OUS_FUSION_PERSISTENCE_PATH=/data/fusion_registry.json \
    OUS_FUSION_BIND_ADDR=0.0.0.0:8080

EXPOSE 8080
ENTRYPOINT ["primiflow-fusion"]
CMD ["serve"]
