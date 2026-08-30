// Copyright (c) 2026 璇玑 RelGraph · 统一元数据层 (Unified Metadata Layer)
// Licensed under the MIT License.

//! Raft 分布式元数据节点
//!
//! 基于 Raft 共识算法的分布式元数据管理。
//! 支持多节点部署，保证元数据的一致性和高可用。

use parking_lot::RwLock;
use std::collections::HashMap;

use crate::error::{MetaError, MetaResult};
use crate::types::now_ms;

/// Raft 节点角色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftNodeRole {
    /// 领导者
    Leader,
    /// 跟随者
    Follower,
    /// 候选者
    Candidate,
}

impl RaftNodeRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            RaftNodeRole::Leader => "leader",
            RaftNodeRole::Follower => "follower",
            RaftNodeRole::Candidate => "candidate",
        }
    }
}

/// Raft 日志条目
#[derive(Debug, Clone)]
pub struct RaftLogEntry {
    /// 日志索引
    pub index: u64,
    /// 任期
    pub term: u64,
    /// 命令类型
    pub command_type: String,
    /// 命令数据（JSON）
    pub command_data: String,
    /// 提交时间戳
    pub timestamp: u64,
}

/// Raft 节点状态
#[derive(Debug, Clone)]
struct RaftState {
    /// 节点 ID
    node_id: String,
    /// 当前角色
    role: RaftNodeRole,
    /// 当前任期
    current_term: u64,
    /// 投票给的候选人
    voted_for: Option<String>,
    /// 日志条目
    log: Vec<RaftLogEntry>,
    /// 已提交的日志索引
    commit_index: u64,
    /// 最后应用的日志索引
    last_applied: u64,
    /// 集群节点 ID 列表
    peers: Vec<String>,
    /// 领导者 ID
    leader_id: Option<String>,
}

impl RaftState {
    fn new(node_id: String, peers: Vec<String>) -> Self {
        Self {
            node_id,
            role: RaftNodeRole::Follower,
            current_term: 0,
            voted_for: None,
            log: vec![RaftLogEntry {
                index: 0,
                term: 0,
                command_type: "noop".to_string(),
                command_data: String::new(),
                timestamp: now_ms(),
            }],
            commit_index: 0,
            last_applied: 0,
            peers,
            leader_id: None,
        }
    }

    fn last_log_index(&self) -> u64 {
        self.log.last().map(|e| e.index).unwrap_or(0)
    }

    fn last_log_term(&self) -> u64 {
        self.log.last().map(|e| e.term).unwrap_or(0)
    }
}

/// Raft 元数据节点
///
/// 提供基于 Raft 共识的分布式元数据管理。
/// 这是一个简化的 Raft 实现，用于架构验证。
pub struct RaftMetadataNode {
    /// Raft 状态
    state: RwLock<RaftState>,
    /// 应用的状态机结果
    applied_state: RwLock<HashMap<String, String>>,
}

impl RaftMetadataNode {
    /// 创建新的 Raft 节点
    pub fn new(node_id: &str, peers: Vec<String>) -> Self {
        Self {
            state: RwLock::new(RaftState::new(node_id.to_string(), peers)),
            applied_state: RwLock::new(HashMap::new()),
        }
    }

    /// 获取节点 ID
    pub fn node_id(&self) -> String {
        self.state.read().node_id.clone()
    }

    /// 获取当前角色
    pub fn role(&self) -> RaftNodeRole {
        self.state.read().role
    }

    /// 获取当前任期
    pub fn current_term(&self) -> u64 {
        self.state.read().current_term
    }

    /// 获取领导者 ID
    pub fn leader_id(&self) -> Option<String> {
        self.state.read().leader_id.clone()
    }

    /// 是否为领导者
    pub fn is_leader(&self) -> bool {
        self.state.read().role == RaftNodeRole::Leader
    }

    /// 集群节点数
    pub fn cluster_size(&self) -> usize {
        self.state.read().peers.len() + 1
    }

