//! 层6 · 真实持久化层（数据层）
//!
//! 把 κ‑τ 引擎的**[知识库（拓扑荷 Q 沉淀资产）]** 与 **[六维溯源主图]** 真实落库，
//! 支持进程重启后**重放**恢复到引擎，实现「需求‑功能‑算法‑业务流‑代码」全链路可审计、可复现。
//!
//! 存储后端二选一（统一 `Persistence` 接口）：
//! - `Memory`：进程内存储，零外部依赖，适合测试与嵌入式；
//! - `Sqlite`：基于 `mox-system::PersistenceProvider`（底层通过 `SqlitePersistence` 落盘到文件），生产可用。
//!
//! `TopologyGraph` 与 `AssocGraph` 均 `derive(Serialize/Deserialize)`，因此可**精确**序列化 /
//! 反序列化，重放后引擎状态与落库前逐字节一致。

use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use mox_ai_flow_svc::primitive::{KnowledgeBase, PrimiEngine, StoredTopology};
use mox_ai_flow_svc::topology::TopologyGraph;
use serde::{Deserialize, Serialize};
use mox_platform_system_core::persistence_provider::{PersistenceProvider, SqlRow, SqlValue};
use mox_platform_system_core::sqlite_provider::SqlitePersistence;

use crate::assoc::AssocGraph;
use crate::runner::PipelineReport;

/// 从 [`SqlRow`] 中提取指定列的文本值（缺失或不匹配 → 默认空字符串）
fn take_text(row: &SqlRow, col: &str) -> String {
    match row.get(col) {
        Some(SqlValue::Text(s)) => s.clone(),
        _ => String::new(),
    }
}

/// 从 [`SqlRow`] 中提取指定列的浮点值（缺失或不匹配 → 0.0）
fn take_real(row: &SqlRow, col: &str) -> f64 {
    match row.get(col) {
        Some(SqlValue::Real(f)) => *f,
        Some(SqlValue::Int(i)) => *i as f64,
        _ => 0.0,
    }
}

/// 从 [`SqlRow`] 中提取指定列的整数值（缺失或不匹配 → 0）
fn take_int(row: &SqlRow, col: &str) -> i64 {
    match row.get(col) {
        Some(SqlValue::Int(i)) => *i,
        Some(SqlValue::Real(f)) => *f as i64,
        Some(SqlValue::Bool(true)) => 1,
        Some(SqlValue::Bool(false)) => 0,
        _ => 0,
    }
}

/// 项目运行记录（落库后可审计、可复现）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub policy: String,
    pub kappa: f64,
    pub tau: f64,
    pub conserved: bool,
    pub acyclic: bool,
    pub reused: usize,
    pub regularized: bool,
    pub q_before: f64,
    pub q_after: f64,
    pub bound_nodes: usize,
    pub bound_edges: usize,
    pub created_at: String,
}

