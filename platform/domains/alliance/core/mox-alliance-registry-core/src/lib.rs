// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家注册中心 · 分级心跳聚合核心（纯计算，零 IO）。
//!
//! 海量编排下（见 `docs/architecture/microservices/07-massive-scale-100k-nodes.md`
//! §五），每节点直连注册中心心跳的流量为 `N/lease`；按 node→rack→cell 分级聚合
//! （10:1:1）后降为 `N/(lease·fan_in·cell_batch)`，即万分之十。本 crate 提供：
//!
//! - [`RackAggregator`]：rack 层聚合器（收集叶级心跳，按批发布 [`GroupDigest`]）
//! - [`CellAggregator`]：cell 层聚合器（合并 rack 摘要，按 tick 发布 [`CellDigest`]）
//! - [`AggregatedRenewal`] / [`RegistryApplier`]：注册中心侧续约载荷与应用语义
//!
//! 时钟一律可注入（对齐 `RegistryStore::reap_expired_at` 约定），便于确定性单测。

pub mod aggregation;

pub use aggregation::{
    AggregatedRenewal, AggregationHealth, CellAggregator, GroupDigest, MemberStatus, NodeBeat,
    RackAggregator, RackConfig, RegistryApplier, Renewal,
};
