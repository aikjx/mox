# 璇玑 RelGraph · self_sync_all 模块化贯通验收报告

> 生成：2026-08-23T13:31:58.969Z · STRICT=OFF

## 0. 总览
| 指标 | 值 | 目标 | 状态 |
|------|:----:|:----:|:----:|
| 模块标准卡（6 字段）总数 | 76 | ≥ 75 | ✅ |
| Schema 校验错误数 | 0 | 0 | ✅ |
| 骨架标记卡数量（__skeleton） | 76 | 0 | 📋 骨架阶段 |
| TODO 占位字段数 | 88 | 0 | 📋 骨架阶段 |
| P9 判重闸门（moduleId 唯一） | 0 重复 | 0 重复 | ✅ |
| 图谱节点数（Entity） | 76 | ≥ 483 | 📋（当前骨架 75 模块 + 现有 289 = 约 364 目标阶段 483） |
| 7 类边总数 | 315 | ≥ 410 | 📋（骨架阶段） |
| 每模块平均边数 | 4.14 | ≥ 5.5 | ⚠️ |
| 模块级贯通率（有边模块 / 总模块） | 100.0% | ≥ 100% | ✅ |

## 1. 输出文件
- (A) `platform\backend-node\data\all_module_cards.json`（53 KB）
- (B) `platform\backend-node\data\graph_modules.nodes.json`（kg-hub 节点形态）
- (C) `platform\backend-node\data\graph_modules.edges.json`（7 类边）
- (D) 本报告 `platform\backend-node\data\all_module_cards.report.md`

## 2. 后续 TODO（Day 1~2）
1. 替换 16 Rust Crate 的 src/lib.rs 三常量解析，去掉 __skeleton=true
2. Node 8 域读取各域 index.js + 补 6 字段真实 raci/upstreamDownstream
3. Frontend 28 视图写 views/_cards/*.module.md 6 字段卡
4. Enterprise 文档 24 份读取 00-INDEX 主责列，补 450 字段真实值
5. upstreamDownstream 手工填写真实上下游（6 大类依赖关系）
6. --strict 0 exit 全绿 → 进入 kg-hub ingest