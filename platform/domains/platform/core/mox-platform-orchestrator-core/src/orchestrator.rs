use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use mox_platform_datastore_core::{UniversalBizDAO, TxManager, Filter, SortSpec};
use mox_platform_datastore_core::MetaRepository;
use mox_platform_datastore_core::IamRepository;

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

pub struct Orchestrator {
    pub pipeline: Pipeline,
    pub registry: ModuleRegistry,
    pub event_bus: EventBus,
    pub metrics: Metrics,
}

impl Orchestrator {
    pub fn enterprise_default() -> Self {
        Self {
            pipeline: Pipeline::enterprise_default(),
            registry: ModuleRegistry::new(),
            event_bus: EventBus::new(),
            metrics: Metrics::new(),
        }
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
        let PipelineResult { success, error, stages_run } = result;
        let stage_names = stages_run
            .iter()
            .map(|s| format!("{:?}", s))
            .collect();
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

    pub async fn execute_with_tokio<M, I>(
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
            orc.execute(&req, dao.as_ref(), tx_ref, meta_repo.as_ref(), iam_repo.as_ref())
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

        // Event 阶段非阻塞事件队列已在同步 pipeline 中写入
        // 此处可进一步异步推送通知，省略实现

        result
    }
}
