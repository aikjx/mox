# 接口层入口 — API Layer

> **层定位**：L4 接口与数据层（接口半边）。本目录是**接口契约类事实的唯一存放位置**。
> 上层入口：[文档中心](../README.md) · 结构规范：[ARCHITECTURE-OF-DOCS.md](../ARCHITECTURE-OF-DOCS.md) · 数据半边：[`../database/`](../database/README.md)

---

## 一、本层文档

| 文档 | 权威等级 | 说明 | 校验 |
|------|:--------:|------|------|
| [`API-SPECIFICATION.md`](./API-SPECIFICATION.md) | 🟢 | REST 接口契约规范（路径/方法/入参/出参/错误码约定） | 与 `../API-REGISTRY.md` 对照 |
| [`API-CRYPTO-TRANSPORT.md`](./API-CRYPTO-TRANSPORT.md) | 🟢 | 接口 data 压缩+SM4-GCM 加密传输一键开关（协商头/线上格式/全链路证明） | `python tools/alliance-demo/crypto_proof.py` |
| [`PORT-REGISTRY.md`](./PORT-REGISTRY.md) | 🟢 | **端口分配唯一权威**（服务 → 端口 → 用途） | `python scripts/gate/verify-ports.py` |
| [`TCP-SPECIFICATION.md`](./TCP-SPECIFICATION.md) | 🟢 | TCP/长连接协议规范（帧格式、心跳、鉴权） | 人工评审 |
| [`mox-module-manifest.schema.json`](./mox-module-manifest.schema.json) | 🟢 | 模块清单（manifest）JSON Schema | `python tools/module_catalog.py --check` |

## 二、与接口总账的关系

| 文档 | 位置 | 职责分工 |
|------|------|----------|
| 接口**契约规范**（怎么写） | 本目录 `API-SPECIFICATION.md` | 命名、版本、错误码、鉴权、分页等**约定** |
| 接口**注册总账**（有哪些） | [`../API-REGISTRY.md`](../API-REGISTRY.md) | 223 条路由 ↔ 实现源码一一对应、46 域、独立服务矩阵 |
| 接口**实现真源** | `platform/gateway/**/actuator.rs` 的 `ROUTES` | 代码即真源，注册表由脚本生成 |

```bash
python scripts/doc/gen-api-registry.py     # 重新生成 ../API-REGISTRY.md（禁止手改注册表正文）
python scripts/gate/verify-ports.py         # 端口漂移校验（CI 门禁）
```

## 三、变更流程

1. **改路由** → 先改代码 `ROUTES` → 重新生成 `../API-REGISTRY.md` → 在 `../enterprise/00-INDEX.md` 变更记录留痕。
2. **改端口** → 同步 `PORT-REGISTRY.md` → `verify-ports.py` 必须通过 → 若涉及部署，同步 `deploy/` 配置。
3. **改协议** → 提 ADR（放 `../enterprise/`，命名 `*-ADR-<编号>.md`）→ 更新本目录文档与相关模块文档。

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 配置模型、调用方场景与既有注册路由 |
| 处理与边界 | 规范编辑/校验/发布/选择/执行操作、版本兼容、错误和并发；拒绝任意函数或路径绑定 |
| 输出 | 可验证契约与消费者适配要求，不捏造端点 |
| 维护角色 | API契约owner（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/api/LOWCODE-CONFIGURATION-CONTRACT.md#contract) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。

资源知识目标契约：[操作与提供方能力](docs/api/RESOURCE-KNOWLEDGE-CONTRACT.md#operations)，未登记为已存在的新端点。