impl ProjectRecord {
    /// 从一次闭环报告生成项目记录
    pub fn from_report(id: &str, rep: &PipelineReport) -> Self {
        Self {
            id: id.to_string(),
            name: rep.requirement.clone(),
            policy: rep.policy.to_string(),
            kappa: rep.kappa,
            tau: rep.tau,
            conserved: rep.conserved,
            acyclic: rep.acyclic,
            reused: rep.reused,
            regularized: rep.regularized,
            q_before: rep.q_before,
            q_after: rep.q_after,
            bound_nodes: rep.bound_nodes,
            bound_edges: rep.bound_edges,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

/// 持久化后端
pub enum Persistence {
    /// 进程内存储（无外部依赖）
    Memory {
        assets: Vec<StoredTopology>,
        kb_graph_json: Option<String>,
        trace_graph_json: Option<String>,
        projects: Vec<ProjectRecord>,
    },
    /// 落盘 SQLite（通过 mox-system 的 PersistenceProvider trait，解耦 rusqlite driver）
    Sqlite {
        provider: Arc<dyn PersistenceProvider>,
        path: Option<String>,
    },
}

const SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS kb_assets(
    id TEXT, signature TEXT, charge REAL, reuse_count INTEGER);
 CREATE TABLE IF NOT EXISTS kb_graph(
    single INTEGER PRIMARY KEY, graph_json TEXT);
 CREATE TABLE IF NOT EXISTS trace(
    single INTEGER PRIMARY KEY, graph_json TEXT);
 CREATE TABLE IF NOT EXISTS projects(
    id TEXT PRIMARY KEY, name TEXT, policy TEXT, kappa REAL, tau REAL,
    conserved INTEGER, acyclic INTEGER, reused INTEGER, regularized INTEGER,
    q_before REAL, q_after REAL, bound_nodes INTEGER, bound_edges INTEGER,
    created_at TEXT);";

impl Persistence {
    /// 内存后端（测试/嵌入式）
    pub fn memory() -> Self {
        Persistence::Memory {
            assets: Vec::new(),
            kb_graph_json: None,
            trace_graph_json: None,
            projects: Vec::new(),
        }
    }

    /// 打开（或创建）SQLite 文件后端
    pub fn sqlite(path: &str) -> Result<Self> {
        let pvd = SqlitePersistence::file(path)?;
        pvd.exec_batch(SCHEMA_SQL)?;
        Ok(Persistence::Sqlite {
            provider: Arc::new(pvd),
            path: Some(path.to_string()),
        })
    }

    /// 内存后端也可用 `:memory:` 语义的 SQLite（便于统一测试）
    pub fn sqlite_memory() -> Result<Self> {
        let pvd = SqlitePersistence::memory()?;
        pvd.exec_batch(SCHEMA_SQL)?;
        Ok(Persistence::Sqlite {
            provider: Arc::new(pvd),
            path: None,
        })
    }

    /// 保存知识库（拓扑荷 Q 资产 + 六维关系网）
    pub fn save_kb(&mut self, kb: &KnowledgeBase) -> Result<()> {
        match self {
            Persistence::Memory {
                assets,
                kb_graph_json,
                ..
            } => {
                *assets = kb.stored.clone();
                *kb_graph_json = Some(serde_json::to_string(&kb.graph)?);
                Ok(())
            }
            Persistence::Sqlite { provider, .. } => {
                provider.exec("DELETE FROM kb_assets", &[])?;
                for a in &kb.stored {
                    provider.exec(
                        "INSERT INTO kb_assets(id, signature, charge, reuse_count) VALUES(?1,?2,?3,?4)",
                        &[
                            SqlValue::Text(a.id.clone()),
                            SqlValue::Text(a.signature.clone()),
                            SqlValue::Real(a.charge),
                            SqlValue::Int(a.reuse_count as i64),
                        ],
                    )?;
                }
                let g = serde_json::to_string(&kb.graph)?;
                provider.exec(
                    "INSERT INTO kb_graph(single, graph_json) VALUES(1,?1) \
                     ON CONFLICT(single) DO UPDATE SET graph_json=excluded.graph_json",
                    &[SqlValue::Text(g)],
                )?;
                Ok(())
            }
        }
    }

    /// 载入知识库（重放核心）
    pub fn load_kb(&self) -> Result<KnowledgeBase> {
        match self {
            Persistence::Memory {
                assets,
                kb_graph_json,
                ..
            } => {
                let graph: TopologyGraph = match kb_graph_json {
                    Some(g) => serde_json::from_str(g)?,
                    None => Default::default(),
                };
                Ok(KnowledgeBase {
                    graph,
                    stored: assets.clone(),
                })
            }
            Persistence::Sqlite { provider, .. } => {
                let mut graph: TopologyGraph = Default::default();
                if let Some(row) =
                    provider.query_one("SELECT graph_json FROM kb_graph WHERE single=1", &[])?
                {
                    if let Some(SqlValue::Text(g)) = row.get("graph_json") {
                        graph = serde_json::from_str(g)?;
                    }
                }
                let rows = provider.query(
                    "SELECT id, signature, charge, reuse_count FROM kb_assets",
                    &[],
                )?;
                let mut stored = Vec::with_capacity(rows.len());
                for row in rows {
                    stored.push(StoredTopology {
                        id: take_text(&row, "id"),
                        signature: take_text(&row, "signature"),
                        charge: take_real(&row, "charge"),
                        reuse_count: take_int(&row, "reuse_count") as u64,
                    });
                }
                Ok(KnowledgeBase { graph, stored })
            }
        }
    }

    /// 保存六维溯源主图
    pub fn save_graph(&mut self, g: &AssocGraph) -> Result<()> {
        let json = serde_json::to_string(g)?;
        match self {
            Persistence::Memory {
                trace_graph_json, ..
            } => {
                *trace_graph_json = Some(json);
                Ok(())
            }
            Persistence::Sqlite { provider, .. } => {
                provider.exec(
                    "INSERT INTO trace(single, graph_json) VALUES(1,?1) \
                     ON CONFLICT(single) DO UPDATE SET graph_json=excluded.graph_json",
                    &[SqlValue::Text(json)],
                )?;
                Ok(())
            }
        }
    }

    /// 载入六维溯源主图（重放）
    pub fn load_graph(&self) -> Result<AssocGraph> {
        let json = match self {
            Persistence::Memory {
                trace_graph_json, ..
            } => trace_graph_json.clone(),
            Persistence::Sqlite { provider, .. } => {
                let mut out = None;
                if let Some(row) =
                    provider.query_one("SELECT graph_json FROM trace WHERE single=1", &[])?
                {
                    if let Some(SqlValue::Text(g)) = row.get("graph_json") {
                        out = Some(g.clone());
                    }
                }
                out
            }
        };
        match json {
            Some(g) => Ok(serde_json::from_str(&g)?),
            None => Ok(AssocGraph::new()),
        }
    }

    /// 保存项目运行记录
    pub fn save_project(&mut self, rec: &ProjectRecord) -> Result<()> {
        match self {
            Persistence::Memory { projects, .. } => {
                projects.retain(|p| p.id != rec.id);
                projects.push(rec.clone());
                Ok(())
            }
            Persistence::Sqlite { provider, .. } => {
                provider.exec(
                    "INSERT INTO projects(id,name,policy,kappa,tau,conserved,acyclic,reused,regularized,q_before,q_after,bound_nodes,bound_edges,created_at)\
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)\
                     ON CONFLICT(id) DO UPDATE SET \
                       name=excluded.name, policy=excluded.policy, kappa=excluded.kappa, tau=excluded.tau,\
                       conserved=excluded.conserved, acyclic=excluded.acyclic, reused=excluded.reused,\
                       regularized=excluded.regularized, q_before=excluded.q_before, q_after=excluded.q_after,\
                       bound_nodes=excluded.bound_nodes, bound_edges=excluded.bound_edges, created_at=excluded.created_at",
                    &[
                        SqlValue::Text(rec.id.clone()),
                        SqlValue::Text(rec.name.clone()),
                        SqlValue::Text(rec.policy.clone()),
                        SqlValue::Real(rec.kappa),
                        SqlValue::Real(rec.tau),
                        SqlValue::Int(rec.conserved as i64),
                        SqlValue::Int(rec.acyclic as i64),
                        SqlValue::Int(rec.reused as i64),
                        SqlValue::Int(rec.regularized as i64),
                        SqlValue::Real(rec.q_before),
                        SqlValue::Real(rec.q_after),
                        SqlValue::Int(rec.bound_nodes as i64),
                        SqlValue::Int(rec.bound_edges as i64),
                        SqlValue::Text(rec.created_at.clone()),
                    ],
                )?;
                Ok(())
            }
        }
    }

    /// 列出全部项目记录（按时间倒序）
    pub fn list_projects(&self) -> Result<Vec<ProjectRecord>> {
        match self {
            Persistence::Memory { projects, .. } => Ok(projects.clone()),
            Persistence::Sqlite { provider, .. } => {
                let rows = provider.query(
                    "SELECT id,name,policy,kappa,tau,conserved,acyclic,reused,regularized,\
                            q_before,q_after,bound_nodes,bound_edges,created_at FROM projects \
                     ORDER BY created_at DESC",
                    &[],
                )?;
                let mut out = Vec::with_capacity(rows.len());
                for row in rows {
                    out.push(ProjectRecord {
                        id: take_text(&row, "id"),
                        name: take_text(&row, "name"),
                        policy: take_text(&row, "policy"),
                        kappa: take_real(&row, "kappa"),
                        tau: take_real(&row, "tau"),
                        conserved: take_int(&row, "conserved") != 0,
                        acyclic: take_int(&row, "acyclic") != 0,
                        reused: take_int(&row, "reused") as usize,
                        regularized: take_int(&row, "regularized") != 0,
                        q_before: take_real(&row, "q_before"),
                        q_after: take_real(&row, "q_after"),
                        bound_nodes: take_int(&row, "bound_nodes") as usize,
                        bound_edges: take_int(&row, "bound_edges") as usize,
                        created_at: take_text(&row, "created_at"),
                    });
                }
                Ok(out)
            }
        }
    }

    /// 按 ID 查询单个项目记录（不存在返回 `Ok(None)`）
    pub fn get_project(&self, id: &str) -> Result<Option<ProjectRecord>> {
        match self {
            Persistence::Memory { projects, .. } => {
                Ok(projects.iter().find(|p| p.id == id).cloned())
            }
            Persistence::Sqlite { provider, .. } => {
                let row = provider.query_one(
                    "SELECT id,name,policy,kappa,tau,conserved,acyclic,reused,regularized,\
                            q_before,q_after,bound_nodes,bound_edges,created_at FROM projects \
                     WHERE id=?1",
                    &[SqlValue::Text(id.to_string())],
                )?;
                Ok(row.map(|row| ProjectRecord {
                    id: take_text(&row, "id"),
                    name: take_text(&row, "name"),
                    policy: take_text(&row, "policy"),
                    kappa: take_real(&row, "kappa"),
                    tau: take_real(&row, "tau"),
                    conserved: take_int(&row, "conserved") != 0,
                    acyclic: take_int(&row, "acyclic") != 0,
                    reused: take_int(&row, "reused") as usize,
                    regularized: take_int(&row, "regularized") != 0,
                    q_before: take_real(&row, "q_before"),
                    q_after: take_real(&row, "q_after"),
                    bound_nodes: take_int(&row, "bound_nodes") as usize,
                    bound_edges: take_int(&row, "bound_edges") as usize,
                    created_at: take_text(&row, "created_at"),
                }))
            }
        }
    }

    /// 闭环后落库：知识库 + 溯源图 + 项目记录 一并持久化
    pub fn persist_pipeline(
        &mut self,
        engine: &PrimiEngine,
        master: &AssocGraph,
        project_id: &str,
        rep: &PipelineReport,
    ) -> Result<()> {
        self.save_kb(&engine.kb)?;
        self.save_graph(master)?;
        self.save_project(&ProjectRecord::from_report(project_id, rep))?;
        Ok(())
    }

    /// 重放：把落库的知识库与溯源图恢复到引擎与主图（进程重启后继续复用成熟链路）
    pub fn replay_into(&self, engine: &mut PrimiEngine, master: &mut AssocGraph) -> Result<()> {
        engine.kb = self.load_kb()?;
        *master = self.load_graph()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{enterprise_specs, run_all};
    use mox_ai_flow_svc::primitive::ResourceBudget;

    fn memory_engine() -> PrimiEngine {
        PrimiEngine::new(10.0, KnowledgeBase::new(), ResourceBudget::default())
    }

    #[test]
    fn sqlite_roundtrip_kb_and_graph() {
        let dir = std::env::temp_dir().join(format!("primiflow_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("store.db").to_string_lossy().to_string();
        let mut store = Persistence::sqlite(&path).unwrap();

        let mut engine = memory_engine();
        let master = AssocGraph::new();
        let specs = enterprise_specs();
        let reports = run_all(&mut engine, &specs, dir.as_path()).unwrap();
        // 落库
        for (i, rep) in reports.iter().enumerate() {
            store
                .persist_pipeline(&engine, &master, &format!("p{i}"), rep)
                .unwrap();
        }
        let assets_before = engine.kb.stored.len();
        assert!(assets_before >= 1, "应已沉淀拓扑资产");

        // 重放：全新引擎 + 空主图，从落库恢复
        let mut engine2 = memory_engine();
        let mut master2 = AssocGraph::new();
        store.replay_into(&mut engine2, &mut master2).unwrap();

        assert_eq!(engine2.kb.stored.len(), assets_before, "资产应完整恢复");
        assert_eq!(
            master2.nodes.len(),
            master.nodes.len(),
            "溯源节点应完整恢复"
        );
        assert_eq!(master2.edges.len(), master.edges.len(), "溯源边应完整恢复");
        // 知识库图谱实体也应恢复（六维关系网可继续检索复用）
        assert_eq!(
            engine2.kb.graph.entities.len(),
            engine.kb.graph.entities.len()
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn memory_store_is_functional() {
        let mut store = Persistence::memory();
        let mut engine = memory_engine();
        let master = AssocGraph::new();
        let specs = enterprise_specs();
        let reports = run_all(
            &mut engine,
            &specs,
            std::path::Path::new(&std::env::temp_dir().join("pf_mem")),
        )
        .unwrap();
        for (i, rep) in reports.iter().enumerate() {
            store
                .persist_pipeline(&engine, &master, &format!("p{i}"), rep)
                .unwrap();
        }
        assert!(store.list_projects().unwrap().len() >= 4);
        let mut engine2 = memory_engine();
        let mut master2 = AssocGraph::new();
        store.replay_into(&mut engine2, &mut master2).unwrap();
        assert_eq!(engine2.kb.stored.len(), engine.kb.stored.len());
    }

    #[test]
    fn sqlite_memory_roundtrip() {
        let mut store = Persistence::sqlite_memory().unwrap();

        let mut engine = memory_engine();
        let master = AssocGraph::new();
        let specs = enterprise_specs();
        let reports = run_all(
            &mut engine,
            &specs,
            std::path::Path::new(&std::env::temp_dir().join("pf_sqlmem")),
        )
        .unwrap();
        for (i, rep) in reports.iter().enumerate() {
            store
                .persist_pipeline(&engine, &master, &format!("p{i}"), rep)
                .unwrap();
        }
        let assets_before = engine.kb.stored.len();
        assert!(assets_before >= 1);

        // 同一连接内重放
        let mut engine2 = memory_engine();
        let mut master2 = AssocGraph::new();
        store.replay_into(&mut engine2, &mut master2).unwrap();
        assert_eq!(engine2.kb.stored.len(), assets_before);
        assert_eq!(master2.nodes.len(), master.nodes.len());
        assert_eq!(master2.edges.len(), master.edges.len());
    }

    #[test]
    fn file_persistence_survives_reopen() {
        let dir = std::env::temp_dir().join(format!("primiflow_file_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("store.db").to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path);

        // 第一次连接：写
        {
            let mut store = Persistence::sqlite(&path).unwrap();
            let mut engine = memory_engine();
            let master = AssocGraph::new();
            let specs = enterprise_specs();
            let reports = run_all(&mut engine, &specs, dir.as_path()).unwrap();
            for (i, rep) in reports.iter().enumerate() {
                store
                    .persist_pipeline(&engine, &master, &format!("p{i}"), rep)
                    .unwrap();
            }
            assert!(store.list_projects().unwrap().len() >= 4);
        }
        // 证明文件确实落盘
        assert!(std::path::Path::new(&path).exists(), "SQLite 文件应已落盘");

        // 第二次连接：重新打开同一文件，重放恢复
        {
            let pvd = SqlitePersistence::file(&path).unwrap();
            let store = Persistence::Sqlite {
                provider: Arc::new(pvd),
                path: Some(path.clone()),
            };
            let mut engine2 = memory_engine();
            let mut master2 = AssocGraph::new();
            store.replay_into(&mut engine2, &mut master2).unwrap();
            assert!(!engine2.kb.stored.is_empty(), "重开文件后应恢复资产");
            assert!(
                store.list_projects().unwrap().len() >= 4,
                "重开文件后应恢复项目记录"
            );
        }

        let _ = std::fs::remove_file(&path);
    }
}
