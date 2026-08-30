// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Master 节点 Raft 共识模块
//!
//! 基于自研简化版 Raft 协议实现 Master 高可用（HA）。
//! 支持三种角色：Leader / Follower / Standby（Learner）
//!
//! 元数据 Raft 日志类型：
//! - 卷分配（VolumeAllocation）
//! - 心跳上报（Heartbeat）
//! - 副本迁移（ReplicaMigration）
//! - 配置变更（ConfigChange）
//!
//! 核心特性：
//! - Leader 选举与任期管理
//! - 日志复制与提交
//! - 快照生成与恢复
//! - 领导者租约（Leader Lease）优化读路径

use crate::error::{MasterError, MasterResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Master 节点角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftRole {
    /// 领导者：处理所有写请求和读请求
    Leader,
    /// 跟随者：复制日志，参与选举投票
    Follower,
    /// 备用/学习者：只复制日志，不参与投票，不参选
    Standby,
}

impl std::fmt::Display for RaftRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RaftRole::Leader => write!(f, "Leader"),
            RaftRole::Follower => write!(f, "Follower"),
            RaftRole::Standby => write!(f, "Standby"),
        }
    }
}

/// Raft 日志条目类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftLogType {
    /// 卷分配日志：分配新的卷及副本位置
    VolumeAllocation,
    /// 心跳日志：Volume 节点心跳上报
    Heartbeat,
    /// 副本迁移日志：副本从一个节点迁移到另一个节点
    ReplicaMigration,
    /// 配置变更日志：集群节点增减
    ConfigChange,
    /// 快照标记日志：用于日志压缩边界
    SnapshotMarker,
    /// 空日志：新 Leader 上任后提交的第一条日志
    NoOp,
}

/// Raft 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftLogEntry {
    /// 日志索引（单调递增）
    pub index: u64,
    /// 任期号
    pub term: u64,
    /// 日志类型
    pub log_type: RaftLogType,
    /// 日志数据（序列化后的业务数据）
    pub data: Vec<u8>,
    /// 创建时间戳（ms）
    pub created_at_ms: u64,
}

/// 卷分配日志数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeAllocationLog {
    pub volume_id: String,
    pub size: u64,
    pub replica_count: u8,
    pub replica_ids: Vec<String>,
    pub replica_addresses: Vec<String>,
}

/// 心跳日志数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatLog {
    pub volume_id: String,
    pub used_bytes: u64,
    pub is_healthy: bool,
    pub cpu_pct: u8,
}

/// 副本迁移日志数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaMigrationLog {
    pub set_id: String,
    pub from_volume: String,
    pub to_volume: String,
    pub to_address: String,
}

/// 配置变更日志数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeLog {
    pub node_id: String,
    pub node_addr: String,
    pub change_type: ConfigChangeType,
}

/// 配置变更类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigChangeType {
    AddNode,
    RemoveNode,
    PromoteToVoter,
    DemoteToLearner,
}

/// Raft 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftNodeInfo {
    pub node_id: String,
    pub addr: String,
    pub is_voter: bool,
}

/// Raft 快照元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftSnapshotMeta {
    /// 快照包含的最后日志索引
    pub last_index: u64,
    /// 快照包含的最后日志任期
    pub last_term: u64,
    /// 快照创建时间
    pub created_at_ms: u64,
    /// 快照大小（字节）
    pub size_bytes: u64,
}

/// Raft 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// 本节点 ID
    pub node_id: String,
    /// 本节点地址
    pub listen_addr: String,
    /// 选举超时时间（ms）
    pub election_timeout_ms: u64,
    /// 心跳间隔（ms）
    pub heartbeat_interval_ms: u64,
    /// 日志最大条目数（超过后触发快照）
    pub max_log_entries: u64,
    /// 领导者租约时长（ms）
    pub leader_lease_ms: u64,
}

impl Default for RaftConfig {
    fn default() -> Self {
        RaftConfig {
            node_id: String::new(),
            listen_addr: "127.0.0.1:9333".to_string(),
            election_timeout_ms: 3000,
            heartbeat_interval_ms: 500,
            max_log_entries: 10000,
            leader_lease_ms: 1500,
        }
    }
}

/// 投票请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteRequest {
    pub term: u64,
    pub candidate_id: String,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

/// 投票响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    pub term: u64,
    pub vote_granted: bool,
}

/// 追加日志请求（也用作心跳）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    pub term: u64,
    pub leader_id: String,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<RaftLogEntry>,
    pub leader_commit: u64,
}

/// 追加日志响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    pub term: u64,
    pub success: bool,
    pub match_index: u64,
    pub conflict_index: Option<u64>,
}

/// 安装快照请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotRequest {
    pub term: u64,
    pub leader_id: String,
    pub last_included_index: u64,
    pub last_included_term: u64,
    pub offset: u64,
    pub data: Vec<u8>,
    pub done: bool,
}

/// 安装快照响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotResponse {
    pub term: u64,
    pub success: bool,
}

/// 集群成员状态（Leader 追踪每个 Follower 的复制进度）
#[derive(Debug, Clone)]
struct PeerReplicationState {
    /// 下一个要发送的日志索引
    next_index: u64,
    /// 已匹配的最高日志索引
    match_index: u64,
    /// 最近一次成功追加的时间戳
    last_append_time_ms: u64,
    /// 是否正在进行快照传输（预留字段，用于未来快照同步优化）
    #[allow(dead_code)]
    snapshot_in_progress: bool,
}

