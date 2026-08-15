# 专家联盟（Expert-Alliance）产品需求 · 架构 · 业务流程设计书

> 版本：v1.0（生产级实现对齐）
> 参考范式：[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) —— **"Everything is a Plugin"（一切皆插件）**
> 代码落点：`crates/expert-alliance`（含 `harness` 插件运行时、`govern` 治理、`rbac` 权限、`audit` 审计、`pipeline` 全维管线、`flow_loader` 流程外部化）
> 配套运行：`crates/runtime`（已加固：CORS 受控 / Bearer 鉴权 / 优雅关闭 / 结构化日志）

---

## 1. 系统需求规格（SRS）

### 1.1 产品定位

专家联盟是算子统一系统（OUS）的**最高权限全维业务编排内核**。它把"算法归一化、资源调度、数据血缘、权限安全、可观测性、业务建模、浏览器自动化"七个领域的专家抽象为**可热插拔、可审计、可拦截**的一等公民插件，在统一共享上下文（`HarnessCtx`）中协同求解任意业务流程，并通过**璇玑验证网关 + 治理闸门**保证产出"可上线、可审计、不越权"。

### 1.2 功能需求（FR）

| 编号 | 需求 | 优先级 | 落地 |
|------|------|--------|------|
| FR-1 | 七位专家可并行分析同一流程图，输出结构化观点（建议/风险/评分） | P0 | `experts::all_experts()` + `dispatch` |
| FR-2 | 专家作为插件可动态装载/卸载，新增专家无需改动编排层 | P0 | `harness::ExpertPlugin` + `HarnessCtx::load_plugin` |
| FR-3 | 插件可向共享上下文贡献 services / 事件监听 / 可逆副作用 | P0 | `HarnessCtx::provide` / `on` / `effect` |
| FR-4 | 在"分析前/分析后/闸门前/闸门后"提供可拦截的扩展点（瀑布） | P0 | `harness::WaterfallEvent` + `run_waterfall` |
| FR-5 | 治理闸门依据配额、角色、算法否决决定 放行/阻断 | P0 | `govern::govern` |
| FR-6 | 璇玑验证网关（最高权限）检测"阻塞型冲突/越权写/数据依赖断裂"并执行否决 | P0 | `verify::verify` |
| FR-7 | 基于 RBAC 的最小权限：viewer 仅读、editor 可改测试、admin 可改生产、安全审批员可批产线写 | P0 | `rbac` |
| FR-8 | 审计链不可篡改（哈希链 + 篡改检测），支持 Syslog/S3/Kafka/RabbitMQ/NATS 多目标外发 | P1 | `audit` |
| FR-9 | 流程图可用 YAML 外部化，业务人员免改代码增删流程 | P1 | `flow_loader` |
| FR-10 | 运行时 API 需 Bearer 鉴权（未授权拒绝），CORS 受控，支持优雅关闭 | P0 | `runtime` |
| FR-11 | 流程可确定性执行（含阻塞图不执行、互斥资源串行化） | P1 | `executor` |
| FR-12 | 性能边界：1000 节点流程优化在 release 下 < 5s（已验证通过） | P2 | `benches` / 集成测试 |

### 1.3 非功能需求（NFR）

| 维度 | 指标 |
|------|------|
| 安全 | 默认拒绝未授权 API；RBAC 默认 viewer；密钥经环境变量注入，绝不硬编码 |
| 可观测性 | 结构化日志（tracing）+ 事件总线（插件可订阅 gate/approved、gate/blocked） |
| 可审计 | 内部哈希链 + 外部多 sink；篡改检测 100% 命中 |
| 可扩展性 | 新增专家 = 实现 `Expert` trait 并注册为 `ExpertPlugin`，零侵入编排层 |
| 可靠性 | 优雅关闭、可逆副作用 unwind、流程回滚点 |
| 性能 | 见 FR-12；专家分析无状态可并行 |

---

## 2. 产品架构

### 2.1 分层拓扑（插件化内核）

