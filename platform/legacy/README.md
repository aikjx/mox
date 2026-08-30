# ⚠️ DEPRECATED / 已废弃

本目录中的代码为**历史遗留实现**，已不再维护，仅作参考保留。

## 废弃说明

| 目录 | 原位置 | 废弃原因 | 替代方案 |
|------|--------|----------|----------|
| `mox-server/` | platform/mox-server | Python 版初代后端，已被 Rust 重写 | `platform/domains/` 下的 Rust 模块化架构 |
| `backend-rust/` | platform/backend-rust | 早期 Rust 后端原型，架构已重构 | `platform/domains/` + `platform/gateway/` |
| `mox-store/` | platform/mox-store | 商城服务原型，已整合入 market 域 | `platform/domains/market/` |

## 保留期限

这些代码将保留至 v3.0 正式发布后 3 个月，届时如无特殊需求将彻底删除。

## 注意事项

- ❌ 请勿在新代码中引用这些模块
- ❌ 请勿修复这些代码中的 bug
- ✅ 仅可作为历史参考和迁移对照使用
- ✅ 如有疑问请联系架构组
