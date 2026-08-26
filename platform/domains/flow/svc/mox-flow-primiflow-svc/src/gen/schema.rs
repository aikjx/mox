//! 数据设计 · 由关联图谱自动生成（mox_flow_primiflow_svc::assoc::primiflow_seed）
//! 对应 primiflow/SPEC.md §4 数据模型
//!
//! 本文件是 PrimiFlow 全平台的**统一数据载体**：八层模块（C1–C8）全部围绕这 6 张表
//! 与 2 个状态/类型枚举读写，保证「图是图、代码是代码、数据是数据」三态同源。
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 拓扑生命周期状态（SPEC §4 `topologies.status`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyStatus {
    /// 草稿：拓扑已涌现但尚未通过 ℛ̂
    Draft,
    /// 已正则化：通过 ℛ̂ 校验，可交付画布
    Regularized,
    /// 已冻结：作为资产 Q 入库，供 κ 复用
    Frozen,
    /// 已驳回：存在矛盾环 / 幻觉，回写对话重生成
    Rejected,
}

impl TopologyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TopologyStatus::Draft => "draft",
            TopologyStatus::Regularized => "regularized",
            TopologyStatus::Frozen => "frozen",
            TopologyStatus::Rejected => "rejected",
        }
    }
    pub fn parse(s: &str) -> TopologyStatus {
        match s {
            "regularized" => TopologyStatus::Regularized,
            "frozen" => TopologyStatus::Frozen,
            "rejected" => TopologyStatus::Rejected,
            _ => TopologyStatus::Draft,
        }
    }
}

/// 八份标准化说明书种类（SPEC §8 `artifacts.kind`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// 1 需求规格说明书
    RequirementSpec,
    /// 2 功能设计说明书
    FeatureDesign,
    /// 3 业务流程说明书
    BusinessProcess,
    /// 4 数据模型说明书
    DataModel,
    /// 5 接口契约说明书
    ApiContract,
    /// 6 定时任务说明书
    ScheduledTask,
    /// 7 代码工程说明书（骨架/桩）
    CodeProject,
    /// 8 部署运维说明书
    Deployment,
}

