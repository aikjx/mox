# OUS 治理台前端 API 实现就绪文档

> 生成时间：2026-08-16  
> 状态：**✅ 已实现**  
> 版本：v3.0-governance-console

---

## 摘要

为 OUS（算子统一系统）企业级前端治理台实现了完整的 REST API 基础设施，包含 8 个 REST 端点 + 1 个 WebSocket 实时推送通道。所有写操作均写入 AuditChain 哈希链，满足 SOC2/GDPR 合规要求。

---

## 新增文件清单

| 文件路径 | 说明 |
|---------|------|
| `crates/runtime/src/handlers/governance.rs` | 治理台核心 handlers（Dashboard / Audit / Config / Veto / WebSocket） |
| `crates/runtime/src/routes/governance.rs` | 治理台路由定义（权限映射 + 路由树） |
| `tests/governance_api.rs` | 集成测试（11 个测试用例，覆盖核心路径） |

## 修改文件清单

| 文件路径 | 修改内容 |
|---------|---------|
| `crates/runtime/src/lib.rs` | 新增 `handlers` + `routes` 模块导出 |
| `crates/runtime/src/main.rs` | 挂载治理台路由 + 初始化 `GovernanceState` |
| `crates/runtime/Cargo.toml` | 新增 `tokio-tungstenite` + `futures-util` 依赖 |

---

## API 端点清单

### 实时监控面板

```
GET /api/governance/dashboard
```

返回：
```json
{
  "timestamp": 1723785600,
  "totalFlows": 10,
  "approvedFlows": 6,
  "blockedFlows": 4,
  "vetoRate": 0.4,
  "auditEventCount": 25,
  "expertStates": { "security": { "healthScore": 0.85, ... }, ... },
  "recentVetoes": [...],
  "auditChainVerified": true,
  "businessLeagueHealth": 0.91,
  "devLeagueHealth": 0.88
}
```

### 专家状态

```
GET /api/governance/experts/status
```

返回双璇玑十四维专家状态（业务7维 + 开发7维），含各维度健康分、否决次数。

### 否决事件列表

```
GET /api/governance/veto/events?page=1&page_size=20&expert_id=security&blocked=true
```

支持分页（page / page_size）和过滤（flow_id / expert_id / dimension / from_ts / to_ts / blocked）。

### 审计日志

```
GET /api/governance/audit/logs?page=1&page_size=20&subject=alice
```

查询 AuditChain 内部哈希链，支持分页和过滤。返回哈希链完整性验证数据（prev_hash 连续性）。

### RBAC 配置

```
GET /api/governance/config/rbac    # 读取
PUT /api/governance/config/rbac    # 更新（写入 AuditChain）
```

更新时自动递增 `version`，追加审计事件到 AuditChain。

### 专家配置

```
GET /api/governance/config/experts    # 读取（权重 + 阈值）
PUT /api/governance/config/experts    # 更新（写入 AuditChain + 广播 WebSocket）
```

### WebSocket 实时推送

```
GET /api/governance/ws
```

推送消息类型：
- `connected` — 连接成功
- `veto_event` — 否决事件（实时）
- `expert_status_change` — 专家状态变化（配置更新触发）

### 治理评估触发

```
POST /api/governance/assess
Body: { "flow_id": "...", "flow_name": "...", "flow": FlowGraph }
```

触发双璇玑十四维治理评估，写入否决事件 + 审计链 + 广播 WebSocket。

---

## 核心设计

### GovernanceState — 治理台全局状态

```rust
pub struct GovernanceState {
    pub audit_chain: Arc<Mutex<AuditChain>>,         // 内存哈希链（防篡改）
    pub veto_events: Arc<Mutex<Vec<VetoEvent>>>,     // 否决事件历史
    pub expert_states: Arc<RwLock<HashMap<String, ExpertStatus>>>,  // 14维专家状态
    pub rbac_config: Arc<RwLock<RbacConfig>>,        // RBAC 配置（含版本）
    pub expert_config: Arc<RwLock<ExpertConfig>>,    // 专家权重配置
    pub veto_broadcast: broadcast::Sender<VetoEvent>,
    pub state_broadcast: broadcast::Sender<ExpertStatusChange>,
}
```