/// Raft 共识状态机
///
/// 参考 async-raft 的设计理念，但用更轻量的同步实现，
/// 便于嵌入 MasterServer 中，无外部依赖。
pub struct RaftMaster {
    /// Raft 配置
    config: RaftConfig,
    /// 当前角色
    role: parking_lot::RwLock<RaftRole>,
    /// 当前任期
    current_term: parking_lot::Mutex<u64>,
    /// 本任期内投票给的候选人
    voted_for: parking_lot::Mutex<Option<String>>,
    /// 日志条目（索引从 1 开始，index 0 为虚拟条目）
    log: parking_lot::Mutex<VecDeque<RaftLogEntry>>,
    /// 日志的基础偏移（快照压缩后，log[0] 对应的实际索引）
    log_base_index: parking_lot::Mutex<u64>,
    /// 已提交的最高日志索引
    commit_index: parking_lot::Mutex<u64>,
    /// 已应用到状态机的最高日志索引
    last_applied: parking_lot::Mutex<u64>,
    /// 已知的 Leader ID
    leader_id: parking_lot::RwLock<Option<String>>,
    /// 集群节点列表
    nodes: parking_lot::RwLock<Vec<RaftNodeInfo>>,
    /// 每个节点的复制进度（仅 Leader 使用）
    peer_state: parking_lot::Mutex<HashMap<String, PeerReplicationState>>,
    /// 上次收到 Leader 心跳的时间（用于选举超时判断）
    last_heartbeat_time_ms: parking_lot::Mutex<u64>,
    /// 领导者租约到期时间（Leader 端）
    leader_lease_expiry_ms: parking_lot::Mutex<u64>,
    /// 快照元数据
    snapshot_meta: parking_lot::Mutex<Option<RaftSnapshotMeta>>,
    /// 统计指标
    metrics: Arc<RaftMetrics>,
}

/// Raft 统计指标
#[derive(Debug, Default)]
pub struct RaftMetrics {
    /// 选举次数
    pub elections_total: parking_lot::Mutex<u64>,
    /// 成为 Leader 的次数
    pub leader_elections_won: parking_lot::Mutex<u64>,
    /// 已提交的日志总数
    pub logs_committed: parking_lot::Mutex<u64>,
    /// 已应用的日志总数
    pub logs_applied: parking_lot::Mutex<u64>,
    /// 快照生成次数
    pub snapshots_created: parking_lot::Mutex<u64>,
    /// 心跳发送次数
    pub heartbeats_sent: parking_lot::Mutex<u64>,
    /// 心跳接收次数
    pub heartbeats_received: parking_lot::Mutex<u64>,
}

impl RaftMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert(
            "raft_elections_total".into(),
            *self.elections_total.lock(),
        );
        m.insert(
            "raft_leader_elections_won".into(),
            *self.leader_elections_won.lock(),
        );
        m.insert(
            "raft_logs_committed".into(),
            *self.logs_committed.lock(),
        );
        m.insert(
            "raft_logs_applied".into(),
            *self.logs_applied.lock(),
        );
        m.insert(
            "raft_snapshots_created".into(),
            *self.snapshots_created.lock(),
        );
        m.insert(
            "raft_heartbeats_sent".into(),
            *self.heartbeats_sent.lock(),
        );
        m.insert(
            "raft_heartbeats_received".into(),
            *self.heartbeats_received.lock(),
        );
        m
    }
}

impl RaftMaster {
    /// 创建新的 Raft Master 节点（初始为 Follower 角色）
    pub fn new(config: RaftConfig) -> Self {
        let mut log = VecDeque::new();
        // index 0 为虚拟哨兵条目，方便 prev_log_index=0 的边界处理
        log.push_back(RaftLogEntry {
            index: 0,
            term: 0,
            log_type: RaftLogType::NoOp,
            data: Vec::new(),
            created_at_ms: now_ms(),
        });

        let peer_state = HashMap::new();
        // 初始化时还不知道集群其他节点，peer_state 留空

        Self {
            config,
            role: parking_lot::RwLock::new(RaftRole::Follower),
            current_term: parking_lot::Mutex::new(0),
            voted_for: parking_lot::Mutex::new(None),
            log: parking_lot::Mutex::new(log),
            log_base_index: parking_lot::Mutex::new(0),
            commit_index: parking_lot::Mutex::new(0),
            last_applied: parking_lot::Mutex::new(0),
            leader_id: parking_lot::RwLock::new(None),
            nodes: parking_lot::RwLock::new(Vec::new()),
            peer_state: parking_lot::Mutex::new(peer_state),
            last_heartbeat_time_ms: parking_lot::Mutex::new(now_ms()),
            leader_lease_expiry_ms: parking_lot::Mutex::new(0),
            snapshot_meta: parking_lot::Mutex::new(None),
            metrics: Arc::new(RaftMetrics::new()),
        }
    }

    /// 获取当前角色
    pub fn role(&self) -> RaftRole {
        *self.role.read()
    }

    /// 获取当前任期
    pub fn current_term(&self) -> u64 {
        *self.current_term.lock()
    }

    /// 获取当前 Leader ID（可能为 None）
    pub fn leader_id(&self) -> Option<String> {
        self.leader_id.read().clone()
    }

    /// 检查当前节点是否为 Leader 且租约有效（可安全处理读请求）
    pub fn is_leader_leased(&self) -> bool {
        if *self.role.read() != RaftRole::Leader {
            return false;
        }
        let expiry = *self.leader_lease_expiry_ms.lock();
        now_ms() < expiry
    }

    /// 获取指标快照
    pub fn metrics(&self) -> Arc<RaftMetrics> {
        self.metrics.clone()
    }

    // -----------------------------------------------------------------------
    // 集群管理
    // -----------------------------------------------------------------------

