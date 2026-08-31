// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 测试专家（骨架 · TODO：后续迭代补全完整实现）

use crate::context::ExpertContext;
use crate::expert::Expert;
use mox_ai_expert_proto::{Dimension, ExpertId, ExpertOpinion};

pub struct TestingExpert;

impl Expert for TestingExpert {
    fn id(&self) -> ExpertId {
        "testing".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Testing
    }
    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        if ctx.code_ir.is_none() {
            return ExpertOpinion::skipped(
                "testing",
                Dimension::Testing,
                "无代码 IR，开发璇玑跳过",
            );
        }
        ExpertOpinion::empty("testing", Dimension::Testing)
    }
}
