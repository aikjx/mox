// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BusinessEvent {
    Created {
        tenant_id: String,
        entity_code: String,
        biz_id: String,
        fields: Map<String, Value>,
    },
    Updated {
        tenant_id: String,
        entity_code: String,
        biz_id: String,
        fields: Map<String, Value>,
    },
    Deleted {
        tenant_id: String,
        entity_code: String,
        biz_id: String,
    },
    StatusChanged {
        tenant_id: String,
        entity_code: String,
        biz_id: String,
        old_status: String,
        new_status: String,
    },
    WorkflowApproved {
        tenant_id: String,
        entity_code: String,
        biz_id: String,
        workflow_instance_id: String,
    },
}

pub struct EventBus {
    pub queue: Mutex<Vec<BusinessEvent>>,
    pub listeners: DashMap<String, Vec<Box<dyn Fn(&BusinessEvent) + Send + Sync>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            listeners: DashMap::new(),
        }
    }

    pub fn publish(&self, event: BusinessEvent) {
        let kind = match &event {
            BusinessEvent::Created { .. } => "created",
            BusinessEvent::Updated { .. } => "updated",
            BusinessEvent::Deleted { .. } => "deleted",
            BusinessEvent::StatusChanged { .. } => "status_changed",
            BusinessEvent::WorkflowApproved { .. } => "workflow_approved",
        };
        if let Some(list) = self.listeners.get("*") {
            for l in list.value() {
                l(&event);
            }
        }
        if let Some(list) = self.listeners.get(kind) {
            for l in list.value() {
                l(&event);
            }
        }
        self.queue.lock().unwrap().push(event);
    }

    pub fn queue_len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn on<F>(&self, kind: &str, f: F)
    where
        F: Fn(&BusinessEvent) + Send + Sync + 'static,
    {
        self.listeners
            .entry(kind.to_string())
            .or_default()
            .push(Box::new(f));
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
