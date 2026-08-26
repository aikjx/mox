# 专家联盟处理模式 — mox-dualrpc 集成设计

> mox-dualrpc 作为专家联盟系统的通信底座，统一 gRPC + JSON-RPC + MCP 三协议

---

## 一、定位

mox-dualrpc 是专家联盟系统的**通信基础设施层**，解决三个核心问题：

1. **内部服务间高性能通信** → gRPC (tonic) 二进制，P99 < 1ms
2. **对外/第三方/MCP 兼容通信** → JSON-RPC 2.0 文本，零依赖
3. **零配置自动转码** → 业务代码只写一套，双协议自动暴露

---

## 二、专家联盟服务协议矩阵

| 服务 | 内部 gRPC | 对外 JSON-RPC | MCP 暴露 | 说明 |
|------|-----------|--------------|----------|------|
| `gateway-http` | — | ✅ | ✅ | 接入层，协议路由 |
| `gateway-grpc` | ✅ | — | — | 内部 gRPC 网关 |
| `alliance-scheduler` | ✅ | ✅ | ✅ | 任务创建/查询/取消 |
| `alliance-executor` | ✅ | ❌ | ❌ | DAG 执行，仅内部 |
| `alliance-fusion` | ✅ | ❌ | ❌ | 结果融合，仅内部 |
| `expert-registry` | ✅ | ✅ | ✅ | 专家 CRUD/列表/健康 |
| `expert-agent` | ✅ | ❌ | ❌ | Agent 运行时，仅内部 |
| `expert-memory` | ✅ | ❌ | ❌ | 记忆/案例，仅内部 |

**原则**：
- 内部服务间 → 纯 gRPC，零转码开销
- 对外 API / MCP 工具 → JSON-RPC，网关自动转码
- 执行/融合/记忆/Agent → 不对外暴露，`expose = false`

---

## 三、MCP 工具自动生成

所有标注 `expose = true` 的方法自动出现在 MCP `tools/list` 响应中：

```json
{
  "tools": [
    {
      "name": "expert.alliance.CreateTask",
      "description": "创建专家联盟协作任务",
      "inputSchema": {
        "type": "object",
        "properties": {
          "title": { "type": "string" },
          "description": { "type": "string" },
          "preference": { "type": "object" }
        },
        "required": ["title", "description"]
      }
    },
    {
      "name": "expert.alliance.GetTask",
      "description": "查询任务状态和结果",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": { "type": "string" }
        },
        "required": ["task_id"]
      }
    },
    {
      "name": "expert.registry.ListExperts",
      "description": "列出所有可用专家",
      "inputSchema": { "type": "object", "properties": {} }
    }
  ]
}
```

**Claude Desktop / Cursor 配置**：
```json
{
  "mcpServers": {
    "mox-expert-alliance": {
      "url": "http://localhost:8080/mcp",
      "headers": { "Authorization": "Bearer <jwt>" }
    }
  }
}
```

---

## 四、注解驱动的专家服务定义

```rust
use mox_dualrpc::prelude::*;

// === alliance-scheduler 服务 ===

#[derive(Clone)]
struct AllianceScheduler;

impl AllianceScheduler {
    /// 创建任务 — 对外暴露，MCP 工具
    #[dual_rpc(
        method = "expert.alliance.CreateTask",
        cache_ttl_ms = 0,  // 写操作不缓存
        expose = true
    )]
    async fn create_task(&self, req: CreateTaskRequest) -> Result<CreateTaskResponse, DualRpcError> {
        // 1. 任务解析
        // 2. 专家匹配（图谱推理）
        // 3. 协作计划生成（DAG）
        // 4. 触发执行（NATS 事件）
        Ok(CreateTaskResponse { task_id, plan })
    }

    /// 查询任务 — 对外暴露，缓存5秒
    #[dual_rpc(
        method = "expert.alliance.GetTask",
        cache_ttl_ms = 5000,
        cache_key = "$.task_id",
        expose = true
    )]
    async fn get_task(&self, req: GetTaskRequest) -> Result<GetTaskResponse, DualRpcError> {
        // 从 PostgreSQL + Redis 查询任务状态
        Ok(GetTaskResponse { task_id, status, result, progress })
    }

    /// 内部调度 — 不对外暴露
    #[dual_rpc(expose = false)]
    async fn internal_schedule(&self, req: ScheduleRequest) -> Result<ScheduleResponse, DualRpcError> {
        // 内部调度逻辑，仅 gRPC 调用
        Ok(ScheduleResponse {})
    }
}

// === expert-registry 服务 ===

#[derive(Clone)]
struct ExpertRegistry;

impl ExpertRegistry {
    #[dual_rpc(method = "expert.registry.ListExperts", cache_ttl_ms = 10000, expose = true)]
    async fn list_experts(&self, req: ListExpertsRequest) -> Result<ListExpertsResponse, DualRpcError> {
        Ok(ListExpertsResponse { experts })
    }

    #[dual_rpc(method = "expert.registry.RegisterExpert", cache_ttl_ms = 0, expose = true)]
    async fn register_expert(&self, req: RegisterExpertRequest) -> Result<RegisterExpertResponse, DualRpcError> {
        // 注册专家 + 写入知识图谱
        Ok(RegisterExpertResponse { expert_id })
    }
}
```