    /// 添加节点到集群配置
    pub fn add_node(&self, node: RaftNodeInfo) {
        let mut nodes = self.nodes.write();
        if !nodes.iter().any(|n| n.node_id == node.node_id) {
            nodes.push(node.clone());
        }
        drop(nodes);

        // 如果是 Leader，初始化该节点的复制状态
        if *self.role.read() == RaftRole::Leader {
            let last_idx = self.last_log_index();
            let mut ps = self.peer_state.lock();
            ps.entry(node.node_id.clone()).or_insert(PeerReplicationState {
                next_index: last_idx + 1,
                match_index: 0,
                last_append_time_ms: now_ms(),
                snapshot_in_progress: false,
            });
        }
    }

    /// 从集群移除节点
    pub fn remove_node(&self, node_id: &str) {
        let mut nodes = self.nodes.write();
        nodes.retain(|n| n.node_id != node_id);
        drop(nodes);

        let mut ps = self.peer_state.lock();
        ps.remove(node_id);
    }

    /// 获取集群节点列表
    pub fn list_nodes(&self) -> Vec<RaftNodeInfo> {
        self.nodes.read().clone()
    }

    /// 获取投票节点数
    pub fn voter_count(&self) -> usize {
        self.nodes.read().iter().filter(|n| n.is_voter).count()
    }

    // -----------------------------------------------------------------------
    // 日志操作
    // -----------------------------------------------------------------------

    /// 获取最后一条日志的索引
    pub fn last_log_index(&self) -> u64 {
        let log = self.log.lock();
        let base = *self.log_base_index.lock();
        log.back().map(|e| e.index).unwrap_or(base)
    }

    /// 获取最后一条日志的任期
    pub fn last_log_term(&self) -> u64 {
        let log = self.log.lock();
        log.back().map(|e| e.term).unwrap_or(0)
    }

    /// 获取指定索引处的日志条目
    pub fn get_log_entry(&self, index: u64) -> Option<RaftLogEntry> {
        let log = self.log.lock();
        let base = *self.log_base_index.lock();
        if index < base {
            return None;
        }
        let offset = (index - base) as usize;
        log.get(offset).cloned()
    }

    /// 追加日志条目（仅 Leader 调用）
    pub fn append_log(&self, log_type: RaftLogType, data: Vec<u8>) -> MasterResult<u64> {
        if *self.role.read() != RaftRole::Leader {
            return Err(MasterError::Internal(
                "not leader, cannot append log".to_string(),
            ));
        }
        let term = *self.current_term.lock();
        let mut log = self.log.lock();
        let base = *self.log_base_index.lock();
        let next_index = log.back().map(|e| e.index + 1).unwrap_or(base + 1);
        let entry = RaftLogEntry {
            index: next_index,
            term,
            log_type,
            data,
            created_at_ms: now_ms(),
        };
        log.push_back(entry);
        Ok(next_index)
    }

    // -----------------------------------------------------------------------
    // 选举相关
    // -----------------------------------------------------------------------

    /// 检查是否选举超时（Follower/Candidate 调用）
    pub fn is_election_timeout(&self) -> bool {
        let role = *self.role.read();
        if role == RaftRole::Leader {
            return false;
        }
        let last_hb = *self.last_heartbeat_time_ms.lock();
        now_ms().saturating_sub(last_hb) > self.config.election_timeout_ms
    }

    /// 发起选举（转换为 Candidate 并请求投票）
    /// 返回本节点是否获得了自己的投票（总是 true）以及当前任期
    pub fn start_election(&self) -> (u64, RequestVoteRequest) {
        // 增加任期
        let mut term = self.current_term.lock();
        *term += 1;
        let new_term = *term;
        drop(term);

        // 转换为 Candidate，投给自己
        *self.role.write() = RaftRole::Follower; // 先标记，等下外部根据结果决定
        *self.voted_for.lock() = Some(self.config.node_id.clone());

        *self.metrics.elections_total.lock() += 1;

        let req = RequestVoteRequest {
            term: new_term,
            candidate_id: self.config.node_id.clone(),
            last_log_index: self.last_log_index(),
            last_log_term: self.last_log_term(),
        };

        (new_term, req)
    }

    /// 处理投票请求
    pub fn handle_request_vote(&self, req: RequestVoteRequest) -> RequestVoteResponse {
        let mut current_term = self.current_term.lock();

        // 如果请求任期小于当前任期，拒绝
        if req.term < *current_term {
            return RequestVoteResponse {
                term: *current_term,
                vote_granted: false,
            };
        }

        // 如果请求任期大于当前任期，更新任期并重置投票
        if req.term > *current_term {
            *current_term = req.term;
            *self.voted_for.lock() = None;
            // 转为 Follower
            *self.role.write() = RaftRole::Follower;
            *self.leader_id.write() = None;
        }

        let voted_for = self.voted_for.lock();
        let can_vote = voted_for.is_none() || voted_for.as_deref() == Some(&req.candidate_id);
        drop(voted_for);

        // 检查候选人日志是否至少和自己一样新
        let log_ok = self.is_candidate_log_up_to_date(req.last_log_index, req.last_log_term);

        if can_vote && log_ok {
            *self.voted_for.lock() = Some(req.candidate_id.clone());
            // 重置选举计时器
            *self.last_heartbeat_time_ms.lock() = now_ms();
            RequestVoteResponse {
                term: req.term,
                vote_granted: true,
            }
        } else {
            RequestVoteResponse {
                term: req.term,
                vote_granted: false,
            }
        }
    }

    /// 判断候选人的日志是否至少和自己的一样新
    fn is_candidate_log_up_to_date(&self, cand_last_index: u64, cand_last_term: u64) -> bool {
        let my_last_term = self.last_log_term();
        let my_last_index = self.last_log_index();

        // 首先比较最后一条日志的任期
        if cand_last_term != my_last_term {
            return cand_last_term > my_last_term;
        }
        // 任期相同则比较索引
        cand_last_index >= my_last_index
    }

    /// 收集选举结果，判断是否当选
    /// votes_granted 为已获得的赞成票数（包括自己）
    pub fn check_election_won(&self, votes_granted: usize) -> bool {
        let voter_count = self.voter_count().max(1);
        let majority = voter_count / 2 + 1;
        votes_granted >= majority
    }

