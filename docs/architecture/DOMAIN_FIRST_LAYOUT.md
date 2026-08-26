# infotopograph 最优目录结构规范 v2.0（按域组织，域内分层）

> 核心原则：**一个业务域的所有代码在一个目录下**，域内再按层(api/svcapi/core/svc/sdk)组织。
> 这是企业级微服务架构的最优解——模块独立部署、独立升级、团队分工清晰、新增域零摩擦。

---

## 一、为什么按域组织优于按层组织

### 1.1 对比

| 维度 | 按层组织（旧） | 按域组织（新，最优） |
|------|----------------|----------------------|
| **找代码** | 图谱代码散落在 core/kg/ + services/kg/ + sdk/ 3处 | 全部在 domains/kg/ 一个目录 |
| **新增业务域** | 需在 foundation/core/api/svcapi/services/sdk 6个层目录各加子目录 | 只需新增 domains/xxx/ 一个目录 |
| **独立部署** | 一个域的代码跨多层，构建需跨目录引用 | 一个域一个目录，直接 docker build |
| **团队分工** | 代码散落，CODEOWNERS 需配多条规则 | 一个团队一个域目录，一条规则 |
| **独立升级** | 域内版本散落在多层 Cargo.toml | 域内可统一版本，独立发布 |
| **微服务边界** | 层是技术边界，不是业务边界 | 域天然就是微服务边界 |
| **对接零修改** | 跨层引用路径复杂 | 域内引用路径短，域间只依赖契约 |

### 1.2 结论

**按域组织（Domain-first）是企业级微服务的最优解。** 按层组织只适合小型单体项目，当模块数超过20个、需要独立部署时，按域组织的优势呈指数级放大。

---

## 二、最终目录结构

