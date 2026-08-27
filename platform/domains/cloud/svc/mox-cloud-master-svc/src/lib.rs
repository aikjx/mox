// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Cloud Drive L4 Master (All self-implemented, no external storage system)
//!
//! Mox Cloud Drive Master — 控制面 (Control Plane)
//! 负责: volume 注册/心跳、卷分配、副本 quorum、快照管理、集群状态
//!
//! 完全自研实现。

pub mod error;
pub mod master_server;
pub mod snapshot;
pub mod volume_allocator;
pub mod volume_replica;

pub use error::{MasterError, MasterResult};
pub use master_server::{
    MasterConfig, MasterServer, Metrics, VolumeId, VolumeLoadReport, VolumeStatus,
    VolumeStatusState,
};
pub use snapshot::{SnapshotId, SnapshotManager, SnapshotMeta};
pub use volume_allocator::{VolumeAllocation, VolumeAllocator, VolumeInfo};
pub use volume_replica::{ReplicaHealth, ReplicaInfo, ReplicaSet, ReplicaSetManager};
