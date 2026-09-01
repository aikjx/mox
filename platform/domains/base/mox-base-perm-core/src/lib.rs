//! MOX 统一基座 · 节点级权限层
//!
//! 在 mox-rbac-engine（RBAC+ABAC）之上，提供**节点级 / 字段级 ACL**：
//! - 节点级：`node:kg:xxx` 粒度读写控制（知识图谱 / 文档 / 对象统一授权）
//! - 字段级：节点 props 内敏感字段的访问掩码（脱敏策略）
//! - 范围过滤：查询结果按当前主体权限裁剪（与 mox-base-query-core 联动）
//!
//! ## 设计原则
//! - 基于 mox-rbac-engine（平台级权限基座），不重复造轮子。
//! - data 域 mox-data-compliance-svc 复用本层 ACL 与脱敏策略。
//! - platform 域 IAM 注入本层（IAM 是角色/主体的来源）。

use async_trait::async_trait;
use mox_rbac_engine::{EvaluationResult, RbacEngine};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 权限错误
#[derive(Debug, Error)]
pub enum PermError {
    #[error("节点级权限拒绝: 主体 {subject} 对 {resource} 无 {action} 权限")]
    Denied {
        subject: String,
        resource: String,
        action: String,
    },
    #[error("字段脱敏: 主体 {subject} 无权访问字段 {field}")]
    FieldDenied { subject: String, field: String },
    #[error("其他错误: {0}")]
    Other(String),
}

/// 权限结果
pub type PermResult<T> = Result<T, PermError>;

/// 节点级访问动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeAction {
    /// 读取
    Read,
    /// 写入
    Write,
    /// 删除
    Delete,
    /// 遍历（沿边）
    Traverse,
}

impl NodeAction {
    /// 转为 RBAC action 字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeAction::Read => "read",
            NodeAction::Write => "write",
            NodeAction::Delete => "delete",
            NodeAction::Traverse => "traverse",
        }
    }
}

/// 节点级权限检查器 trait
///
/// 各域实现此 trait（内部可持有 RbacEngine 实例），对外提供统一节点授权。
#[async_trait]
pub trait NodePerm: Send + Sync {
    /// 检查主体对节点的动作权限
    async fn authorize(
        &self,
        subject: &str,
        node_id: &str,
        action: NodeAction,
    ) -> PermResult<()>;

    /// 检查字段级访问（脱敏）
    async fn check_field(&self, subject: &str, field: &str) -> PermResult<()>;

    /// 过滤节点 ID 列表：仅保留主体有权读取的节点
    async fn filter_readable(&self, subject: &str, node_ids: &[String]) -> PermResult<Vec<String>>;
}

/// 基于 mox-rbac-engine 的节点权限实现
///
/// 资源路径规则：`node:<domain>/<kind>/*`（如 `node:kg/node/*`），
/// 复用 RBAC 引擎的 action 语义（read/write/delete）。
pub struct RbacNodePerm {
    engine: RbacEngine,
}

impl Default for RbacNodePerm {
    fn default() -> Self {
        Self::new()
    }
}

impl RbacNodePerm {
    /// 新建 RBAC 节点权限（内置角色）
    pub fn new() -> Self {
        Self {
            engine: RbacEngine::with_builtin_roles(),
        }
    }

    /// 将节点 ID 映射为 RBAC 资源路径
    fn resource_path(node_id: &str) -> String {
        format!("node:{}", node_id)
    }
}

#[async_trait]
impl NodePerm for RbacNodePerm {
    async fn authorize(
        &self,
        subject: &str,
        node_id: &str,
        action: NodeAction,
    ) -> PermResult<()> {
        let resource = Self::resource_path(node_id);
        // subject 格式约定：`user:<role>`，取冒号后部分作为 RBAC 角色
        let role = subject.rsplit(':').next().unwrap_or("viewer");
        let result = self
            .engine
            .check(subject, &[role.to_string()], action.as_str(), &resource);
        match result {
            EvaluationResult::Granted { .. } => Ok(()),
            EvaluationResult::Denied { .. } => Err(PermError::Denied {
                subject: subject.to_string(),
                resource,
                action: action.as_str().to_string(),
            }),
        }
    }