```
infotopograph/
├── platform/
│   ├── foundation/              # 跨域共享基础层（所有域共享，零业务逻辑）
│   │   ├── mox-platform-foundation/   # 公共类型/元数据/错误码/ID生成
│   │   └── mox-cloud-foundation/      # 云存储域抽象接口
│   │
│   ├── framework/               # 企业级横切框架（所有服务共享的基础设施）
│   │   └── mox-framework/            # config/logging/error/health/metrics/tracing/auth/tenant/resilience/server
│   │
│   ├── gateway/                 # 统一接入网关（协议分流/路由/鉴权/限流）
│   │   └── mox-platform-gateway-svc/
│   │
│   └── domains/                 # ★ 业务域（模块优先，一个域=一个微服务边界）
│       ├── kg/                   # 知识图谱域（9 crates）
│       │   ├── core/                  # 纯计算核心（零IO）
│       │   │   ├── mox-kg-algo-core/       # 图算法 PageRank/最短路径/社区
│       │   │   └── mox-kg-meta-core/       # 图元数据/类型系统
│       │   ├── svc/                   # 业务服务实现
│       │   │   ├── mox-kg-storage-svc/     # 自研分布式图存储(RocksDB+Raft)
│       │   │   ├── mox-kg-service-svc/     # 图查询/遍历/CRUD
│       │   │   ├── mox-kg-streams-svc/     # 图变更流/CDC
│       │   │   ├── mox-kg-spark-svc/       # 图Spark计算
│       │   │   ├── mox-kg-hub-svc/         # 图谱Hub(本体/推理/摄入/索引/治理)
│       │   │   └── mox-kg-fusion-svc/      # 知识融合/实体对齐
│       │   ├── sdk/                   # 客户端SDK
│       │   │   └── mox-kg-sdk/              # 图谱客户端SDK
│       │   ├── api/                   # 对外DTO+REST/JSON-RPC契约（待建）
│       │   └── svcapi/                # 服务间gRPC契约（待建）
│       │
│       ├── ai/                   # AI智能域（5 crates）
│       │   ├── core/
│       │   │   ├── mox-ai-core/             # AI核心类型/接口
│       │   │   └── mox-ai-intent-core/      # 意图识别核心
│       │   ├── svc/
│       │   │   ├── mox-ai-flow-svc/         # AI流程编排
│       │   │   ├── mox-ai-expert-svc/       # 专家服务/注册
│       │   │   └── mox-ai-agent-svc/        # AI Agent/ReAct循环
│       │   ├── api/  svcapi/  sdk/
│       │
│       ├── flow/                 # 流程自动化域（6 crates）
│       │   ├── core/
│       │   │   ├── mox-flow-operator-core/   # 算子核心/接口
│       │   │   └── mox-flow-optimizer-core/  # DAG优化器
│       │   ├── svc/
│       │   │   ├── mox-flow-operator-wasm-svc/# WASM算子运行时
│       │   │   ├── mox-flow-primiflow-svc/   # PrimiFlow核心引擎
│       │   │   ├── mox-flow-fusion-svc/      # 流程融合
│       │   │   └── mox-flow-bridge-svc/      # 外部系统桥接
│       │   ├── api/  svcapi/  sdk/
│       │
│       ├── data/                 # 数据治理域（9 crates）
│       │   ├── core/
│       │   │   ├── mox-data-formula-core/    # 公式引擎
│       │   │   ├── mox-data-norm-core/       # 数据归一化
│       │   │   └── mox-data-standards-core/  # 数据标准
│       │   ├── svc/
│       │   │   ├── mox-data-plane-svc/       # 数据平面/接入
│       │   │   ├── mox-data-etl-svc/         # ETL WASM运行时
│       │   │   ├── mox-data-compliance-svc/  # 合规/审计/治理
│       │   │   └── mox-data-catalog-svc/     # 业务/数据目录
│       │   └── sdk/
│       │       ├── mox-data-formula-native/   # 公式引擎原生绑定
│       │       └── mox-data-norm-intent-native/# 归一化+意图原生绑定
│       │
│       ├── cloud/                # 云存储域（5 crates）
│       │   ├── svc/
│       │   │   ├── mox-cloud-master-svc/     # 云盘主控/元数据
│       │   │   ├── mox-cloud-volume-svc/     # 云盘卷/块存储
│       │   │   ├── mox-cloud-s3-svc/         # S3兼容对象存储
│       │   │   └── mox-cloud-filer-svc/      # 文件器
│       │   └── sdk/
│       │       └── mox-cloud-sdk/             # 云存储客户端SDK
│       │
│       ├── voice/                # 语音域（7 crates）
│       │   ├── core/
│       │   │   └── mox-voice-dsp-core/       # 数字信号处理
│       │   ├── svc/
│       │   │   ├── mox-voice-core-svc/       # 语音核心/会话
│       │   │   ├── mox-voice-asr-svc/        # 语音识别
│       │   │   ├── mox-voice-intent-svc/     # 语音意图
│       │   │   ├── mox-voice-operator-svc/    # 语音算子/桌面操作
│       │   │   └── mox-voice-desktop-app/     # 桌面客户端
│       │   └── sdk/
│       │       └── mox-voice-dsp-py/          # Python DSP绑定
│       │
│       ├── market/               # 市场域（1 crate）
│       │   └── svc/
│       │       └── mox-market-template-svc/   # 模板市场
│       │
│       └── platform/             # 平台基础域（3 crates）
│           ├── core/
│           │   └── mox-platform-system-core/   # 用户/角色/权限核心
│           ├── svc/
│           │   └── mox-platform-orchestrator-svc/# 编排器(原runtime拆分)
│           └── sdk/
│               └── mox-platform-test-harness/   # 测试框架
│
├── projects/
│   └── mox-dualrpc/             # 多协议通信底座（gRPC+JSON-RPC+Dubbo零配置）
│
├── tools/                        # 架构工具链
│   ├── architecture_audit.py          # 算法级架构审计
│   ├── architecture_constraint_test.py# CI架构约束测试
│   ├── migrate_architecture.py        # 归一化迁移脚本
│   ├── migrate_to_domain_first.py     # 按域组织迁移脚本
│   └── update_cargo_paths.py          # Cargo路径更新脚本
│
├── docs/architecture/
│   ├── NORMALIZED_ARCHITECTURE.md     # 归一化架构规范
│   ├── OPTIMAL_ARCHITECTURE.md        # 最优架构总纲
│   └── DOMAIN_FIRST_LAYOUT.md         # 本文档
│
└── Cargo.toml                    # workspace根（48内部crate统一workspace.dependencies）
```

