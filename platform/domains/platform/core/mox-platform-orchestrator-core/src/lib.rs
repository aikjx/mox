//! MOX 平台业务编排核心
//! 10 阶段 Pipeline 业务编排 + BizModule 行业包 + 事件总线 + 指标

pub mod pipeline;
pub mod module;
pub mod event;
pub mod metrics;
pub mod orchestrator;

pub use crate::orchestrator::{Orchestrator, BizAction, BusinessRequest, OrchestratorResult};
pub use crate::pipeline::{Pipeline, PipelineCtx, PipelineResult, StageId, StepResult, Stage};
pub use crate::module::{BizModule, ModuleRegistry, CommonModule, FinanceModule, MedicalModule, ManufacturingModule, GovernmentModule, EducationModule, RetailModule};
pub use crate::event::{EventBus, BusinessEvent};
pub use crate::metrics::Metrics;