### 审计链完整性保证

- **哈希链**：每个事件记录 `prev_hash`，防篡改验证通过 `AuditChain::verify()`
- **版本化配置**：RBAC 和专家配置变更均递增 `version`，记录到 AuditChain
- **WebSocket 广播**：否决事件和专家状态变化实时推送给前端

### 双璇玑十四维

```
业务璇玑（Business League）：
  Business | Algorithm | Permission | Resource | Security | Data | Observability

开发璇玑（Dev League）：
  ApiCompat | Performance | Maintainability | Testing | Style | Cost | Sensitive
```

### 璇玑验证（Mox）最高权限

⛨ 璇玑验证网关优先级最高，算法否决不可被任何 RBAC/权限覆盖：
- `algo.vetoed = true` → 治理闸门强制 BLOCK
- 政务敏感数据越权写 → 必然被拦截

---

## 权限要求

| 端点 | 所需权限 | 角色 |
|------|---------|------|
| GET /dashboard | `governance:read` | viewer+ |
| GET /experts/status | `governance:read` | viewer+ |
| GET /veto/events | `governance:read` | viewer+ |
| GET /audit/logs | `governance:audit_read` | auditor |
| GET/PUT /config/rbac | `governance:config_write` | safety_approver / admin |
| GET/PUT /config/experts | `governance:config_write` | safety_approver / admin |
| POST /assess | `governance:assess` | editor+ |
| GET /ws | 独立 token 校验 | - |

---

## 关键约束落实

| 约束 | 实现 |
|------|------|
| 使用现有 RBAC 中间件 | 扩展 `Permission::from_route`，新增 governance 路径映射 |
| 写操作需审计 | 配置更新写入 `AuditChain.append()` |
| 配置变更需版本化 | `RbacConfig.version` / `ExpertConfig.version` 原子递增 |
| API 遵循 OpenAPI 规范 | 已在 `openapi.rs` 中定义（可扩展 governance tag） |
| WebSocket 实时推送 | `tokio::sync::broadcast` 双通道（veto + state） |

---

## 测试用例

| 测试 | 描述 |
|------|------|
| `dashboard_empty_state` | 空态仪表盘（14维专家初始化） |
| `experts_status_14_dimensions` | 验证双璇玑十四维状态 |
| `veto_events_pagination` | 分页 + 按 expert_id 过滤 |
| `audit_logs_chain_integrity` | 哈希链防篡改验证 + 过滤 |
| `rbac_config_versioning` | RBAC 版本递增 + 审计链追加 |
| `expert_config_update_and_audit` | 专家权重更新 + 审计链追加 |
| `governance_assess_full_pipeline` | 完整治理评估链路 |
| `governance_sensitive_flow_blocked` | 敏感流应被否决（璇玑验证） |
| `dashboard_veto_rate_calculation` | 否决率正确计算 |
| `audit_chain_tamper_detection` | 篡改检测（verify 失败） |
| `expert_status_broadcast` | WebSocket 广播触发验证 |

---

## 运行方式

```bash
# 编译
cargo build -p runtime

# 运行测试
cargo test -p runtime --test governance_api -- --nocapture

# 启动服务器（带 OUS_API_TOKEN）
OUS_API_TOKEN=your_token cargo run -p runtime

# 访问 API 文档
curl http://localhost:3000/api/docs
```

---

## 前端集成建议

1. **仪表盘**：每 30s 轮询 `GET /api/governance/dashboard`
2. **实时否决**：连接 `GET /api/governance/ws`，监听 `veto_event` 消息
3. **专家状态**：连接 WebSocket 监听 `expert_status_change`，或轮询 `GET /api/governance/experts/status`
4. **审计日志**：`GET /api/governance/audit/logs?page=1&page_size=50`
5. **配置变更**：PUT 请求后验证响应中 `version` 已递增

---

## 已知限制

- WebSocket 鉴权在 handler 内独立实现（未复用现有 Bearer Token 中间件）
- 外部 AuditSink（Syslog / S3 / NATS / RabbitMQ）需额外配置
- OpenAPI 规范文档中 `governance` tag 需手动补充
