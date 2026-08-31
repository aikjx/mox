// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 文档专家（骨架 · TODO：后续迭代补全完整实现）

use crate::context::ExpertContext;
use crate::expert::Expert;
use mox_ai_expert_proto::{Dimension, ExpertId, ExpertOpinion};

pub struct DocumentationExpert;

impl Expert for DocumentationExpert {
    fn id(&self) -> ExpertId {
        "documentation".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Documentation
    }
    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        if ctx.code_ir.is_none() {
            return ExpertOpinion::skipped(
                "documentation",
                Dimension::Documentation,
                "无代码 IR，开发璇玑跳过",
            );
        }
        ExpertOpinion::empty("documentation", Dimension::Documentation)
    }
}