    /// 当选为 Leader 后的初始化
    pub fn become_leader(&self) {
        *self.role.write() = RaftRole::Leader;
        *self.leader_id.write() = Some(self.config.node_id.clone());
        *self.leader_lease_expiry_ms.lock() = now_ms() + self.config.leader_lease_ms;

        *self.metrics.leader_elections_won.lock() += 1;

        // 初始化每个节点的复制进度
        let last_idx = self.last_log_index();
        let nodes = self.nodes.read().clone();
        let mut ps = self.peer_state.lock();
        for node in &nodes {
            if node.node_id != self.config.node_id {
                ps.insert(node.node_id.clone(), PeerReplicationState {
                    next_index: last_idx + 1,
                    match_index: 0,
                    last_append_time_ms: now_ms(),
                    snapshot_in_progress: false,
                });
            }
        }
        drop(ps);

        // 提交一条空日志，确保能提交之前任期的日志
        let _ = self.append_log(RaftLogType::NoOp, Vec::new());
    }

    /// 下台为 Follower（发现更高任期的 Leader 时）
    pub fn step_down(&self, new_term: u64, leader_id: String) {
        let mut term = self.current_term.lock();
        if new_term > *term {
            *term = new_term;
        }
        drop(term);
        *self.role.write() = RaftRole::Follower;
        *self.leader_id.write() = Some(leader_id);
        *self.last_heartbeat_time_ms.lock() = now_ms();
        *self.metrics.heartbeats_received.lock() += 1;
    }

    // -----------------------------------------------------------------------
    // 日志复制
    // -----------------------------------------------------------------------

    /// 生成给指定节点的 AppendEntries 请求
    pub fn build_append_entries(&self, peer_id: &str) -> Option<AppendEntriesRequest> {
        if *self.role.read() != RaftRole::Leader {
            return None;
        }

        let ps = self.peer_state.lock();
        let peer = ps.get(peer_id)?;
        let next_idx = peer.next_index;
        drop(ps);

        let log = self.log.lock();
        let base = *self.log_base_index.lock();
        let _last_idx = log.back().map(|e| e.index).unwrap_or(base);

        // 如果 next_idx 小于 log_base_index，说明需要发送快照
        if next_idx <= base {
            return None; // 调用方应使用 InstallSnapshot
        }

        // 计算 prev_log_index 和 prev_log_term
        let prev_log_index = next_idx - 1;
        let prev_offset = prev_log_index.saturating_sub(base) as usize;
        let prev_log_term = log
            .get(prev_offset)
            .map(|e| e.term)
            .unwrap_or(0);

        // 收集要发送的日志条目（最多一次发 100 条）
        let start_offset = next_idx.saturating_sub(base) as usize;
        let mut entries = Vec::new();
        for i in start_offset..log.len() {
            if entries.len() >= 100 {
                break;
            }
            entries.push(log[i].clone());
        }

        let commit_index = *self.commit_index.lock();

        Some(AppendEntriesRequest {
            term: *self.current_term.lock(),
            leader_id: self.config.node_id.clone(),
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit: commit_index,
        })
    }

    /// 处理 AppendEntries 请求（Follower 调用）
    pub fn handle_append_entries(&self, req: AppendEntriesRequest) -> AppendEntriesResponse {
        let mut current_term = self.current_term.lock();

        // 任期检查
        if req.term < *current_term {
            return AppendEntriesResponse {
                term: *current_term,
                success: false,
                match_index: 0,
                conflict_index: None,
            };
        }

        // 更新任期和 Leader 信息
        if req.term > *current_term {
            *current_term = req.term;
            *self.voted_for.lock() = None;
        }
        *self.role.write() = RaftRole::Follower;
        *self.leader_id.write() = Some(req.leader_id.clone());
        *self.last_heartbeat_time_ms.lock() = now_ms();
        *self.metrics.heartbeats_received.lock() += 1;

        drop(current_term);

        // 一致性检查：prev_log_index 处的日志任期必须匹配
        let mut log = self.log.lock();
        let base = *self.log_base_index.lock();
        let last_idx = log.back().map(|e| e.index).unwrap_or(base);

        // 如果 prev_log_index 超出范围
        if req.prev_log_index > last_idx {
            return AppendEntriesResponse {
                term: req.term,
                success: false,
                match_index: last_idx,
                conflict_index: Some(last_idx + 1),
            };
        }

        // 如果 prev_log_index 在快照之前（我们没有这些日志了）
        if req.prev_log_index < base {
            // 如果请求中包含有效日志，我们需要从快照后开始匹配
            if !req.entries.is_empty() {
                let first_entry_idx = req.entries[0].index;
                if first_entry_idx <= base {
                    // 这些日志已经在快照里了，跳过前面的
                    let entries_after_base: Vec<RaftLogEntry> = req
                        .entries
                        .iter()
                        .filter(|e| e.index > base)
                        .cloned()
                        .collect();
                    if entries_after_base.is_empty() {
                        // 所有条目都在快照中，直接成功
                        let match_idx = req.entries.last().map(|e| e.index).unwrap_or(base);
                        self.update_commit_index(req.leader_commit);
                        return AppendEntriesResponse {
                            term: req.term,
                            success: true,
                            match_index: match_idx,
                            conflict_index: None,
                        };
                    }
                }
            }
            return AppendEntriesResponse {
                term: req.term,
                success: false,
                match_index: base,
                conflict_index: Some(base + 1),
            };
        }

        // 检查 prev_log_index 处的任期是否匹配
        let prev_offset = (req.prev_log_index - base) as usize;
        if log[prev_offset].term != req.prev_log_term {
            // 找到冲突：回退到该任期的第一条日志
            let conflict_term = log[prev_offset].term;
            let mut conflict_idx = req.prev_log_index;
            while conflict_idx > base {
                let offset = (conflict_idx - 1 - base) as usize;
                if log[offset].term != conflict_term {
                    break;
                }
                conflict_idx -= 1;
            }
            return AppendEntriesResponse {
                term: req.term,
                success: false,
                match_index: 0,
                conflict_index: Some(conflict_idx),
            };
        }

        // 一致性检查通过，追加新日志
        let mut insert_idx = req.prev_log_index + 1;
        for entry in &req.entries {
            let offset = insert_idx.saturating_sub(base) as usize;
            if offset < log.len() {
                // 该位置已有日志
                if log[offset].term != entry.term {
                    // 冲突：删除该位置及之后的所有日志
                    log.truncate(offset);
                    log.push_back(entry.clone());
                }
                // 任期相同，保留现有日志即可
            } else {
                log.push_back(entry.clone());
            }
            insert_idx += 1;
        }

        let match_index = req
            .entries
            .last()
            .map(|e| e.index)
            .unwrap_or(req.prev_log_index);

        drop(log);

        // 更新 commit_index
        self.update_commit_index(req.leader_commit);

        AppendEntriesResponse {
            term: req.term,
            success: true,
            match_index,
            conflict_index: None,
        }
    }