impl ArtifactKind {
    pub fn index(&self) -> u8 {
        match self {
            ArtifactKind::RequirementSpec => 1,
            ArtifactKind::FeatureDesign => 2,
            ArtifactKind::BusinessProcess => 3,
            ArtifactKind::DataModel => 4,
            ArtifactKind::ApiContract => 5,
            ArtifactKind::ScheduledTask => 6,
            ArtifactKind::CodeProject => 7,
            ArtifactKind::Deployment => 8,
        }
    }
    pub fn title(&self) -> &'static str {
        match self {
            ArtifactKind::RequirementSpec => "需求规格说明书",
            ArtifactKind::FeatureDesign => "功能设计说明书",
            ArtifactKind::BusinessProcess => "业务流程说明书",
            ArtifactKind::DataModel => "数据模型说明书",
            ArtifactKind::ApiContract => "接口契约说明书",
            ArtifactKind::ScheduledTask => "定时任务说明书",
            ArtifactKind::CodeProject => "代码工程说明书",
            ArtifactKind::Deployment => "部署运维说明书",
        }
    }
    pub fn parse(s: &str) -> Option<ArtifactKind> {
        match s {
            "requirement_spec" => Some(ArtifactKind::RequirementSpec),
            "feature_design" => Some(ArtifactKind::FeatureDesign),
            "business_process" => Some(ArtifactKind::BusinessProcess),
            "data_model" => Some(ArtifactKind::DataModel),
            "api_contract" => Some(ArtifactKind::ApiContract),
            "scheduled_task" => Some(ArtifactKind::ScheduledTask),
            "code_project" => Some(ArtifactKind::CodeProject),
            "deployment" => Some(ArtifactKind::Deployment),
            _ => None,
        }
    }
    /// 八类全枚举（文档生成遍历用）
    pub fn all() -> [ArtifactKind; 8] {
        [
            ArtifactKind::RequirementSpec,
            ArtifactKind::FeatureDesign,
            ArtifactKind::BusinessProcess,
            ArtifactKind::DataModel,
            ArtifactKind::ApiContract,
            ArtifactKind::ScheduledTask,
            ArtifactKind::CodeProject,
            ArtifactKind::Deployment,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub tenant_id: Option<String>,
    /// 默认滑块：{"k":0.7,"t":0.3}
    pub k_t_pref: String,
    pub budget_c: f32,
    pub created_at: DateTime<Utc>,
}

impl Project {
    pub fn new(
        name: impl Into<String>,
        tenant_id: Option<String>,
        k: f64,
        t: f64,
        budget_c: f32,
    ) -> Self {
        let k_t_pref = serde_json::json!({ "k": k, "t": t }).to_string();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            tenant_id,
            k_t_pref,
            budget_c,
            created_at: Utc::now(),
        }
    }
    /// 从默认偏好解析 κ、τ
    pub fn pref(&self) -> (f64, f64) {
        let v: serde_json::Value =
            serde_json::from_str(&self.k_t_pref).unwrap_or(serde_json::json!({"k":0.7,"t":0.3}));
        (
            v.get("k").and_then(|x| x.as_f64()).unwrap_or(0.7),
            v.get("t").and_then(|x| x.as_f64()).unwrap_or(0.3),
        )
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub role: String,
    pub content: String,
    /// 含滑块 s、κ、τ、C 的 JSON
    pub meta: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Conversation {
    pub fn user(project_id: Uuid, content: impl Into<String>, meta: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            role: "user".into(),
            content: content.into(),
            meta,
            created_at: Utc::now(),
        }
    }
    pub fn assistant(project_id: Uuid, content: impl Into<String>, meta: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            role: "assistant".into(),
            content: content.into(),
            meta,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topology {
    pub id: Uuid,
    pub project_id: Uuid,
    pub status: String,
    pub k: f32,
    pub t: f32,
    pub c: f32,
    /// ℛ̂ 后的残差 Δ
    pub residual_delta: f32,
    pub graph_json: String,
    pub created_at: DateTime<Utc>,
}

impl Topology {
    pub fn new(
        project_id: Uuid,
        k: f64,
        t: f64,
        c: f64,
        delta: f64,
        graph_json: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            status: TopologyStatus::Draft.as_str().into(),
            k: k as f32,
            t: t as f32,
            c: c as f32,
            residual_delta: delta as f32,
            graph_json: graph_json.into(),
            created_at: Utc::now(),
        }
    }
    pub fn set_status(&mut self, s: TopologyStatus) {
        self.status = s.as_str().into();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: Uuid,
    pub topology_id: Uuid,
    pub name: String,
    /// 业务域标签，用于 κ 检索硬过滤
    pub domain: Option<String>,
    pub graph_json: String,
    pub frozen_at: DateTime<Utc>,
}

impl Asset {
    pub fn new(
        topology_id: Uuid,
        name: impl Into<String>,
        domain: Option<String>,
        graph_json: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            topology_id,
            name: name.into(),
            domain,
            graph_json: graph_json.into(),
            frozen_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: Uuid,
    pub project_id: Uuid,
    /// 见 `ArtifactKind`
    pub kind: String,
    pub title: String,
    /// Markdown / 代码
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl Artifact {
    pub fn new(
        project_id: Uuid,
        kind: ArtifactKind,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            kind: match kind {
                ArtifactKind::RequirementSpec => "requirement_spec",
                _ => "",
            }
            .to_string(),
            title: title.into(),
            content: content.into(),
            created_at: Utc::now(),
        }
    }
    /// 用枚举构造（避免上面的占位 bug）
    pub fn of(project_id: Uuid, kind: ArtifactKind, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            kind: match kind {
                ArtifactKind::RequirementSpec => "requirement_spec",
                ArtifactKind::FeatureDesign => "feature_design",
                ArtifactKind::BusinessProcess => "business_process",
                ArtifactKind::DataModel => "data_model",
                ArtifactKind::ApiContract => "api_contract",
                ArtifactKind::ScheduledTask => "scheduled_task",
                ArtifactKind::CodeProject => "code_project",
                ArtifactKind::Deployment => "deployment",
            }
            .to_string(),
            title: kind.title().into(),
            content: content.into(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceLink {
    pub id: Uuid,
    pub project_id: Uuid,
    pub requirement_id: String,
    pub feature_id: String,
    pub business_id: String,
    pub algorithm_id: String,
    pub task_id: String,
    pub code_id: String,
}

impl TraceLink {
    pub fn new(
        project_id: Uuid,
        requirement_id: impl Into<String>,
        feature_id: impl Into<String>,
        business_id: impl Into<String>,
        algorithm_id: impl Into<String>,
        task_id: impl Into<String>,
        code_id: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            requirement_id: requirement_id.into(),
            feature_id: feature_id.into(),
            business_id: business_id.into(),
            algorithm_id: algorithm_id.into(),
            task_id: task_id.into(),
            code_id: code_id.into(),
        }
    }
    /// 六维链路是否完整（每个维度都有绑定）
    pub fn is_complete(&self) -> bool {
        !self.requirement_id.is_empty()
            && !self.feature_id.is_empty()
            && !self.business_id.is_empty()
            && !self.algorithm_id.is_empty()
            && !self.task_id.is_empty()
            && !self.code_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_pref_roundtrip() {
        let p = Project::new("demo", Some("t1".into()), 0.7, 0.3, 1.0);
        let (k, t) = p.pref();
        assert!((k - 0.7).abs() < 1e-9);
        assert!((t - 0.3).abs() < 1e-9);
    }

    #[test]
    fn artifact_kind_all_present() {
        for k in ArtifactKind::all() {
            let a = Artifact::of(Uuid::new_v4(), k, "x");
            assert_eq!(ArtifactKind::parse(&a.kind), Some(k));
        }
    }

    #[test]
    fn trace_link_completeness() {
        let tl = TraceLink::new(Uuid::new_v4(), "R1", "F1", "B1", "A1", "T1", "C1");
        assert!(tl.is_complete());
        let tl2 = TraceLink::new(Uuid::new_v4(), "R1", "", "B1", "A1", "T1", "C1");
        assert!(!tl2.is_complete());
    }
}
