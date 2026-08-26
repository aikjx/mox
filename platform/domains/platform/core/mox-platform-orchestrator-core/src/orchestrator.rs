use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

use mox_platform_datastore_core::IamRepository;
use mox_platform_datastore_core::MetaRepository;
use mox_platform_datastore_core::{AuditLogEntry, User};
use mox_platform_datastore_core::{Filter, SortSpec, TxManager, UniversalBizDAO};

use crate::event::EventBus;
use crate::metrics::Metrics;
use crate::module::ModuleRegistry;
use crate::pipeline::{Pipeline, PipelineCtx, PipelineResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BizAction {
    Create,
    Update,
    Delete,
    Get,
    List,
}

impl BizAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            BizAction::Create => "create",
            BizAction::Update => "update",
            BizAction::Delete => "delete",
            BizAction::Get => "get",
            BizAction::List => "list",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessRequest {
    pub tenant_id: String,
    pub user_id: String,
    pub entity_code: String,
    pub action: BizAction,
    pub biz_id: Option<String>,
    pub biz_code: Option<String>,
    pub workflow_instance_id: Option<String>,
    pub data: Option<Map<String, Value>>,
    pub filters: Vec<Filter>,
    pub sort: SortSpec,
    pub page: i64,
    pub page_size: i64,
}

impl Default for BusinessRequest {
    fn default() -> Self {
        Self {
            tenant_id: String::new(),
            user_id: String::new(),
            entity_code: String::new(),
            action: BizAction::Get,
            biz_id: None,
            biz_code: None,
            workflow_instance_id: None,
            data: None,
            filters: vec![],
            sort: SortSpec::default(),
            page: 1,
            page_size: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorResult {
    pub success: bool,
    pub error: Option<String>,
    pub data: Option<Value>,
    pub total: Option<i64>,
    pub biz_id: Option<String>,
    pub version: Option<i64>,
    pub pipeline_stages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    pub biz_id: String,
    pub entity_code: String,
    pub version: i64,
    pub data: Value,
}

struct NoopIam;

impl IamRepository for NoopIam {
    fn check_permission(
        &self,
        _tenant_id: &str,
        _user_id: &str,
        _entity_code: &str,
        _action: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    fn get_user(&self, _user_id: &str) -> anyhow::Result<User> {
        anyhow::bail!("noop iam")
    }
    fn write_audit_log(&self, _entry: AuditLogEntry) -> anyhow::Result<()> {
        Ok(())
    }
}

fn noop_iam() -> NoopIam {
    NoopIam
}

pub struct Orchestrator {
    pub pipeline: Pipeline,
    pub registry: ModuleRegistry,
    pub event_bus: EventBus,
    pub metrics: Metrics,
    pub meta: Option<Arc<dyn MetaRepository>>,
    pub dao: Option<Arc<UniversalBizDAO>>,
    pub pipelines: std::collections::HashMap<String, Pipeline>,
}

impl Orchestrator {
    pub fn enterprise_default() -> Self {
        Self {
            pipeline: Pipeline::enterprise_default(),
            registry: ModuleRegistry::new(),
            event_bus: EventBus::new(),
            metrics: Metrics::new(),
            meta: None,
            dao: None,
            pipelines: std::collections::HashMap::new(),
        }
    }

    pub fn new<M: MetaRepository + 'static>(meta: Arc<M>, dao: Arc<UniversalBizDAO>) -> Self {
        let mut s = Self::enterprise_default();
        s.meta = Some(meta as Arc<dyn MetaRepository>);
        s.dao = Some(dao);
        s
    }

    pub fn register_pipeline(&mut self, name: &str) {
        self.pipelines
            .insert(name.to_string(), Pipeline::enterprise_default());
    }

    fn default_tenant(tenant_id: Option<&String>) -> String {
        tenant_id.cloned().unwrap_or_else(|| "default".to_string())
    }

    pub fn list_pipelines(&self) -> Vec<String> {
        self.pipelines.keys().cloned().collect()
    }

    pub fn create_sync(
        &self,
        entity_code: &str,
        tenant_id: Option<String>,
        data: BTreeMap<String, Value>,
        actor: &str,
    ) -> anyhow::Result<SyncRecord> {
        let dao = self
            .dao
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("DAO not set"))?;
        let meta = self
            .meta
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Meta repo not set"))?;
        let tid = Self::default_tenant(tenant_id.as_ref());
        let map: Map<String, Value> = data.into_iter().collect();
        let iam = noop_iam();
        let (biz_id, _biz_code, version) =
            dao.create(&**meta, &iam, &tid, entity_code, actor, &map, None, None)?;
        let data = dao
            .get(&**meta, &tid, entity_code, &biz_id)?
            .unwrap_or(Value::Null);
        Ok(SyncRecord {
            biz_id,
            entity_code: entity_code.to_string(),
            version,
            data,
        })
    }

    pub fn update_sync(
        &self,
        biz_id: &str,
        patch: BTreeMap<String, Value>,
        actor: &str,
    ) -> anyhow::Result<SyncRecord> {
        let dao = self
            .dao
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("DAO not set"))?;
        let meta = self
            .meta
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Meta repo not set"))?;
        let map: Map<String, Value> = patch.into_iter().collect();
        let (tid, entity_code) = Self::resolve_biz(dao, biz_id)?;
        let version = dao.update(&**meta, &tid, &entity_code, biz_id, actor, &map)?;
        let data = dao
            .get(&**meta, &tid, &entity_code, biz_id)?
            .unwrap_or(Value::Null);
        Ok(SyncRecord {
            biz_id: biz_id.to_string(),
            entity_code,
            version,
            data,
        })
    }

    pub fn delete_sync(&self, biz_id: &str, actor: &str) -> anyhow::Result<()> {
        let dao = self
            .dao
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("DAO not set"))?;
        let (tid, entity_code) = Self::resolve_biz(dao, biz_id)?;
        dao.delete(&tid, &entity_code, biz_id, actor, None)
    }

    pub fn get_sync(&self, biz_id: &str) -> anyhow::Result<Option<SyncRecord>> {
        let dao = self
            .dao
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("DAO not set"))?;
        let meta = self
            .meta
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Meta repo not set"))?;
        let (tid, entity_code, version) = match Self::resolve_biz_ver(dao, biz_id) {
            Ok(x) => x,
            Err(_) => return Ok(None),
        };
        let data = match dao.get(&**meta, &tid, &entity_code, biz_id)? {
            Some(d) => d,
            None => return Ok(None),
        };
        Ok(Some(SyncRecord {
            biz_id: biz_id.to_string(),
            entity_code,
            version,
            data,
        }))
    }

    pub fn list_sync(
        &self,
        entity_code: &str,
        tenant_id: Option<&str>,
    ) -> anyhow::Result<Vec<Value>> {
        let dao = self
            .dao
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("DAO not set"))?;
        let meta = self
            .meta
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Meta repo not set"))?;
        let tid = tenant_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| "default".to_string());
        let res = dao.list(
            &**meta,
            &tid,
            entity_code,
            vec![],
            SortSpec::default(),
            1,
            1000,
        )?;
        Ok(res.items)
    }

    pub fn version_count_sync(&self, biz_id: &str) -> i64 {
        let dao = match self.dao.as_ref() {
            Some(d) => d,
            None => return 0,
        };
        let conn = dao.conn.lock();
        let cnt: Result<i64, _> = conn.query_row(
            "SELECT COUNT(*) FROM biz_data_version WHERE biz_id = ?1",
            params![biz_id],
            |r| r.get(0),
        );
        cnt.unwrap_or(0)
    }

    pub fn audit_chain_sync(&self, biz_id: &str) -> Vec<Value> {
        let dao = match self.dao.as_ref() {
            Some(d) => d,
            None => return vec![],
        };
        let conn = dao.conn.lock();
        let mut stmt = match conn.prepare(
            "SELECT version_id, biz_id, version_num, changed_fields, operation_type, operator_user_id, prev_hash, curr_hash, created_at \
             FROM biz_data_version WHERE biz_id = ?1 ORDER BY version_num ASC",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = stmt.query_map(params![biz_id], |r| {
            Ok(serde_json::json!({
                "version_id": r.get::<_, String>(0).ok(),
                "biz_id": r.get::<_, String>(1).ok(),
                "version_num": r.get::<_, i64>(2).ok(),
                "changed_fields": r.get::<_, String>(3).ok(),
                "operation_type": r.get::<_, String>(4).ok(),
                "operator_user_id": r.get::<_, String>(5).ok(),
                "prev_hash": r.get::<_, String>(6).ok(),
                "curr_hash": r.get::<_, String>(7).ok(),
                "created_at": r.get::<_, String>(8).ok(),
            }))
        });
        match rows {
            Ok(iter) => iter.filter_map(|v| v.ok()).collect(),
            Err(_) => vec![],
        }
    }

    pub fn blocking<F, R>(
        f: F,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<R>> + Send>>
    where
        F: FnOnce() -> anyhow::Result<R> + Send + 'static,
        R: Send + 'static,
    {
        Box::pin(async move {
            tokio::task::spawn_blocking(f)
                .await
                .map_err(|e| anyhow::anyhow!("join error: {}", e))?
        })
    }

    fn resolve_biz(dao: &UniversalBizDAO, biz_id: &str) -> anyhow::Result<(String, String)> {
        let conn = dao.conn.lock();
        let (tid, ec): (String, String) = conn
            .query_row(
                "SELECT tenant_id, biz_type FROM biz_data WHERE biz_id = ?1",
                params![biz_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| anyhow::anyhow!("resolve_biz failed: {}", e))?;
        Ok((tid, ec))
    }

    fn resolve_biz_ver(
        dao: &UniversalBizDAO,
        biz_id: &str,
    ) -> anyhow::Result<(String, String, i64)> {
        let conn = dao.conn.lock();
        let (tid, ec, ver): (String, String, i64) = conn
            .query_row(
                "SELECT tenant_id, biz_type, version FROM biz_data WHERE biz_id = ?1",
                params![biz_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| anyhow::anyhow!("resolve_biz_ver failed: {}", e))?;
        Ok((tid, ec, ver))
    }

    pub fn execute<M: MetaRepository, I: IamRepository>(
        &self,
        req: &BusinessRequest,
        dao: &UniversalBizDAO,
        tx_manager: Option<&TxManager>,
        meta_repo: &M,
        iam_repo: &I,
    ) -> OrchestratorResult {
        let mut ctx = PipelineCtx::new(req);
        let result = self.pipeline.run(
            &mut ctx,
            dao,
            tx_manager,
            &self.registry,
            &self.event_bus,
            &self.metrics,
            meta_repo,
            iam_repo,
            req,
        );
        let PipelineResult {
            success,
            error,
            stages_run,
        } = result;
        let stage_names = stages_run.iter().map(|s| format!("{:?}", s)).collect();
        OrchestratorResult {
            success,
            error,
            data: ctx.response_data.clone(),
            total: ctx.response_list_total,
            biz_id: ctx.biz_id.clone(),
            version: ctx.version,
            pipeline_stages: stage_names,
        }
    }

    pub async fn execute_async<M, I>(
        self: Arc<Self>,
        req: BusinessRequest,
        dao: Arc<UniversalBizDAO>,
        tx_manager: Option<Arc<TxManager>>,
        meta_repo: Arc<M>,
        iam_repo: Arc<I>,
    ) -> OrchestratorResult
    where
        M: MetaRepository + 'static,
        I: IamRepository + 'static,
    {
        let orc = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let tx_ref: Option<&TxManager> = tx_manager.as_ref().map(|t| t.as_ref());
            orc.execute(
                &req,
                dao.as_ref(),
                tx_ref,
                meta_repo.as_ref(),
                iam_repo.as_ref(),
            )
        })
        .await
        .unwrap_or_else(|e| OrchestratorResult {
            success: false,
            error: Some(format!("spawn_blocking panic: {}", e)),
            data: None,
            total: None,
            biz_id: None,
            version: None,
            pipeline_stages: vec![],
        });

        result
    }
}
