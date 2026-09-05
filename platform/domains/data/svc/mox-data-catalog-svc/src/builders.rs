// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_ai_flow_sdk::model::{ExpertRule, FlowNode, NodeKind, Severity};

/// Guard 节点（校验/脱敏/审计，无外部工具）
pub(crate) fn guard(id: &str, name: &str, ms: u64) -> FlowNode {
    FlowNode::new(id, name, NodeKind::Guard).with_duration(ms)
}

/// 节点构造辅助
pub(crate) fn start(id: &str) -> FlowNode {
    FlowNode::new(id, id, NodeKind::Start)
}

pub(crate) fn end(id: &str) -> FlowNode {
    FlowNode::new(id, id, NodeKind::End)
}

/// 给节点设耗时（FlowNode::new 默认 0）
pub(crate) trait WithDuration {
    fn with_duration(self, ms: u64) -> FlowNode;
}

impl WithDuration for FlowNode {
    fn with_duration(mut self, ms: u64) -> FlowNode {
        self.duration_ms = ms;
        self
    }
}

/// 合规规则构造辅助（基础版，无 required_guard_tags）
pub(crate) fn rule(id: &str, desc: &str, prefixes: &[&str]) -> ExpertRule {
    ExpertRule {
        id: id.into(),
        description: desc.into(),
        severity: Severity::Blocking,
        resource_prefixes: prefixes.iter().map(|s| s.to_string()).collect(),
        tool_kinds: Vec::new(),
        required_guard_tags: Vec::new(),
    }
}