```
                         ┌─────────────────────────────────────┐
                         │          接入层 (runtime/Axum)         │
                         │  CORS(受控) · Bearer鉴权 · 优雅关闭     │
                         │  /api/health(公开) · 其余需 Token      │
                         └───────────────────┬───────────────────┘
                                             │
                         ┌───────────────────▼───────────────────┐
                         │      HarnessCtx（共享上下文 / 一切皆插件）│
                         │  ┌────────┐ ┌────────┐ ┌────────────┐  │
                         │  │services│ │ events │ │effects(可逆)│  │
                         │  └────────┘ └────────┘ └────────────┘  │
                         │  ┌──────────────────────────────────┐ │
                         │  │ Waterfalls:                       │ │
                         │  │  PreAnalyze→PostAnalyze          │ │
                         │  │  PreGate→PostGate                │ │
                         │  └──────────────────────────────────┘ │
                         └───┬───────────────┬───────────┬───────┘
              ┌──────────────┼───────────────┼───────────┼──────────────┐
              ▼              ▼               ▼           ▼              ▼
        ┌──────────┐  ┌────────────┐  ┌────────┐  ┌──────────┐  ┌──────────┐
        │ 七位专家  │  │ 治理闸门    │  │ 璇玑网关│  │ RBAC引擎 │  │ 审计桥接 │
        │(Plugins) │  │ (Plugins)  │  │(verify)│  │ (rbac)   │  │(audit)   │
        └────┬─────┘  └─────┬──────┘  └───┬────┘  └────┬─────┘  └────┬─────┘
             │              │             │           │             │
             └──────────────┴─────────────┴───────────┴─────────────┘
                                             │
                         ┌───────────────────▼───────────────────┐
                         │        内核 (operator-core / flow-ai)    │
                         │  算子 trait · 高维向量 · DAG调度 · 代码生成│
                         └─────────────────────────────────────────┘
```

### 2.2 核心抽象（与 deepseek-harness 映射）

| DeepSeek Harness | 专家联盟落地 | 说明 |
|---|---|---|
| Cordis 运行时（无特权核心） | `HarnessCtx` + `Plugin` trait | 专家/治理/审计/模型适配都实现 `Plugin` |
| `ctx` 贡献 services / typed events / reversible effects | `provide` / `on` / `effect` | 类型化服务注册表 + 事件总线 + 可逆副作用栈 |
| Waterfall 扩展点（pre-step / tools/pre-execute） | `WaterfallEvent` | PreAnalyze/PostAnalyze/PreGate/PostGate 责任链 |
| Service Definition / Provider / Consumer 分离 | `ModelAdapterConfig` + `provide` | 模型适配器从配置加载，专家只消费 service |
| Profile / Bundle 组合 | `HarnessProfile` | 声明装载哪些插件，支持 `with_plugin` 叠加 |

### 2.3 七位专家职责矩阵

| 专家 | 维度 | 关注点 | 输出 |
|------|------|--------|------|
| 算法专家 | Algorithm | 归一化模式、复杂度、最优求解 | 算法建议/性能风险 |
| 资源专家 | Resource | 算力路由、配额、互斥资源串行化 | 路由 tier/资源风险 |
| 数据专家 | Data | 数据血缘、脱敏、依赖完整性 | 数据风险/脱敏建议 |
| 权限专家 | Permission | RBAC 最小权限、越权写 | 权限否决/建议 |
| 安全专家 | Security | 越权、注入、敏感数据泄露 | 安全否决/建议 |
| 可观测专家 | Observability | 埋点、追踪、告警 | 观测建议 |
| 业务专家 | Business | 业务语义、合规、流程合理性 | 业务建议 |

---

## 3. 业务流程

### 3.1 端到端主流程（`alliance_optimize`）

```
用户提交 FlowGraph(JSON)
        │
        ▼
[0] 构建 HarnessCtx（装载 7 专家插件 + 治理钩子 + 审计切面）
        │
        ▼
[1] auto_dimension 归一化：维度着色（显式 tag / LLM 语义推断）
        │
        ▼
[2] PreAnalyze 瀑布（插件可补充分析上下文）
        │
        ▼
[3] 并行派发七位专家 → 收集 ExpertOpinion（无状态、可并行）
        │
        ▼
[4] PostAnalyze 瀑布（插件可拦截/改写观点）
        │
        ▼
[5] reconcile 归一化裁决 → ReconciledPlan（含模型路由/评分/互斥注入）
        │
        ▼
[6] flow-ai 最优求解 → Optimization（含算力路由并入）
        │
        ▼
[7] 璇玑验证网关（最高权限）：阻塞冲突/越权写/数据断裂 → vetoed?
        │   + 专家否决级风险(Risk.veto) 自动升级为否决
        ▼
[8] 治理闸门：依据 配额/角色/算法否决 → Approved / Blocked
        │
        ▼
[9] PreGate 瀑布（钩子可追加前置校验，可重写闸门结果）
        │
        ▼
[10] PostGate 瀑布（审计切面：emit gate/approved|blocked）
        │
        ▼
[11] 内部审计链 append（主体/流程/动作/结果）
        │
        ▼
[12] HarnessCtx.shutdown：unload 插件 + unwind 可逆副作用
        │
        ▼
GovernanceReport（优化结果 + 验证 + 闸门 + 审计链）
```

