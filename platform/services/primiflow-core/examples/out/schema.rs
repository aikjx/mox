//! 数据设计 · 由关联图谱自动生成（primiflow_core::assoc::primiflow_seed）
//! 对应 primiflow/SPEC.md §4 数据模型
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DataR1 {
    pub id: Uuid,
    pub project_id: Uuid,
    pub graph_json: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DataR2 {
    pub id: Uuid,
    pub project_id: Uuid,
    pub graph_json: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DataR3 {
    pub id: Uuid,
    pub project_id: Uuid,
    pub graph_json: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DataR4 {
    pub id: Uuid,
    pub project_id: Uuid,
    pub graph_json: String,
    pub created_at: DateTime<Utc>,
}
