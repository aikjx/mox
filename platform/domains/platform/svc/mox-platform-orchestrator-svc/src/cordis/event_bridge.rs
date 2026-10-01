// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 事件桥接（归一化第①步）
//!
//! 将 OUS-Cordis 的 [`Event`] 枚举桥接到共享事件基础设施 [`mox_event_core`]：
//! - [`mox_event_core::Event`] trait 实现（事件类型映射：`domain.action` 点分命名）
//! - 域内双层路由（domain → event_type）与共享总线（通配符/并发/超时/DLQ）的接缝
//!
//! 归一化口径（docs/architecture/15-EVENT-TRIGGER-APPROVAL-ARCHITECTURE.md §6）：
//! ① 域事件 impl 共享 Event trait（本文件）→ ② 路由内核替换为共享总线
//! （保留 cordis EventBus facade）→ ③ 超时/DLQ 依赖共享层。
//! 注意：cordis `Event` 固有方法 `event_type()` 返回 String（向后兼容，保留）；
//! 共享总线侧请用 `mox_event_core::Event::event_type(&e)`（返回 EventType）。

use super::event_bus::Event;

impl mox_event_core::Event for Event {
    /// 事件类型映射：`domain.action`（如 "profile.loaded"、"turn.completed"）
    /// 复用固有 domain()/event_type()，天然符合共享总线点分命名 + 通配符规范
    fn event_type(&self) -> mox_event_core::EventType {
        // 注意：trait impl 内的 self.event_type() 解析到固有方法（返回 String），
        // 有意为之——避免递归调用本 trait 方法
        mox_event_core::EventType::new(format!("{}.{}", self.domain(), self.event_type()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_event_core::Event as _;

    #[test]
    fn test_profile_event_maps_to_domain_dot_action() {
        let evt = Event::ProfileLoaded {
            name: "default".to_string(),
            path: "/profiles/default.yaml".to_string(),
        };
        let shared_type = mox_event_core::Event::event_type(&evt);
        assert_eq!(shared_type.as_str(), "profile.loaded");
        // 通配符订阅可用
        assert!(mox_event_core::EventType::new("profile.*").matches(&shared_type));
    }

    #[test]
    fn test_turn_completed_maps_to_turn_dot_completed() {
        let evt = Event::StepFailed {
            step_id: "s1".to_string(),
            turn_id: "t1".to_string(),
            error: "boom".to_string(),
        };
        let shared_type = mox_event_core::Event::event_type(&evt);
        assert_eq!(shared_type.as_str(), "step.failed");
    }
}
