// Copyright (c) 2026 璇玑 RelGraph · 架构归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 事件桥接（归一化第①步）
//!
//! 将本域的 [`PlatformEvent`] 桥接到共享事件基础设施 [`mox_event_core`]：
//! - [`mox_event_core::Event`] trait 实现（事件类型映射 + 默认序列化）
//! - 域内总线（同步、无超时/DLQ）与共享总线（异步/并发/超时/DLQ）的接缝
//!
//! 归一化口径（docs/architecture/15-EVENT-TRIGGER-APPROVAL-ARCHITECTURE.md §6）：
//! ① 域事件 impl 共享 Event trait（本文件）→ ② 路由内核替换为共享总线
//! （保留本域 EventBus facade）→ ③ 超时/DLQ 依赖共享层。
//! 第②③步涉及 API 破坏（域内同步 publish 返回 Vec<EventHandleResult>），
//! 需业务域排期后渐进执行；本文件先行落地第①步，纯加法、零破坏。

use crate::event_bus::PlatformEvent;

impl mox_event_core::Event for PlatformEvent {
    /// 事件类型映射：域内枚举 → 共享总线点分字符串
    /// （采用既有 `EventType::name()` 的 snake_case 全名，如 "intent_recognized"）
    fn event_type(&self) -> mox_event_core::EventType {
        mox_event_core::EventType::new(self.event_type.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::EventType as FlowEventType;
    use crate::types::NormalizationSystem;
    use mox_event_core::Event as _;
    use serde_json::json;

    #[test]
    fn test_platform_event_bridges_to_shared_event_type() {
        let evt = PlatformEvent::new(
            FlowEventType::IntentRecognized,
            NormalizationSystem::AiAssistant,
            "tenant-1",
            json!({"intent": "ask_question"}),
        );
        // trait 方法（共享总线类型）
        let shared_type = mox_event_core::Event::event_type(&evt);
        assert_eq!(shared_type.as_str(), "intent_recognized");

        // 默认序列化路径可用（to_payload 默认实现走 serde_json）
        let payload = mox_event_core::Event::to_payload(&evt).unwrap();
        assert!(!payload.is_empty());
    }

    #[test]
    fn test_process_completed_maps_to_dot_name() {
        let evt = PlatformEvent::new(
            FlowEventType::ProcessCompleted,
            NormalizationSystem::ProcessAlgo,
            "tenant-1",
            json!({"process_id": "p1"}),
        );
        let shared_type = mox_event_core::Event::event_type(&evt);
        assert_eq!(shared_type.as_str(), "process_completed");
    }
}