    /// 提交一条命令
    pub fn submit_command(&self, command_type: &str, command_data: &str) -> MetaResult<u64> {
        let mut state = self.state.write();

        if state.role != RaftNodeRole::Leader {
            return Err(MetaError::RaftError(
                "not the leader, cannot submit command".to_string(),
            ));
        }

        let index = state.last_log_index() + 1;
        let entry = RaftLogEntry {
            index,
            term: state.current_term,
            command_type: command_type.to_string(),
            command_data: command_data.to_string(),
            timestamp: now_ms(),
        };

        state.log.push(entry);

        // 简化：假设已提交（实际实现中需要等待多数派确认）
        state.commit_index = index;

        // 应用到状态机
        drop(state);
        self.apply_log(index);

        Ok(index)
    }

    /// 应用日志到状态机
    fn apply_log(&self, index: u64) {
        let state = self.state.read();
        if let Some(entry) = state.log.get(index as usize) {
            let mut applied = self.applied_state.write();

            match entry.command_type.as_str() {
                "set" => {
                    // 格式: "key=value"
                    if let Some((key, value)) = entry.command_data.split_once('=') {
                        applied.insert(key.to_string(), value.to_string());
                    }
                }
                "delete" => {
                    applied.remove(&entry.command_data);
                }
                _ => {}
            }

            drop(applied);
            drop(state);
            self.state.write().last_applied = index;
        }
    }

    /// 获取已应用的值
    pub fn get_applied(&self, key: &str) -> Option<String> {
        self.applied_state.read().get(key).cloned()
    }

    /// 成为领导者（用于测试）
    pub fn become_leader(&self) {
        let mut state = self.state.write();
        state.current_term += 1;
        state.role = RaftNodeRole::Leader;
        state.leader_id = Some(state.node_id.clone());
        state.voted_for = Some(state.node_id.clone());
    }

    /// 成为跟随者
    pub fn become_follower(&self, term: u64, leader_id: &str) {
        let mut state = self.state.write();
        if term > state.current_term {
            state.current_term = term;
        }
        state.role = RaftNodeRole::Follower;
        state.leader_id = Some(leader_id.to_string());
        state.voted_for = None;
    }

    /// 请求投票
    pub fn request_vote(
        &self,
        candidate_id: &str,
        candidate_term: u64,
        last_log_index: u64,
        last_log_term: u64,
    ) -> MetaResult<bool> {
        let mut state = self.state.write();

        // 如果候选人任期比自己小，拒绝
        if candidate_term < state.current_term {
            return Ok(false);
        }

        // 更新任期
        if candidate_term > state.current_term {
            state.current_term = candidate_term;
            state.voted_for = None;
            state.role = RaftNodeRole::Follower;
        }

        // 检查是否已投票
        if state.voted_for.is_some() && state.voted_for.as_deref() != Some(candidate_id) {
            return Ok(false);
        }

        // 检查候选人日志是否至少和自己一样新
        let my_last_term = state.last_log_term();
        let my_last_index = state.last_log_index();

        let log_ok = last_log_term > my_last_term
            || (last_log_term == my_last_term && last_log_index >= my_last_index);

        if !log_ok {
            return Ok(false);
        }

        // 投票
        state.voted_for = Some(candidate_id.to_string());
        Ok(true)
    }

    /// 追加日志条目（模拟 RPC）
    pub fn append_entries(
        &self,
        leader_id: &str,
        leader_term: u64,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<RaftLogEntry>,
        leader_commit: u64,
    ) -> MetaResult<bool> {
        let mut state = self.state.write();

        // 如果领导者任期比自己小，拒绝
        if leader_term < state.current_term {
            return Ok(false);
        }

        // 更新任期和角色
        state.current_term = leader_term;
        state.role = RaftNodeRole::Follower;
        state.leader_id = Some(leader_id.to_string());

        // 检查前一日志是否匹配
        if prev_log_index > 0 {
            if prev_log_index >= state.log.len() as u64 {
                return Ok(false);
            }
            if state.log[prev_log_index as usize].term != prev_log_term {
                return Ok(false);
            }
        }

        // 追加新条目
        let mut insert_index = prev_log_index + 1;
        for entry in entries {
            if insert_index < state.log.len() as u64 {
                if state.log[insert_index as usize].term != entry.term {
                    // 冲突，截断
                    state.log.truncate(insert_index as usize);
                    state.log.push(entry);
                }
            } else {
                state.log.push(entry);
            }
            insert_index += 1;
        }

        // 更新提交索引
        if leader_commit > state.commit_index {
            let new_commit = leader_commit.min(state.last_log_index());
            state.commit_index = new_commit;

            // 应用新提交的日志
            let last_applied = state.last_applied;
            drop(state);

            for i in (last_applied + 1)..=new_commit {
                self.apply_log(i);
            }
        }

        Ok(true)
    }