    /// 处理 AppendEntries 响应（Leader 调用）
    pub fn handle_append_entries_response(
        &self,
        peer_id: &str,
        resp: &AppendEntriesResponse,
    ) -> MasterResult<()> {
        // 检查任期
        let mut current_term = self.current_term.lock();
        if resp.term > *current_term {
            *current_term = resp.term;
            *self.role.write() = RaftRole::Follower;
            *self.leader_id.write() = None;
            return Ok(());
        }
        if *self.role.read() != RaftRole::Leader {
            return Ok(());
        }
        drop(current_term);

        let mut ps = self.peer_state.lock();
        let peer = match ps.get_mut(peer_id) {
            Some(p) => p,
            None => return Ok(()),
        };

        if resp.success {
            peer.match_index = resp.match_index;
            peer.next_index = resp.match_index + 1;
            peer.last_append_time_ms = now_ms();
            drop(ps);

            // 尝试推进 commit_index
            self.advance_commit_index();

            // 更新 Leader 租约
            *self.leader_lease_expiry_ms.lock() = now_ms() + self.config.leader_lease_ms;
        } else {
            // 不一致，回退 next_index
            if let Some(conflict_idx) = resp.conflict_index {
                peer.next_index = conflict_idx;
            } else {
                peer.next_index = peer.next_index.saturating_sub(1).max(1);
            }
        }

        Ok(())
    }

    /// 推进 commit_index（Leader 调用）
    fn advance_commit_index(&self) {
        let log = self.log.lock();
        let base = *self.log_base_index.lock();
        let last_idx = log.back().map(|e| e.index).unwrap_or(base);
        let current_commit = *self.commit_index.lock();
        let current_term = *self.current_term.lock();

        // 找到大多数节点都已复制的最高日志索引
        let voter_count = self.voter_count().max(1);
        let majority = voter_count / 2 + 1;

        // 收集所有 match_index（包括 Leader 自己）
        let mut match_indices = vec![last_idx]; // Leader 自己
        let ps = self.peer_state.lock();
        for node in self.nodes.read().iter() {
            if node.is_voter && node.node_id != self.config.node_id {
                if let Some(p) = ps.get(&node.node_id) {
                    match_indices.push(p.match_index);
                }
            }
        }
        drop(ps);

        match_indices.sort_unstable_by(|a, b| b.cmp(a)); // 降序

        // majority-th 个元素就是新的 commit_index 的候选
        if match_indices.len() >= majority {
            let candidate = match_indices[majority - 1];
            if candidate > current_commit {
                // 只能提交当前任期的日志
                let offset = candidate.saturating_sub(base) as usize;
                if offset < log.len() && log[offset].term == current_term {
                    *self.commit_index.lock() = candidate;
                    *self.metrics.logs_committed.lock() += candidate.saturating_sub(current_commit);
                }
            }
        }
    }

    /// 更新 commit_index（Follower 根据 Leader 的 leader_commit 更新）
    fn update_commit_index(&self, leader_commit: u64) {
        let mut commit = self.commit_index.lock();
        if leader_commit > *commit {
            let last_idx = self.last_log_index();
            *commit = leader_commit.min(last_idx);
        }
    }

    /// 获取新提交的、待应用的日志（状态机消费）
    pub fn pending_applied_entries(&self, max_count: usize) -> Vec<RaftLogEntry> {
        let commit = *self.commit_index.lock();
        let mut applied = self.last_applied.lock();
        if *applied >= commit {
            return Vec::new();
        }

        let log = self.log.lock();
        let base = *self.log_base_index.lock();
        let start = (*applied + 1).saturating_sub(base) as usize;
        let end = commit.saturating_sub(base) as usize;
        let end = end.min(log.len() - 1);
        let count = (end + 1).saturating_sub(start).min(max_count);

        let mut entries = Vec::with_capacity(count);
        for i in start..start + count {
            if i < log.len() {
                entries.push(log[i].clone());
            }
        }

        if !entries.is_empty() {
            *applied = entries.last().unwrap().index;
            *self.metrics.logs_applied.lock() += entries.len() as u64;
        }

        entries
    }

    // -----------------------------------------------------------------------
    // 快照相关
    // -----------------------------------------------------------------------

    /// 检查是否需要创建快照
    pub fn should_snapshot(&self) -> bool {
        let log = self.log.lock();
        log.len() as u64 > self.config.max_log_entries
    }

