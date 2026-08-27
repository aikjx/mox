// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! `PTEnvelope` 归一化跨层消息（PT-Primi §4 接口契约）
//!
//! 相邻层（L1-L7）之间仅通过标准消息体通信，六维绑定 ID 随消息全程透传，
//! 满足 A5 可追溯。这是把分散模块「一体化」为闭环的通信骨架。

use crate::unified::{Layer, PrimitiveCoords};
use serde::{Deserialize, Serialize};

/// 归一化跨层信封：承载一次跨层调用的全部可溯源上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTEnvelope {
    /// 全局追踪 id（贯穿 L1→L7）
    pub trace_id: String,
    /// 来源层 / 目标层
    pub from_layer: Layer,
    pub to_layer: Layer,
    /// 随信携带的原语坐标（κ,τ,C,Q）
    pub primitive: PrimitiveCoords,
    /// 业务载荷（各层自定义 JSON）
    pub payload: serde_json::Value,
    /// 六维绑定 ID 链（REQ-/FUN-/BIZ-/ALG-/TSK-/COD-），全程透传
    pub bind_ids: Vec<String>,
    /// 消息签名（完整性校验）
    pub signature: String,
}

impl PTEnvelope {
    pub fn new(
        trace_id: impl Into<String>,
        from_layer: Layer,
        to_layer: Layer,
        primitive: PrimitiveCoords,
        payload: serde_json::Value,
        bind_ids: Vec<String>,
    ) -> Self {
        let tid: String = trace_id.into();
        let signature = Self::sign(&tid, &primitive, &bind_ids);
        Self {
            trace_id: tid,
            from_layer,
            to_layer,
            primitive,
            payload,
            bind_ids,
            signature,
        }
    }

    /// 跨层转发：保留 trace_id / primitive / bind_ids，仅切换层并更新载荷
    pub fn forward(&self, to_layer: Layer, payload: serde_json::Value) -> Self {
        Self::new(
            self.trace_id.clone(),
            self.to_layer,
            to_layer,
            self.primitive,
            payload,
            self.bind_ids.clone(),
        )
    }

    /// 追加六维绑定 ID（溯源链生长）
    pub fn with_bind(mut self, id: impl Into<String>) -> Self {
        self.bind_ids.push(id.into());
        self.signature = Self::sign(&self.trace_id, &self.primitive, &self.bind_ids);
        self
    }

    /// 守恒残差随信封透传校验（PT-Primi §3.1 A3）
    pub fn is_conserved(&self, eps: f64) -> bool {
        self.primitive.is_conserved(eps)
    }

    fn sign(trace_id: &str, p: &PrimitiveCoords, binds: &[String]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        trace_id.hash(&mut h);
        (p.kappa as i64).hash(&mut h);
        (p.tau as i64).hash(&mut h);
        (p.c as i64).hash(&mut h);
        binds.hash(&mut h);
        format!("{:016x}", h.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_forwards_and_keeps_trace() {
        let e = PTEnvelope::new(
            "tr-1",
            Layer::RequirementSemantic,
            Layer::PrimitiveMapping,
            PrimitiveCoords::from_kt(3.0, 4.0),
            serde_json::json!({"q": "需求文本"}),
            vec!["REQ-1".into()],
        );
        let e2 = e.forward(Layer::TopologyEmergence, serde_json::json!({"topo": "..."}));
        assert_eq!(e2.trace_id, "tr-1");
        assert_eq!(e2.from_layer, Layer::PrimitiveMapping);
        assert_eq!(e2.to_layer, Layer::TopologyEmergence);
        assert_eq!(e2.bind_ids, vec!["REQ-1"]);
        assert!(e2.is_conserved(1e-3));
    }

    #[test]
    fn envelope_bind_grows_chain() {
        let e = PTEnvelope::new(
            "tr-2",
            Layer::RequirementSemantic,
            Layer::PrimitiveMapping,
            PrimitiveCoords::zero(),
            serde_json::Value::Null,
            vec![],
        )
        .with_bind("REQ-9")
        .with_bind("FUN-9");
        assert_eq!(e.bind_ids, vec!["REQ-9", "FUN-9"]);
    }
}