    async fn check_field(&self, subject: &str, field: &str) -> PermResult<()> {
        // 字段级：敏感字段需显式授权（这里简化为 field 前缀规则，生产可接入 ABAC 条件）
        if field.starts_with("secret:") && !subject.starts_with("admin") {
            return Err(PermError::FieldDenied {
                subject: subject.to_string(),
                field: field.to_string(),
            });
        }
        Ok(())
    }

    async fn filter_readable(&self, subject: &str, node_ids: &[String]) -> PermResult<Vec<String>> {
        let mut allowed = Vec::new();
        for id in node_ids {
            if self
                .authorize(subject, id, NodeAction::Read)
                .await
                .is_ok()
            {
                allowed.push(id.clone());
            }
        }
        Ok(allowed)
    }
}

/// 节点级 ACL（deny-overrides：显式 deny 优先于 allow）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeAcl {
    /// subject -> action set
    allow: std::collections::HashMap<String, std::collections::HashSet<String>>,
    /// subject -> action set（deny 优先）
    deny: std::collections::HashMap<String, std::collections::HashSet<String>>,
}

impl NodeAcl {
    /// 新建 ACL
    pub fn new() -> Self {
        Self::default()
    }

    /// 授予主体某动作
    pub fn allow(&mut self, subject: impl Into<String>, action: NodeAction) -> &mut Self {
        self.allow
            .entry(subject.into())
            .or_default()
            .insert(action.as_str().to_string());
        self
    }

    /// 拒绝主体某动作
    pub fn deny(&mut self, subject: impl Into<String>, action: NodeAction) -> &mut Self {
        self.deny
            .entry(subject.into())
            .or_default()
            .insert(action.as_str().to_string());
        self
    }

    /// 检查主体对动作是否有权限
    pub fn is_allowed(&self, subject: &str, action: NodeAction) -> bool {
        let act = action.as_str();
        // deny 优先
        if self
            .deny
            .get(subject)
            .map(|s| s.contains(act))
            .unwrap_or(false)
        {
            return false;
        }
        // 无 allow 记录默认拒绝
        self.allow
            .get(subject)
            .map(|s| s.contains(act))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn admin_can_authorize() {
        let perm = RbacNodePerm::new();
        let r = perm
            .authorize("user:admin", "node:kg/n1", NodeAction::Read)
            .await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn field_denied_for_non_admin() {
        let perm = RbacNodePerm::new();
        let r = perm.check_field("user:alice", "secret:phone").await;
        assert!(r.is_err());
        let ok = perm.check_field("admin:root", "secret:phone").await;
        assert!(ok.is_ok());
    }

    #[test]
    fn acl_deny_overrides_allow() {
        let mut acl = NodeAcl::new();
        acl.allow("alice", NodeAction::Read);
        acl.deny("alice", NodeAction::Read);
        assert!(!acl.is_allowed("alice", NodeAction::Read));
        assert!(!acl.is_allowed("bob", NodeAction::Read));
    }

    #[test]
    fn acl_allow_works() {
        let mut acl = NodeAcl::new();
        acl.allow("alice", NodeAction::Read);
        assert!(acl.is_allowed("alice", NodeAction::Read));
        assert!(!acl.is_allowed("alice", NodeAction::Write));
    }

    #[test]
    fn node_action_str_mapping() {
        assert_eq!(NodeAction::Read.as_str(), "read");
        assert_eq!(NodeAction::Write.as_str(), "write");
        assert_eq!(NodeAction::Delete.as_str(), "delete");
        assert_eq!(NodeAction::Traverse.as_str(), "traverse");
    }
}
