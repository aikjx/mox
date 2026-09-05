// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 业务全景目录：把"系统所有业务"建模成流程图 + 六维关系网，
//! 并用璇玑（mox-expert）在运行中不断优化架构。
//!
//! 核心思想（与 Hermes / 璇玑架构一致）：
//! - **流程图是唯一需求源与开发产物**：每个业务 = 一张 `mox_ai_flow_sdk::FlowGraph`，
//!   `tags` 携带 `dim:algo|perm|res|sec|data|obs` 做七维着色。
//! - **关系图是跨业务的六维知识网**：所有业务 flow 经 `TopologyGraph::ingest_flow`
//!   汇入同一张图，叠加 Skill/Rule/Memory/Model 实体与 Binds/Recalls/Constrains/Serves 关系。
//! - **使用中不断优化**：`record_hit`/`decay` 做动态权重学习；`impact_of` 做改一节点全链路
//!   同步；`route`/`shortest_path` 做跨业务复用最短路径（命中历史 Skill → 跳过完整 ReAct）。
//!
//! 【DIP 改造】本 crate 生产代码路径不再直接 `use mox_ai_expert_svc::pipeline`
//! （或 context/ir/... 等内部模块）。对外统一依赖：
//! - `mox_ai_expert_svc::expert_traits::{ExpertConsultant, ExpertRegistry, AllianceOrchestrator, ...}` 抽象 trait
//! - `mox_ai_expert_svc::types::{ConsultQuery, ConsultReport, ExpertMeta, ...}` 投影数据类型
//! - 需要「查询专家清单 / 注册专家」处统一用 `Arc<dyn ExpertRegistry>`。
//!
//! 从而实现依赖方向反转：`business-catalog → trait ← mox concrete`。

// ============================================================================
// 模块声明
// ============================================================================

pub mod spiral;

mod builders;
pub mod business;
pub mod constants;
pub mod flows;
pub mod topology;

#[cfg(test)]
mod tests;

// ============================================================================
// 公开 API 重导出（保持向后兼容）
// ============================================================================

pub use constants::{CRATE_ID, CRATE_META, ENGINE_NAME};
pub use business::{register_business_experts, Business};
pub use flows::all_businesses;
pub use topology::build_topology;
