// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 权限专家（骨架 · TODO：后续迭代补全完整实现）

use crate::context::ExpertContext;
use crate::expert::Expert;
use mox_ai_expert_proto::{Dimension, ExpertId, ExpertOpinion};

pub struct PermissionExpert;

impl Expert for PermissionExpert {
    fn id(&self) -> ExpertId {
        "permission".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Permission
    }
    fn analyze(&self, _ctx: &ExpertContext) -> ExpertOpinion {
        // TODO(P2 阶段 4 后续迭代)：迁移完整权限专家逻辑
        ExpertOpinion::empty("permission", Dimension::Permission)
    }
}