    /// 提交索引
    pub fn commit_index(&self) -> u64 {
        self.state.read().commit_index
    }

    /// 最后应用索引
    pub fn last_applied(&self) -> u64 {
        self.state.read().last_applied
    }

    /// 日志长度
    pub fn log_len(&self) -> usize {
        self.state.read().log.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raft_node_initial_state() {
        let node = RaftMetadataNode::new("node1", vec!["node2".to_string(), "node3".to_string()]);

        assert_eq!(node.role(), RaftNodeRole::Follower);
        assert_eq!(node.current_term(), 0);
        assert_eq!(node.cluster_size(), 3);
        assert!(!node.is_leader());
        assert!(node.leader_id().is_none());
    }

    #[test]
    fn test_become_leader() {
        let node = RaftMetadataNode::new("node1", vec!["node2".to_string()]);

        node.become_leader();
        assert!(node.is_leader());
        assert_eq!(node.role(), RaftNodeRole::Leader);
        assert_eq!(node.leader_id(), Some("node1".to_string()));
        assert_eq!(node.current_term(), 1);
    }

    #[test]
    fn test_submit_command_as_leader() {
        let node = RaftMetadataNode::new("node1", vec!["node2".to_string()]);
        node.become_leader();

        let index = node.submit_command("set", "foo=bar").unwrap();
        assert_eq!(index, 1);

        // 验证状态机应用
        assert_eq!(node.get_applied("foo"), Some("bar".to_string()));
    }

    #[test]
    fn test_submit_command_as_follower_fails() {
        let node = RaftMetadataNode::new("node1", vec!["node2".to_string()]);

        let result = node.submit_command("set", "foo=bar");
        assert!(result.is_err());
    }

    #[test]
    fn test_request_vote() {
        let node = RaftMetadataNode::new("node1", vec!["node2".to_string()]);

        // 候选人有更新的任期和日志
        let granted = node
            .request_vote("node2", 1, 1, 0)
            .unwrap();
        assert!(granted);

        // 同一任期内不能再投给别人
        let granted = node
            .request_vote("node3", 1, 1, 0)
            .unwrap();
        assert!(!granted);
    }

    #[test]
    fn test_request_vote_old_term() {
        let node = RaftMetadataNode::new("node1", vec!["node2".to_string()]);
        node.become_leader(); // term = 1

        let granted = node
            .request_vote("node2", 0, 0, 0)
            .unwrap();
        assert!(!granted);
    }

    #[test]
    fn test_append_entries() {
        let leader = RaftMetadataNode::new("leader", vec!["follower".to_string()]);
        let follower = RaftMetadataNode::new("follower", vec!["leader".to_string()]);

        leader.become_leader();
        let entry = RaftLogEntry {
            index: 1,
            term: 1,
            command_type: "set".to_string(),
            command_data: "key=value".to_string(),
            timestamp: now_ms(),
        };

        let success = follower
            .append_entries("leader", 1, 0, 0, vec![entry], 1)
            .unwrap();
        assert!(success);
        assert_eq!(follower.role(), RaftNodeRole::Follower);
        assert_eq!(follower.current_term(), 1);
        assert_eq!(follower.commit_index(), 1);
        assert_eq!(follower.get_applied("key"), Some("value".to_string()));
    }

    #[test]
    fn test_append_entries_old_term() {
        let follower = RaftMetadataNode::new("follower", vec!["leader".to_string()]);
        follower.become_leader(); // term = 1

        let success = follower
            .append_entries("old_leader", 0, 0, 0, vec![], 0)
            .unwrap();
        assert!(!success);
    }

    #[test]
    fn test_delete_command() {
        let node = RaftMetadataNode::new("node1", vec!["node2".to_string()]);
        node.become_leader();

        node.submit_command("set", "foo=bar").unwrap();
        assert_eq!(node.get_applied("foo"), Some("bar".to_string()));

        node.submit_command("delete", "foo").unwrap();
        assert_eq!(node.get_applied("foo"), None);
    }
}