    /// 创建快照（压缩日志到指定索引）
    /// snapshot_data 由调用方提供状态机序列化后的数据
    pub fn create_snapshot(&self, up_to_index: u64, snapshot_data: Vec<u8>) -> MasterResult<RaftSnapshotMeta> {
        let mut log = self.log.lock();
        let base = *self.log_base_index.lock();

        if up_to_index <= base || up_to_index > log.back().map(|e| e.index).unwrap_or(base) {
            return Err(MasterError::Internal(format!(
                "invalid snapshot index: {}, base={}",
                up_to_index, base
            )));
        }

        let up_to_offset = (up_to_index - base) as usize;
        let last_term = log[up_to_offset].term;

        // 截断日志，保留 up_to_index 作为新的哨兵
        log.drain(0..up_to_offset);
        // 第一个条目变为哨兵（index = up_to_index）
        *self.log_base_index.lock() = up_to_index;

        let meta = RaftSnapshotMeta {
            last_index: up_to_index,
            last_term,
            created_at_ms: now_ms(),
            size_bytes: snapshot_data.len() as u64,
        };

        *self.snapshot_meta.lock() = Some(meta.clone());
        *self.metrics.snapshots_created.lock() += 1;

        Ok(meta)
    }

    /// 安装快照（Follower 收到 Leader 的快照后调用）
    pub fn install_snapshot(&self, last_index: u64, last_term: u64, _data: Vec<u8>) -> MasterResult<()> {
        // 简化实现：替换日志为快照对应的哨兵条目
        let mut log = self.log.lock();
        log.clear();
        log.push_back(RaftLogEntry {
            index: last_index,
            term: last_term,
            log_type: RaftLogType::SnapshotMarker,
            data: Vec::new(),
            created_at_ms: now_ms(),
        });
        *self.log_base_index.lock() = last_index;

        // 更新 commit_index 和 last_applied
        let mut commit = self.commit_index.lock();
        if *commit < last_index {
            *commit = last_index;
        }
        drop(commit);

        let mut applied = self.last_applied.lock();
        if *applied < last_index {
            *applied = last_index;
        }
        drop(applied);

        let meta = RaftSnapshotMeta {
            last_index,
            last_term,
            created_at_ms: now_ms(),
            size_bytes: 0,
        };
        *self.snapshot_meta.lock() = Some(meta);

        Ok(())
    }

    /// 获取当前快照元数据
    pub fn snapshot_meta(&self) -> Option<RaftSnapshotMeta> {
        self.snapshot_meta.lock().clone()
    }

    // -----------------------------------------------------------------------
    // Leader 心跳
    // -----------------------------------------------------------------------

    /// Leader 发送心跳（空的 AppendEntries）
    /// 返回每个投票节点的心跳请求（由调用方实际发送）
    pub fn build_heartbeat(&self) -> Vec<(String, AppendEntriesRequest)> {
        if *self.role.read() != RaftRole::Leader {
            return Vec::new();
        }

        *self.metrics.heartbeats_sent.lock() += 1;

        let nodes = self.nodes.read().clone();
        let mut result = Vec::new();
        let commit_index = *self.commit_index.lock();
        let term = *self.current_term.lock();

        for node in &nodes {
            if node.node_id == self.config.node_id {
                continue;
            }
            // 取该节点的 match_index 作为 prev_log_index
            let prev_idx = self
                .peer_state
                .lock()
                .get(&node.node_id)
                .map(|p| p.match_index)
                .unwrap_or(0);

            let prev_term = if prev_idx == 0 {
                0
            } else {
                self.get_log_entry(prev_idx).map(|e| e.term).unwrap_or(0)
            };

            result.push((
                node.node_id.clone(),
                AppendEntriesRequest {
                    term,
                    leader_id: self.config.node_id.clone(),
                    prev_log_index: prev_idx,
                    prev_log_term: prev_term,
                    entries: Vec::new(),
                    leader_commit: commit_index,
                },
            ));
        }

        result
    }

    /// 周期性 tick（驱动选举超时、心跳等）
    /// 返回需要执行的动作提示
    pub fn tick(&self) -> RaftTickAction {
        let role = *self.role.read();
        match role {
            RaftRole::Leader => {
                // Leader 检查是否需要发送心跳
                // 调用方应周期性调用 build_heartbeat
                RaftTickAction::SendHeartbeat
            }
            RaftRole::Follower | RaftRole::Standby => {
                if self.is_election_timeout() && role == RaftRole::Follower {
                    RaftTickAction::StartElection
                } else {
                    RaftTickAction::None
                }
            }
        }
    }
}

