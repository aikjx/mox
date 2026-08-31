//! ETL 执行引擎
//!
//! 负责执行数据抽取、转换、加载流程

use async_trait::async_trait;
use crate::error::HubResult;
use crate::pipeline::{PipelineConfig, PipelineStats, PipelineStatus};

/// ETL 执行器接口
#[async_trait]
pub trait EtlExecutor: Send + Sync {
    /// 执行流水线
    async fn execute(&self, config: &PipelineConfig) -> HubResult<PipelineStats>;

    /// 获取流水线状态
    async fn get_status(&self, pipeline_id: &str) -> HubResult<PipelineStatus>;

    /// 取消流水线
    async fn cancel(&self, pipeline_id: &str) -> HubResult<bool>;

    /// 暂停流水线
    async fn pause(&self, pipeline_id: &str) -> HubResult<bool>;

    /// 恢复流水线
    async fn resume(&self, pipeline_id: &str) -> HubResult<bool>;
}

/// ETL 执行器实现（占位）
pub struct DefaultEtlExecutor;

#[async_trait]
impl EtlExecutor for DefaultEtlExecutor {
    async fn execute(&self, _config: &PipelineConfig) -> HubResult<PipelineStats> {
        // TODO: 实现完整的 ETL 执行逻辑
        Ok(PipelineStats::default())
    }

    async fn get_status(&self, _pipeline_id: &str) -> HubResult<PipelineStatus> {
        Ok(PipelineStatus::Idle)
    }

    async fn cancel(&self, _pipeline_id: &str) -> HubResult<bool> {
        Ok(true)
    }

    async fn pause(&self, _pipeline_id: &str) -> HubResult<bool> {
        Ok(true)
    }

    async fn resume(&self, _pipeline_id: &str) -> HubResult<bool> {
        Ok(true)
    }
}
