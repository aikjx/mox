//! MOX 平台业务编排核心
//! 10 阶段 Pipeline 业务编排 + BizModule 行业包 + 事件总线 + 指标

pub mod event;
pub mod metrics;
pub mod module;
pub mod orchestrator;
pub mod pipeline;

pub use crate::event::{BusinessEvent, EventBus};
pub use crate::metrics::Metrics;
pub use crate::module::{
    BizModule, CommonModule, EducationModule, FinanceModule, GovernmentModule, ManufacturingModule,
    MedicalModule, ModuleRegistry, RetailModule,
};
pub use crate::orchestrator::{BizAction, BusinessRequest, Orchestrator, OrchestratorResult};
pub use crate::pipeline::{Pipeline, PipelineCtx, PipelineResult, Stage, StageId, StepResult};
