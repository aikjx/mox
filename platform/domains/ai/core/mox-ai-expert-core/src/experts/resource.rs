// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 资源专家（骨架 · TODO：后续迭代补全完整实现）

use crate::context::ExpertContext;
use crate::expert::Expert;
use mox_ai_expert_proto::{Dimension, ExpertId, ExpertOpinion};

pub struct ResourceExpert;

impl Expert for ResourceExpert {
    fn id(&self) -> ExpertId {
        "resource".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Resource
    }
    fn analyze(&self, _ctx: &ExpertContext) -> ExpertOpinion {
        // TODO(P2 阶段 4 后续迭代)：迁移完整资源专家逻辑
        ExpertOpinion::empty("resource", Dimension::Resource)
    }
}