/// Tick 动作提示
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftTickAction {
    /// 无需动作
    None,
    /// 需要发送心跳（Leader）
    SendHeartbeat,
    /// 需要发起选举（Follower 超时）
    StartElection,
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Default for RaftMaster {
    fn default() -> Self {
        Self::new(RaftConfig::default())
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_raft(node_id: &str) -> RaftMaster {
        RaftMaster::new(RaftConfig {
            node_id: node_id.to_string(),
            listen_addr: format!("127.0.0.1:{}", 9000 + node_id.len() as u16),
            election_timeout_ms: 3000,
            heartbeat_interval_ms: 500,
            max_log_entries: 100,
            leader_lease_ms: 1500,
        })
    }

    #[test]
    fn test_initial_state() {
        let raft = make_raft("node1");
        assert_eq!(raft.role(), RaftRole::Follower);
        assert_eq!(raft.current_term(), 0);
        assert_eq!(raft.last_log_index(), 0);
        assert_eq!(raft.last_log_term(), 0);
        assert!(raft.leader_id().is_none());
    }

    #[test]
    fn test_append_and_get_log() {
        let raft = make_raft("node1");
        // 先手动设为 Leader 才能 append
        *raft.role.write() = RaftRole::Leader;

        let idx = raft
            .append_log(RaftLogType::VolumeAllocation, b"test".to_vec())
            .unwrap();
        assert_eq!(idx, 1);

        let entry = raft.get_log_entry(1).unwrap();
        assert_eq!(entry.index, 1);
        assert_eq!(entry.log_type, RaftLogType::VolumeAllocation);
        assert_eq!(entry.data, b"test");
    }

    #[test]
    fn test_append_log_not_leader_fails() {
        let raft = make_raft("node1");
        let result = raft.append_log(RaftLogType::VolumeAllocation, b"test".to_vec());
        assert!(result.is_err());
    }

    #[test]
    fn test_request_vote_basic() {
        let raft1 = make_raft("node1");
        let raft2 = make_raft("node2");

        // node1 发起选举
        let (term, req) = raft1.start_election();
        assert_eq!(term, 1);
        assert_eq!(req.candidate_id, "node1");

        // node2 投票
        let resp = raft2.handle_request_vote(req);
        assert!(resp.vote_granted);
        assert_eq!(resp.term, 1);
    }

    #[test]
    fn test_request_vote_old_term_rejected() {
        let raft1 = make_raft("node1");
        let raft2 = make_raft("node2");

        // 让 raft2 的任期更高
        *raft2.current_term.lock() = 5;

        let req = RequestVoteRequest {
            term: 3,
            candidate_id: "node1".to_string(),
            last_log_index: 0,
            last_log_term: 0,
        };

        let resp = raft2.handle_request_vote(req);
        assert!(!resp.vote_granted);
        assert_eq!(resp.term, 5);
    }

    #[test]
    fn test_request_vote_already_voted() {
        let raft = make_raft("node2");
        // 设置当前任期为 1，并已投票给 node3
        *raft.current_term.lock() = 1;
        *raft.voted_for.lock() = Some("node3".to_string());

        let req = RequestVoteRequest {
            term: 1,
            candidate_id: "node1".to_string(),
            last_log_index: 0,
            last_log_term: 0,
        };

        // 同任期内，已经投给 node3，不应再投给 node1
        let resp = raft.handle_request_vote(req);
        assert!(!resp.vote_granted);
        assert_eq!(resp.term, 1);
    }

    #[test]
    fn test_append_entries_heartbeat() {
        let leader = make_raft("leader");
        let follower = make_raft("follower");

        // Leader 任期 1
        *leader.current_term.lock() = 1;
        *leader.role.write() = RaftRole::Leader;

        let req = AppendEntriesRequest {
            term: 1,
            leader_id: "leader".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        };

        let resp = follower.handle_append_entries(req);
        assert!(resp.success);
        assert_eq!(follower.leader_id().as_deref(), Some("leader"));
        assert_eq!(follower.current_term(), 1);
    }

    #[test]
    fn test_append_entries_with_data() {
        let leader = make_raft("leader");
        let follower = make_raft("follower");

        *leader.current_term.lock() = 1;
        *leader.role.write() = RaftRole::Leader;

        // Leader 追加日志
        let idx = leader
            .append_log(RaftLogType::VolumeAllocation, b"data1".to_vec())
            .unwrap();
        assert_eq!(idx, 1);

        // 构造 AppendEntries 请求
        let entry = leader.get_log_entry(1).unwrap();
        let req = AppendEntriesRequest {
            term: 1,
            leader_id: "leader".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![entry],
            leader_commit: 1,
        };

        let resp = follower.handle_append_entries(req);
        assert!(resp.success);
        assert_eq!(resp.match_index, 1);
        assert_eq!(follower.last_log_index(), 1);
    }

    #[test]
    fn test_append_entries_conflict() {
        let leader = make_raft("leader");
        let follower = make_raft("follower");

        // Follower 有不同任期的日志
        *follower.current_term.lock() = 1;
        {
            let mut log = follower.log.lock();
            log.push_back(RaftLogEntry {
                index: 1,
                term: 1,
                log_type: RaftLogType::NoOp,
                data: vec![],
                created_at_ms: 0,
            });
            log.push_back(RaftLogEntry {
                index: 2,
                term: 1,
                log_type: RaftLogType::NoOp,
                data: vec![],
                created_at_ms: 0,
            });
        }

        // Leader (term=2) 发送 term=2 的日志在 index=2
        *leader.current_term.lock() = 2;
        *leader.role.write() = RaftRole::Leader;

        let req = AppendEntriesRequest {
            term: 2,
            leader_id: "leader".to_string(),
            prev_log_index: 1,
            prev_log_term: 1,
            entries: vec![RaftLogEntry {
                index: 2,
                term: 2,
                log_type: RaftLogType::NoOp,
                data: vec![],
                created_at_ms: 0,
            }],
            leader_commit: 2,
        };

        let resp = follower.handle_append_entries(req);
        // 应该成功（index 1 匹配，index 2 被覆盖）
        assert!(resp.success);
        assert_eq!(resp.match_index, 2);
        assert_eq!(follower.last_log_index(), 2);
        assert_eq!(follower.current_term(), 2);
    }

    #[test]
    fn test_become_leader() {
        let raft = make_raft("node1");

        // 添加集群节点
        raft.add_node(RaftNodeInfo {
            node_id: "node1".to_string(),
            addr: "127.0.0.1:9001".to_string(),
            is_voter: true,
        });
        raft.add_node(RaftNodeInfo {
            node_id: "node2".to_string(),
            addr: "127.0.0.1:9002".to_string(),
            is_voter: true,
        });
        raft.add_node(RaftNodeInfo {
            node_id: "node3".to_string(),
            addr: "127.0.0.1:9003".to_string(),
            is_voter: true,
        });

        *raft.current_term.lock() = 1;
        raft.become_leader();

        assert_eq!(raft.role(), RaftRole::Leader);
        assert_eq!(raft.leader_id().as_deref(), Some("node1"));
        assert!(raft.is_leader_leased());

        // peer_state 应该有 node2 和 node3
        let ps = raft.peer_state.lock();
        assert!(ps.contains_key("node2"));
        assert!(ps.contains_key("node3"));
    }

    #[test]
    fn test_step_down() {
        let raft = make_raft("node1");
        *raft.role.write() = RaftRole::Leader;
        *raft.current_term.lock() = 1;

        raft.step_down(2, "node2".to_string());

        assert_eq!(raft.role(), RaftRole::Follower);
        assert_eq!(raft.current_term(), 2);
        assert_eq!(raft.leader_id().as_deref(), Some("node2"));
    }

    #[test]
    fn test_check_election_won() {
        let raft = make_raft("node1");
        for i in 1..=5 {
            raft.add_node(RaftNodeInfo {
                node_id: format!("node{}", i),
                addr: format!("127.0.0.1:900{}", i),
                is_voter: true,
            });
        }

        // 5 个节点，需要 3 票
        assert!(!raft.check_election_won(2));
        assert!(raft.check_election_won(3));
        assert!(raft.check_election_won(5));
    }

    #[test]
    fn test_snapshot() {
        let raft = make_raft("node1");
        *raft.role.write() = RaftRole::Leader;
        *raft.current_term.lock() = 1;

        // 添加 50 条日志
        for i in 0..50 {
            raft.append_log(
                RaftLogType::VolumeAllocation,
                format!("data-{}", i).into_bytes(),
            )
            .unwrap();
        }

        assert_eq!(raft.last_log_index(), 50);

        // 创建快照到 index 25
        let meta = raft.create_snapshot(25, b"snapshot-data".to_vec()).unwrap();
        assert_eq!(meta.last_index, 25);
        assert_eq!(meta.last_term, 1);

        // 日志应该被截断
        assert_eq!(*raft.log_base_index.lock(), 25);
        assert_eq!(raft.last_log_index(), 50);

        // 小于 base 的索引返回 None
        assert!(raft.get_log_entry(10).is_none());
        assert!(raft.get_log_entry(26).is_some());
    }

    #[test]
    fn test_raft_roles_display() {
        assert_eq!(RaftRole::Leader.to_string(), "Leader");
        assert_eq!(RaftRole::Follower.to_string(), "Follower");
        assert_eq!(RaftRole::Standby.to_string(), "Standby");
    }

    #[test]
    fn test_metrics() {
        let raft = make_raft("node1");
        let m = raft.metrics();
        let snap = m.snapshot();
        assert!(snap.contains_key("raft_elections_total"));
        assert!(snap.contains_key("raft_logs_committed"));
        assert!(snap.contains_key("raft_snapshots_created"));
    }

    #[test]
    fn test_election_timeout_check() {
        let raft = make_raft("node1");
        // 初始状态下未超时
        assert!(!raft.is_election_timeout());

        // 手动设置为很久以前
        *raft.last_heartbeat_time_ms.lock() = 0;
        assert!(raft.is_election_timeout());
    }

    #[test]
    fn test_pending_applied_entries() {
        let raft = make_raft("node1");
        *raft.role.write() = RaftRole::Leader;
        *raft.current_term.lock() = 1;

        for i in 0..10 {
            raft.append_log(
                RaftLogType::Heartbeat,
                format!("hb-{}", i).into_bytes(),
            )
            .unwrap();
        }

        // 手动设置 commit_index
        *raft.commit_index.lock() = 5;

        let entries = raft.pending_applied_entries(100);
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].index, 1);
        assert_eq!(entries[4].index, 5);

        // 再次调用应该返回空
        let entries2 = raft.pending_applied_entries(100);
        assert!(entries2.is_empty());
    }

    #[test]
    fn test_add_remove_nodes() {
        let raft = make_raft("node1");
        assert_eq!(raft.list_nodes().len(), 0);
        assert_eq!(raft.voter_count(), 0);

        raft.add_node(RaftNodeInfo {
            node_id: "node2".to_string(),
            addr: "127.0.0.1:9002".to_string(),
            is_voter: true,
        });
        assert_eq!(raft.list_nodes().len(), 1);
        assert_eq!(raft.voter_count(), 1);

        // 重复添加不增加
        raft.add_node(RaftNodeInfo {
            node_id: "node2".to_string(),
            addr: "127.0.0.1:9002".to_string(),
            is_voter: true,
        });
        assert_eq!(raft.list_nodes().len(), 1);

        raft.add_node(RaftNodeInfo {
            node_id: "node3".to_string(),
            addr: "127.0.0.1:9003".to_string(),
            is_voter: false,
        });
        assert_eq!(raft.list_nodes().len(), 2);
        assert_eq!(raft.voter_count(), 1);

        raft.remove_node("node2");
        assert_eq!(raft.list_nodes().len(), 1);
    }

    #[test]
    fn test_tick_action() {
        let raft = make_raft("node1");
        // Follower 初始不超时
        assert_eq!(raft.tick(), RaftTickAction::None);

        // Leader 返回 SendHeartbeat
        *raft.role.write() = RaftRole::Leader;
        assert_eq!(raft.tick(), RaftTickAction::SendHeartbeat);

        // Follower 超时返回 StartElection
        *raft.role.write() = RaftRole::Follower;
        *raft.last_heartbeat_time_ms.lock() = 0;
        assert_eq!(raft.tick(), RaftTickAction::StartElection);
    }

    #[test]
    fn test_log_type_serialization() {
        let types = vec![
            RaftLogType::VolumeAllocation,
            RaftLogType::Heartbeat,
            RaftLogType::ReplicaMigration,
            RaftLogType::ConfigChange,
            RaftLogType::SnapshotMarker,
            RaftLogType::NoOp,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let back: RaftLogType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, back);
        }
    }
}
