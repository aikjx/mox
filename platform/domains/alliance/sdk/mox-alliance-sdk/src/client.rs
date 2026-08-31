// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 联盟客户端
//!
//! Phase 1：SDK 骨架，具体 HTTP 客户端实现后续补充。

use mox_alliance_api::dto::*;
use mox_alliance_common_proto::AllianceResult;
use uuid::Uuid;

/// 联盟客户端
pub struct AllianceClient {
    base_url: String,
}

impl AllianceClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    /// 创建任务
    pub async fn create_task(&self, _request: CreateTaskRequest) -> AllianceResult<CreateTaskResponse> {
        // Phase 1: stub
        Err(mox_alliance_common_proto::AllianceError::internal(
            "SDK not fully implemented in Phase 1",
        ))
    }

    /// 获取任务详情
    pub async fn get_task(&self, _task_id: Uuid) -> AllianceResult<TaskDetailResponse> {
        Err(mox_alliance_common_proto::AllianceError::internal(
            "SDK not fully implemented in Phase 1",
        ))
    }

    /// 执行任务操作
    pub async fn task_action(
        &self,
        _task_id: Uuid,
        _action: TaskActionRequest,
    ) -> AllianceResult<SuccessResponse> {
        Err(mox_alliance_common_proto::AllianceError::internal(
            "SDK not fully implemented in Phase 1",
        ))
    }
}
