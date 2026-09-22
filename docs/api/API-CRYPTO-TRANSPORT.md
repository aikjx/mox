# 接口 data 压缩加密传输 — 一键开关设计与全链路证明

> **层定位**：L4 接口契约层。本文是「传输加密（压缩+加密）」这一事实的唯一权威源。
> 相关：[REST 契约规范](./API-SPECIFICATION.md) · [端口注册表](./PORT-REGISTRY.md) · [联盟现状架构](../expert-alliance/CURRENT-ARCHITECTURE.md)

---

## 一、现状分析：所有模块是否支持 data 压缩加密传输？

**改造前**：不支持。全部接口（网关 :3080、联盟调度 :3100、执行 :3200、注册 :3400）均为明文 JSON；仓库仅有 SM4 算法单元（原 `mox-data-standards-core::gm-sm`）与静态字段加密，无任何**传输层**统一方案，各模块若要支持需逐一改造。

**改造后**：支持，且归一化为**单一开关 + 每服务一行挂载**。任何挂载 `crypto_middleware` 的 axum 服务，其响应信封 `{code, msg, data}` 中的 `data` 自动 gzip 压缩 + SM4-GCM 加密；服务间内部调用（网关↔调度器↔执行器、SDK）同样自动加解密。

| 模块 | 挂载点 | 状态 |
|------|--------|:----:|
| 平台网关 :3080 | `src/lib.rs` 最内层（鉴权/限流之后、业务 handler 之前） | ✅ |
| 联盟调度器 :3100 | `routes.rs` `crypto_middleware` | ✅ |
| 联盟执行器 :3200 | `routes.rs` `crypto_middleware` | ✅ |
| 联盟注册中心 :3400 | `routes.rs` `crypto_middleware` | ✅ |
| 调度器→执行器 HTTP 桥 | `executor_bridge.rs` `crypto_post`/`parse_response` | ✅ |
| 联盟 HTTP SDK | `alliance_remote.rs` `call()`（协商头+seal+双模式解密） | ✅ |

## 二、一键加密设计（`platform/foundation/mox-api-crypto`）

### 2.1 开关（运维只有一个变量）

| 环境变量 | 取值 | 语义 |
|----------|------|------|
| `MOX_API_CRYPTO` | 未设置 / 空 / `off` | **关闭**：中间件透传，零开销（默认，存量行为不变） |
| | `sm4` | **开启**：协商通过的请求/响应全部 gzip+SM4-GCM |
| `MOX_API_CRYPTO_KEY` | 32 位 hex（128-bit SM4 密钥） | 生产必须显式注入；未配置回退内置开发密钥并打 WARN（仅限本地/演示） |

开关经 `CryptoConfig::from_env()` 进程级 OnceLock 缓存，一次读取、全链路一致。

### 2.2 按请求协商（存量明文客户端无感）

- 客户端（浏览器/SDK/服务桥）在请求头携带 `x-mox-crypto: sm4-gcm+gzip` 表示"我支持解密"，服务端**仅对协商请求**加密响应，并在响应头回显同名头。
- 未协商的客户端收到明文，**前端零改造、灰度可回退**——这是"一键添加"不破坏现有 223 条路由调用方的关键。

### 2.3 线上格式（wire format）

```json
{"crypto":{"alg":"SM4-GCM","zip":"gzip","nonce":"<b64 12B>","ct":"<b64>","tag":"<b64 16B>"}}
```

- 明文 = gzip(规范化 JSON)；AAD = `mox-api-crypto/v1`；nonce 每次 `OsRng` 随机；tag 常数时间比较。
- SM4-GCM 为国密分组模式（GM/T 0002-2012 + NIST SP 800-38D），纯 Rust 实现于 `mox-api-crypto::sm4_gcm`（自 `mox-data-standards-core` 归一化迁入，后者经 `gm-sm` feature re-export 保持兼容）。