---

## 五、服务启动模板

```rust
use mox_dualrpc::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 tracing
    tracing_subscriber::fmt::init();

    // 创建服务实例
    let scheduler = AllianceScheduler::new().await?;
    let registry = ExpertRegistry::new().await?;

    // 构建路由（由 #[dual_rpc] 宏自动生成，当前手动注册）
    let mut routes = Vec::new();
    routes.extend(scheduler.routes());
    routes.extend(registry.routes());

    // 启动双协议服务器
    let server = DualRpcServer::builder()
        .grpc_addr("0.0.0.0:50051")
        .jsonrpc_addr("0.0.0.0:8080")
        .register(routes)
        .build()?;

    tracing::info!("专家联盟服务启动: gRPC=:50051, JSON-RPC=:8080");
    server.serve().await?;
    Ok(())
}
```

---

## 六、调用链路

### 6.1 外部 JSON-RPC → 内部 gRPC

```
Claude Desktop (MCP)
    │ JSON-RPC 2.0 over HTTP
    ▼
gateway-http :8080/mcp
    │ 1. 解析 JSON-RPC 请求
    │ 2. L1 路由表查找 (O(1))
    │ 3. L2 JSON→Protobuf 转码 (serde, ~20μs)
    ▼
alliance-scheduler (gRPC :50051)
    │ 业务逻辑
    ▼
expert-registry (gRPC)
expert-memory (gRPC)
自研图存储 (gRPC)
    │
    ▼ Protobuf→JSON 转码
gateway-http → JSON-RPC 响应 → Claude Desktop
```

### 6.2 内部 gRPC 直连

```
alliance-executor
    │ gRPC (tonic, 二进制, ~0.5ms)
    ▼
expert-agent
    │ gRPC
    ▼
ai-inference-sidecar (UDS)
```

内部调用**零转码**，直接 Protobuf 二进制，性能最优。

---

## 七、性能预估

| 调用路径 | 协议 | 预估延迟 | 并发 |
|----------|------|---------|------|
| 内部 gRPC 直连 | gRPC | ~0.5ms | 1000+ |
| 外部 JSON-RPC → 转码 → gRPC | JSON-RPC+gRPC | ~5ms | 500 |
| MCP 工具调用 | JSON-RPC | ~8ms | 200 |
| 批量请求 (10个) | JSON-RPC batch | ~15ms | 100 |

**转码开销**：单次 JSON↔Protobuf ~20μs，占总延迟 < 1%。

---

## 八、与现有 infotopograph 集成

mox-dualrpc 作为独立 crate 放在 `projects/mox-dualrpc/`，专家联盟服务通过 workspace 依赖引用：

```toml
# platform/services/mox-expert-alliance/Cargo.toml
[dependencies]
mox-dualrpc = { path = "../../../projects/mox-dualrpc" }
```

**不修改**现有 31 个微服务的通信方式——它们继续用 axum REST。mox-dualrpc 仅用于专家联盟的 7 个新服务，作为通信底座。

---

## 九、下一步

1. **v0.2**：`#[dual_rpc]` 宏自动生成路由注册函数（消除手动 `make_route` 样板代码）
2. **v0.3**：gRPC 服务端实际运行（接入 tonic `add_service`）
3. **v0.4**：流式 RPC + WebSocket 实时进度推送
4. **v0.5**：专家联盟 7 服务全部迁移到 mox-dualrpc
5. **v1.0**：生产级稳定版，接入可观测性/限流/熔断
