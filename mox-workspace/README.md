# 璇玑 RelGraph · Mox Platform

> 以 Rust 自研高性能知识图谱为唯一中枢的七维归一化关联与自动化治理系统。

## 架构

```
6层8域 DDD 矩阵：

L6  Gateway       mox-platform-gateway-runtime
                    │
L5  Services      mox-*-svc (kg/ai/flow/data/cloud/platform)
                    │
L2  SvcAPI        mox-*-svcapi (gRPC 契约)
                    │
L1  API           mox-*-api (REST DTO)
                    │
L3  Core          mox-*-core (纯计算 · 零IO)
                    │
L0  Foundation    mox-platform-foundation
```

## 快速开始

### 前置条件

- Rust 1.80+ (`rustup default stable`)
- protobuf 编译器 3.20+
- PostgreSQL 14+ / Redis 7+（生产环境）

### 构建

```bash
cargo build
```

### 测试

```bash
cargo test
```

### 运行

```bash
cp .env.example .env
cargo run -p mox-platform-gateway-runtime
```

## 开发规范

- 命名公式：`mox-<domain>-<layer>-<role>`
- 8 个业务域：kg / ai / flow / data / cloud / voice / platform / market
- 6 个架构层：foundation / core / api / svcapi / svc / gateway
- 依赖方向：上层依赖下层，禁止反向依赖
- Core 层零 IO，纯计算，可独立测试

## 文档

- 开发文档：`../developer-docs.html`
- 架构图谱：`../docs-optimal-architecture-map.html`
- 专家联盟白皮书：`expert-alliance-tech-whitepaper/index.html`

## 项目结构

```
mox-workspace/
├── platform/
│   ├── foundation/          # L0 基础层
│   ├── core/                # L3 核心计算层
│   ├── api/                 # L1 对外契约层
│   ├── svcapi/              # L2 服务间契约层
│   ├── services/            # L5 服务实现层
│   └── gateway/             # L6 网关运行时
├── Cargo.toml               # workspace 配置
├── rust-toolchain.toml
└── .env.example
```

## License

MIT OR Apache-2.0