### 2.4 两种响应形态，统一归一化

1. **标准信封** `{code, msg, data}`：只密封 `data`，`code/msg` 保持明文可路由（网关监控、错误码分发不失明）。
2. **裸 DTO**（对象且无 `data` 无 `code`，如调度器直连接口）：**整体密封**为 `{"crypto":…}`，客户端 `open_response` 双模式自动还原。

客户端出向同理：`seal_request` 密封请求体，服务端在中间件内解密后再进鉴权/业务。伪造密文（tag 校验失败）→ 400 拒绝。

### 2.5 归一化程度

- 一个 foundation crate、一个中间件函数、每服务**一行** `.layer(from_fn(crypto_middleware))`；
- 网关内部转发（→调度器→执行器）经由同一 `client` 模块（`outbound_headers`/`seal_request`/`open_response`），算法/格式/开关三处**同源单实现**，不存在逐接口改造的分叉风险。

## 三、全链路全维度归一化证明

### 3.1 自动化证据（活体全栈，`MOX_API_CRYPTO=sm4`）

```bash
python tools/alliance-demo/crypto_proof.py    # 6 项全 PASS
```

| # | 命题 | 结果 |
|---|------|------|
| P1 | 无协商头 → 明文信封逐字节不变（向后兼容） | ✅ 200 |
| P2 | 带协商头 → `data` 变为 SM4-GCM+gzip 密文信封 | ✅ 200 |
| P3 | 压缩有效：密文信封 476B < 明文 2804B（17.0%） | ✅ |
| P4 | 调度器 :3100 裸 DTO 整体加密（同一开关生效） | ✅ 200 |
| P5 | 网关↔调度器内部加解密互操作（密封请求建任务成功） | ✅ 200 |
| P6 | 伪造密文上送被拒（认证失败 400） | ✅ |

### 3.2 单元/集成测试

`mox-api-crypto` 21 测试（codec/middleware/config/SM4-GCM 附录 D 已知向量+压缩有效性+裸 DTO 还原）；联盟三 svc、网关（111）、scheduler-core（95，含 http-bridge）、http-sdk（15）全绿；`alliance_demo.py --modes parallel,voting` 端到端生命周期回归通过（18/22 + 4 降级为无外部 API key 的预期降级，治理体检通过）。

## 四、优化建议（专家联盟演进向）

1. **前端 JS 侧 SM4 引入路径**：当前前端零改造走明文；若需全链路强制加密，建议引入 `sm-crypto`（或 WASM 封装本 crate）在浏览器完成协商，避免网关做 TLS 终止后再解密的中转成本。优先级 P2。
2. **裸 DTO → 信封归一化**：调度器直连接口返回裸 DTO 是历史遗留，中间件为此加了"整体密封"分支。将联盟各 svc 响应统一为 `{code,msg,data}` 信封后可删除该分支，协议面收得更窄。优先级 P2。
3. **生产密钥治理**：DEV_KEY 回退仅限本地；生产接入 KMS/Nacos 加密配置注入 `MOX_API_CRYPTO_KEY`，并建立密钥轮换（信封已含 nonce，双密钥过渡期可按 `kid` 字段扩展）。优先级 P1（上生产前）。
4. **性能**：gzip 级别可调（当前默认）；大 payload（图谱导出类）可评估 zstd；SM4-GCM 纯 Rust 在 <64KB 报文上开销亚毫秒级，暂不需硬件加速。优先级 P3。
5. **协议演进**：gRPC 内网通信（v3 目标态）应复用同一 `mox-api-crypto` codec 封装为 interceptor，保持"一键"语义跨协议一致；与 mTLS 互补（mTLS 管通道，本方案管字段级 data 密级）。优先级 P3。

---

*实现真源：`platform/foundation/mox-api-crypto/`（codec/middleware/client/config/sm4_gcm）；证明脚本：`tools/alliance-demo/crypto_proof.py`。*