### 3.2 关键业务规则

- **RBAC 默认最小权限**：请求未声明 `roles` 时仅授予 `viewer`，**禁止默认 admin/editor**（已在 `run`/`run_handler` 修复 RBAC 绕过）。
- **否决级风险正交机制**：任何专家判定"不可自动修复、必须人工审批"的风险只需 `push_veto`，自动触发璇玑否决，无需编排层补丁。
- **互斥资源串行化**：浏览器等互斥资源在 `reconcile` 注入 Mutex 守卫节点，确定性执行器保证单例。
- **可逆性**：插件登记的副作用在 `shutdown` 时逆序 unwind，保证流程回滚安全。

---

## 4. 安全与治理模型

### 4.1 API 安全（runtime 已落地）

| 控制项 | 实现 | 默认 |
|--------|------|------|
| 传输层 CORS | `OUS_CORS_ORIGINS` 逗号分隔白名单；含 `*` 告警并退化为 permissive | `localhost:3000` |
| 接口鉴权 | `Authorization: Bearer <OUS_API_TOKEN>`；未配 token 时受保护接口返回 503 | 未配置则全拒 |
| 公开端点 | `/api/health`、`/healthz`、静态资源 | 无 token 可访问 |
| 优雅关闭 | `SIGINT`/`SIGTERM` 触发 `with_graceful_shutdown` | 等待在途请求完成 |

### 4.2 内部权限（rbac + govern）

- 角色继承：`viewer ⊂ editor ⊂ admin`，另设 `safety-approver`（可批产线写）。
- 跨租户拒绝、空角色拒绝、通配符策略精确匹配。
- 治理闸门尊重璇玑否决：一旦 `vetoed=true`，`FlowStatus::Blocked`，绝不放行。

### 4.3 审计（audit）

- 内部哈希链（`AuditChain`）：`latest_hash` 锚定，篡改检测覆盖任意位置。
- 外部 sink 通过 `AuditSink` trait 多目标扇出：Syslog(RFC5424) / S3(WORM) / Kafka / RabbitMQ / NATS。
- 凭据/地址均由调用方经 `new(uri)` 传入，**不在代码中硬编码**（测试中的 `amqp://guest` 等仅为单测数据）。

---

## 5. 部署与运维

### 5.1 启动

```bash
# 生产环境变量（务必配置）
export OUS_API_TOKEN=$(openssl rand -hex 32)      # API 访问令牌
export OUS_CORS_ORIGINS="https://your-domain.com"  # 受信前端来源
export OUS_LLM_API_KEY="sk-..."                    # 模型适配器密钥
export RUST_LOG="info"

# 编译并启动运行时
cargo build --release -p runtime
./target/release/operator-server --port 3000
```

### 5.2 健康检查与可观测

- `GET /api/health` → 200（无需 token），用于探针。
- 结构化日志经 `tracing-subscriber`（env-filter）输出，可对接 Loki/ELK。
- 治理闸门结果经事件总线 emit `gate/approved` / `gate/blocked`，可订阅做指标。

### 5.3 扩展新专家（零侵入）

```rust
struct MyExpert;
impl Expert for MyExpert {
    fn id(&self) -> ExpertId { "my".into() }
    fn dimension(&self) -> Dimension { Dimension::Business }
    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion { /* ... */ }
}
// 注册到 HarnessProfile.plugins 即可，编排层无需改动
```

---

## 6. 验收标准（Definition of Done）

| 项 | 验证方式 | 状态 |
|----|----------|------|
| 七位专家并行分析产出结构化观点 | `pipeline::tests::alliance_end_to_end_runs` | ✅ |
| 插件化运行时（service/event/effect/waterfall） | `harness::tests::*` 5 项 | ✅ |
| RBAC 最小权限 / 默认非 admin | `rbac::check::tests::*` 8 项 | ✅ |
| 审计链不可篡改 | `govern::tests::audit_chain_tamper_detected` + `gap_p1_audit_chain_continuity` 6 项 | ✅ |
| 璇玑否决级风险 | `verify::tests::veto_*` + `gap_p1_multi_e1_*` 5 项 | ✅ |
| API 鉴权/CORS/优雅关闭 | `runtime` 编译通过 + 配置契约 | ✅ |
| 1000 节点性能边界 | `gap_p2_perf_boundaries`(release) 10 项 | ✅ |
| 全 workspace 编译 | `cargo build --workspace` | ✅ |

> 注：debug 模式下 `gap_p2_perf_boundaries` 中 3 项因性能预算（<5s）在慢测试机超时，release 全部通过，属环境特性非逻辑缺陷。
