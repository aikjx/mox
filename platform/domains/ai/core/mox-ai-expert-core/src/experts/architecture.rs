// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 架构专家（骨架 · TODO：后续迭代补全完整实现）

use crate::context::ExpertContext;
use crate::expert::Expert;
use mox_ai_expert_proto::{Dimension, ExpertId, ExpertOpinion};

pub struct ArchitectureExpert;

impl Expert for ArchitectureExpert {
    fn id(&self) -> ExpertId {
        "architecture".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Architecture
    }
    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        // TODO(P2 阶段 4 后续迭代)：迁移完整架构专家逻辑
        // 开发璇玑专家：无代码 IR 时 skipped
        if ctx.code_ir.is_none() {
            return ExpertOpinion::skipped(
                "architecture",
                Dimension::Architecture,
                "无代码 IR，开发璇玑跳过",
            );
        }
        ExpertOpinion::empty("architecture", Dimension::Architecture)
    }
}