---

## 三、域内分层规范

每个业务域目录下固定5层：

| 层 | 目录 | 职责 | 依赖方向 | 零依赖？ |
|----|------|------|----------|----------|
| **api** | `api/` | 对外DTO + REST/JSON-RPC接口契约 | 零内部依赖 | ✅ |
| **svcapi** | `svcapi/` | 服务间gRPC契约(.proto+tonic stub) | 仅依赖api | ✅ |
| **core** | `core/` | 纯计算核心(零IO，可独立测试) | 仅依赖foundation | ✅ |
| **svc** | `svc/` | 业务服务实现 | 依赖api+svcapi+core+framework | ❌ |
| **sdk** | `sdk/` | 客户端SDK/FFI绑定 | 依赖api+svcapi | ❌ |

**关键规则**：
1. 域内调用：svc → core（同域内直接依赖）
2. 跨域调用：svcA → svcapiB（只依赖对方契约，不依赖实现）
3. 所有svc都依赖 mox-framework（横切基础设施）
4. core层零IO，可独立单元测试，覆盖率>90%

---

## 四、跨域调用规范（对接零修改的基础）

```
❌ 错误: domains/ai/svc/mox-ai-agent-svc 直接依赖 domains/kg/svc/mox-kg-storage-svc
   (实现依赖实现，升级/替换/协议切换必须改代码)

✅ 正确: domains/ai/svc/mox-ai-agent-svc 依赖 domains/kg/svcapi/mox-kg-svcapi
   (只依赖契约，实现可自由替换，协议可零修改切换)
```

**契约层(svcapi)是对接零修改的核心**：
- 调用方只依赖 svcapi 中的 trait/stub
- 实现方在 svc 中 impl trait
- mox-dualrpc 在运行时根据配置选择 gRPC/JSON-RPC/Dubbo 传输
- 协议切换 = 改配置文件，零代码修改

---

## 五、独立部署与升级

### 5.1 每个域 = 一个独立部署单元

```bash
# 构建知识图谱域镜像
docker build -t mox-kg:v1.2.0 platform/domains/kg/

# 构建AI域镜像
docker build -t mox-ai:v2.0.0 platform/domains/ai/

# K8s独立部署，互不影响
kubectl apply -f deploy/kg-deployment.yaml
kubectl apply -f deploy/ai-deployment.yaml
```

### 5.2 独立升级路径

| 升级类型 | 操作 | 影响范围 |
|----------|------|----------|
| Patch(bug修复) | 域内Cargo.toml版本 1.2.0→1.2.1，重新构建镜像 | 仅该域 |
| Minor(新增接口) | svcapi新增方法，向后兼容 | 旧调用方不受影响 |
| Major(接口变更) | svcapi版本化，可并行运行v1/v2 | 需调用方适配 |
| 协议切换 | 配置文件修改transport字段 | 零代码修改 |
| 新增业务域 | 新增 domains/xxx/ 目录，注册到workspace | 不影响其他域 |

---

## 六、新增业务域的标准流程（零摩擦）

```bash
# 1. 创建域目录结构
mkdir -p platform/domains/newdomain/{api,svcapi,core,svc,sdk}

# 2. 创建核心计算crate
cargo new --lib platform/domains/newdomain/core/mox-newdomain-algo-core

# 3. 创建服务crate
cargo new --lib platform/domains/newdomain/svc/mox-newdomain-service-svc

# 4. 注册到workspace
# 在Cargo.toml的members和workspace.dependencies中添加路径

# 5. 定义契约
# 在svcapi中定义.proto和Rust trait

# 6. 实现服务
# 在svc中impl trait，使用#[dual_rpc_service]宏自动注册

# 7. 独立部署
# docker build + kubectl apply
```

**对比按层组织**：新增一个域需要在6个层目录各创建子目录，修改6处workspace配置。按域组织只需1个目录+2处配置。

---

*本文档为 infotopograph 最优目录结构规范 v2.0。核心创新：按域组织（Domain-first），一个业务域的所有代码在一个目录下，域内再分层。这是企业级微服务架构的最优解。*
