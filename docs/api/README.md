# 接口层入口 — API Layer

> **层定位**：L4 接口与数据层（接口半边）。本目录是**接口契约类事实的唯一存放位置**。
> 上层入口：[文档中心](../README.md) · 结构规范：[ARCHITECTURE-OF-DOCS.md](../ARCHITECTURE-OF-DOCS.md) · 数据半边：[`../database/`](../database/README.md)

---

## 一、本层文档

| 文档 | 权威等级 | 说明 | 校验 |
|------|:--------:|------|------|
| [`API-SPECIFICATION.md`](./API-SPECIFICATION.md) | 🟢 | REST 接口契约规范（路径/方法/入参/出参/错误码约定） | 与 `../API-REGISTRY.md` 对照 |
| [`PORT-REGISTRY.md`](./PORT-REGISTRY.md) | 🟢 | **端口分配唯一权威**（服务 → 端口 → 用途） | `python scripts/verify-ports.py` |
| [`TCP-SPECIFICATION.md`](./TCP-SPECIFICATION.md) | 🟢 | TCP/长连接协议规范（帧格式、心跳、鉴权） | 人工评审 |
| [`mox-module-manifest.schema.json`](./mox-module-manifest.schema.json) | 🟢 | 模块清单（manifest）JSON Schema | `python tools/module_catalog.py --check` |

## 二、与接口总账的关系

| 文档 | 位置 | 职责分工 |
|------|------|----------|
| 接口**契约规范**（怎么写） | 本目录 `API-SPECIFICATION.md` | 命名、版本、错误码、鉴权、分页等**约定** |
| 接口**注册总账**（有哪些） | [`../API-REGISTRY.md`](../API-REGISTRY.md) | 223 条路由 ↔ 实现源码一一对应、46 域、独立服务矩阵 |
| 接口**实现真源** | `platform/gateway/**/actuator.rs` 的 `ROUTES` | 代码即真源，注册表由脚本生成 |

```bash
python scripts/gen-api-registry.py     # 重新生成 ../API-REGISTRY.md（禁止手改注册表正文）
python scripts/verify-ports.py         # 端口漂移校验（CI 门禁）
```

## 三、变更流程

1. **改路由** → 先改代码 `ROUTES` → 重新生成 `../API-REGISTRY.md` → 在 `../enterprise/00-INDEX.md` 变更记录留痕。
2. **改端口** → 同步 `PORT-REGISTRY.md` → `verify-ports.py` 必须通过 → 若涉及部署，同步 `deploy/` 配置。
3. **改协议** → 提 ADR（放 `../enterprise/`，命名 `*-ADR-<编号>.md`）→ 更新本目录文档与相关模块文档。
