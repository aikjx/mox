// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 对话上下文管理：会话状态、历史轮次、实体记忆、任务进度。
//!
//! ## 上下文层次
//! - **会话级**：session_id、用户、创建时间、当前活跃任务
//! - **轮次级**：每轮用户输入 + AI 回复 + 意图理解结果
//! - **记忆级**：跨轮次沉淀的实体/偏好/历史（P2 接入 mox-memory）
//!
//! ## 设计原则
//! - 纯内存实现，P1 零外部依赖
//! - 可序列化，方便持久化到 SQLite / Redis
//! - 支持实体继承：上轮提取的实体在后续轮次自动补全（"它" → 上轮的项目）

use ahash::RandomState;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use crate::entity::Entity;
use crate::pipeline::IntentUnderstanding;
use crate::task_decomp::{TaskPlan, StepStatus};

// ─── 核心类型 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    /// 会话 ID
    pub session_id: String,
    /// 用户 ID
    pub user_id: String,
    /// 租户 ID
    pub tenant_id: Option<String>,
    /// 会话标题（自动生成）
    pub title: String,
    /// 创建时间（UNIX 秒）
    pub created_at: i64,
    /// 最后活跃时间（UNIX 秒）
    pub last_active_at: i64,
    /// 历史轮次（最新在末尾）
    pub turns: Vec<ConversationTurn>,
    /// 最大保留轮次数
    pub max_turns: usize,
    /// 当前活跃的任务计划（可能为 None）
    pub active_task: Option<TaskPlan>,
    /// 实体记忆池：跨轮次沉淀的实体
    pub entity_memory: Vec<Entity>,
    /// 实体记忆最大容量
    pub max_memory_entities: usize,
    /// 对话状态
    pub state: ConversationState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationState {
    /// 空闲 / 等待输入
    Idle,
    /// AI 思考中
    Thinking,
    /// 执行任务中
    Executing,
    /// 等待用户确认
    WaitingConfirmation,
    /// 多轮澄清中
    Clarifying,
    /// 已结束
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// 轮次号（从 1 开始）
    pub turn_id: usize,
    /// 用户输入
    pub user_message: String,
    /// AI 回复（可能为空，流式生成中）
    pub ai_message: Option<String>,
    /// 意图理解结果（可选）
    pub understanding: Option<IntentUnderstanding>,
    /// 时间戳（UNIX 秒）
    pub timestamp: i64,
    /// 轮次耗时（ms）
    pub duration_ms: u64,
}

// ─── 实现 ────────────────────────────────────────────────────────────────────

impl ConversationContext {
    /// 创建新会话
    pub fn new(session_id: String, user_id: String) -> Self {
        let now = chrono_now_sec();
        Self {
            session_id,
            user_id,
            tenant_id: None,
            title: "新对话".into(),
            created_at: now,
            last_active_at: now,
            turns: vec![],
            max_turns: 50,
            active_task: None,
            entity_memory: vec![],
            max_memory_entities: 50,
            state: ConversationState::Idle,
        }
    }

    pub fn with_tenant(mut self, tenant_id: String) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    pub fn with_max_turns(mut self, max: usize) -> Self {
        self.max_turns = max;
        self
    }

    // ── 轮次管理 ──────────────────────────────────────────────────────────

    /// 开始新一轮（用户输入）
    pub fn start_turn(&mut self, user_message: &str) -> usize {
        let turn_id = self.turns.len() + 1;
        let now = chrono_now_sec();
        self.last_active_at = now;
        self.state = ConversationState::Thinking;

        self.turns.push(ConversationTurn {
            turn_id,
            user_message: user_message.to_string(),
            ai_message: None,
            understanding: None,
            timestamp: now,
            duration_ms: 0,
        });

        // 自动标题：首轮取前 20 字
        if turn_id == 1 {
            let mut t = user_message.chars().take(20).collect::<String>();
            if user_message.chars().count() > 20 { t.push('…'); }
            if !t.trim().is_empty() {
                self.title = t;
            }
        }

        // 超量裁剪
        if self.turns.len() > self.max_turns {
            let excess = self.turns.len() - self.max_turns;
            self.turns.drain(0..excess);
        }

        turn_id
    }

    /// 完成当前轮（AI 回复 + 理解结果）
    pub fn complete_turn(
        &mut self,
        ai_message: &str,
        understanding: Option<IntentUnderstanding>,
        duration_ms: u64,
    ) {
        if let Some(turn) = self.turns.last_mut() {
            turn.ai_message = Some(ai_message.to_string());
            turn.understanding = understanding;
            turn.duration_ms = duration_ms;
        }
        self.last_active_at = chrono_now_sec();
        self.state = ConversationState::Idle;

        // 沉淀实体到记忆池（先收集再写入，避免借用冲突）
        let mut entities_to_memorize = Vec::new();
        if let Some(turn) = self.turns.last() {
            if let Some(udl) = &turn.understanding {
                entities_to_memorize.extend(udl.entities.clone());
            }
        }
        if !entities_to_memorize.is_empty() {
            self.memorize_entities(&entities_to_memorize);
        }
    }

    /// 获取当前轮次
    pub fn current_turn(&self) -> Option<&ConversationTurn> {
        self.turns.last()
    }

    /// 获取最近 N 轮
    pub fn recent_turns(&self, n: usize) -> &[ConversationTurn] {
        let start = self.turns.len().saturating_sub(n);
        &self.turns[start..]
    }

    // ── 任务管理 ──────────────────────────────────────────────────────────

    /// 设置活跃任务
    pub fn set_active_task(&mut self, plan: TaskPlan) {
        self.active_task = Some(plan);
        self.state = ConversationState::Executing;
    }

    /// 更新步骤状态
    pub fn update_step_status(&mut self, step_id: &str, status: StepStatus) -> bool {
        if let Some(task) = &mut self.active_task {
            if let Some(step) = task.steps.iter_mut().find(|s| s.id == step_id) {
                step.status = status;
                // 如果有步骤需要确认，更新状态
                if matches!(status, StepStatus::WaitingConfirmation) {
                    self.state = ConversationState::WaitingConfirmation;
                }
                return true;
            }
        }
        false
    }

    /// 完成活跃任务
    pub fn complete_task(&mut self) -> Option<TaskPlan> {
        self.state = ConversationState::Idle;
        self.active_task.take()
    }

    /// 获取当前活跃任务
    pub fn active_task(&self) -> Option<&TaskPlan> {
        self.active_task.as_ref()
    }

    // ── 实体记忆 ──────────────────────────────────────────────────────────

    /// 沉淀实体到记忆池
    pub fn memorize_entities(&mut self, entities: &[Entity]) {
        // 简单策略：追加 + 去重（按 normalized 去重）
        let mut seen: HashSet<String, RandomState> = HashSet::with_hasher(RandomState::new());
        for e in &self.entity_memory {
            if let Some(norm) = &e.normalized {
                seen.insert(format!("{:?}:{}", e.etype, norm));
            }
        }

        let mut new_ents: Vec<Entity> = entities.iter()
            .filter(|e| {
                let key = if let Some(norm) = &e.normalized {
                    format!("{:?}:{}", e.etype, norm)
                } else {
                    format!("{:?}:{}", e.etype, e.text)
                };
                seen.insert(key)
            })
            .cloned()
            .collect();

        self.entity_memory.append(&mut new_ents);

        // 裁剪
        if self.entity_memory.len() > self.max_memory_entities {
            let excess = self.entity_memory.len() - self.max_memory_entities;
            self.entity_memory.drain(0..excess);
        }
    }

    /// 从记忆中获取某类型的实体
    pub fn recall_entities(&self, etype: crate::entity::EntityType) -> Vec<&Entity> {
        self.entity_memory.iter().filter(|e| e.etype == etype).collect()
    }

    /// 获取全部记忆实体
    pub fn all_memory(&self) -> &[Entity] {
        &self.entity_memory
    }

    // ── 上下文补充 ────────────────────────────────────────────────────────

    /// 利用记忆补全实体信息不足的情况
    /// 返回应注入到 pipeline 的补充实体列表
    pub fn contextual_entities(&self) -> Vec<Entity> {
        // P1 简化：直接返回最近记忆中的高置信度实体
        self.entity_memory
            .iter()
            .rev()
            .filter(|e| e.confidence >= 0.7)
            .take(10)
            .cloned()
            .collect()
    }

    /// 获取对话摘要（用于 prompt 拼接）
    pub fn conversation_summary(&self, max_turns: usize) -> String {
        let recent = self.recent_turns(max_turns);
        let mut summary = String::new();
        for turn in recent {
            summary.push_str(&format!("用户: {}\n", turn.user_message));
            if let Some(ai) = &turn.ai_message {
                summary.push_str(&format!("AI: {}\n", ai));
            }
        }
        summary
    }

    // ── 状态管理 ──────────────────────────────────────────────────────────

    pub fn set_state(&mut self, state: ConversationState) {
        self.state = state;
    }

    pub fn state(&self) -> ConversationState {
        self.state
    }

    pub fn end(&mut self) {
        self.state = ConversationState::Ended;
    }

    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }
}

// ─── 会话管理器（多会话） ────────────────────────────────────────────────────

pub struct SessionManager {
    sessions: HashMap<String, ConversationContext, RandomState>,
    max_sessions_per_user: usize,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::with_hasher(RandomState::new()),
            max_sessions_per_user: 20,
        }
    }

    /// 创建新会话
    pub fn create_session(&mut self, user_id: &str) -> ConversationContext {
        let session_id = uuid::Uuid::now_v7().to_string();
        let ctx = ConversationContext::new(session_id.clone(), user_id.to_string());
        self.sessions.insert(session_id.clone(), ctx.clone());
        ctx
    }

    /// 获取会话
    pub fn get_session(&self, session_id: &str) -> Option<&ConversationContext> {
        self.sessions.get(session_id)
    }

    /// 获取可变会话
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut ConversationContext> {
        self.sessions.get_mut(session_id)
    }

    /// 列出用户的所有会话
    pub fn list_user_sessions(&self, user_id: &str) -> Vec<&ConversationContext> {
        let mut list: Vec<&ConversationContext> = self.sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .collect();
        list.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at));
        list
    }

    /// 删除会话
    pub fn delete_session(&mut self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }

    /// 会话总数
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self { Self::new() }
}

// ─── 时间辅助（不依赖 tokio，仅用 std） ──────────────────────────────────────

fn chrono_now_sec() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ─── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, EntityType};
    use crate::task_decomp::RiskLevel;
    use crate::task_decomp::TaskStep;
    use crate::task_decomp::StepStatus;

    fn sample_entity(etype: EntityType, text: &str, norm: &str) -> Entity {
        Entity {
            etype,
            text: text.into(),
            normalized: Some(norm.into()),
            confidence: 0.9,
            start: 0,
            end: text.len(),
        }
    }

    #[test]
    fn creates_new_context() {
        let ctx = ConversationContext::new("sess-1".into(), "user-1".into());
        assert_eq!(ctx.session_id, "sess-1");
        assert_eq!(ctx.user_id, "user-1");
        assert_eq!(ctx.turn_count(), 0);
        assert_eq!(ctx.state, ConversationState::Idle);
    }

    #[test]
    fn start_turn_adds_message() {
        let mut ctx = ConversationContext::new("sess-1".into(), "user-1".into());
        let turn_id = ctx.start_turn("你好");
        assert_eq!(turn_id, 1);
        assert_eq!(ctx.turn_count(), 1);
        assert_eq!(ctx.state, ConversationState::Thinking);
        assert_eq!(ctx.title, "你好");
    }

    #[test]
    fn title_truncated_for_long_input() {
        let mut ctx = ConversationContext::new("sess-1".into(), "user-1".into());
        let long_msg = "这是一个非常非常长的用户输入消息，超过了二十个字符的限制";
        ctx.start_turn(long_msg);
        assert!(ctx.title.ends_with('…'));
    }

    #[test]
    fn complete_turn_sets_ai_response() {
        let mut ctx = ConversationContext::new("sess-1".into(), "user-1".into());
        ctx.start_turn("你好");
        ctx.complete_turn("你好呀！", None, 150);
        let turn = ctx.current_turn().unwrap();
        assert_eq!(turn.ai_message.as_deref(), Some("你好呀！"));
        assert_eq!(turn.duration_ms, 150);
        assert_eq!(ctx.state, ConversationState::Idle);
    }

    #[test]
    fn entities_memorized_across_turns() {
        let mut ctx = ConversationContext::new("sess-1".into(), "user-1".into());
        ctx.start_turn("帮我分析上个月的销售数据");
        let entities = vec![sample_entity(EntityType::TimeRange, "上个月", "last_month")];
        // 直接测试 memorize
        ctx.memorize_entities(&entities);
        assert!(!ctx.entity_memory.is_empty());
        assert_eq!(ctx.recall_entities(EntityType::TimeRange).len(), 1);
    }

    #[test]
    fn duplicate_entities_deduped() {
        let mut ctx = ConversationContext::new("sess-1".into(), "user-1".into());
        let e1 = sample_entity(EntityType::TimeRange, "上个月", "last_month");
        let e2 = sample_entity(EntityType::TimeRange, "上月", "last_month"); // 同 normalized
        ctx.memorize_entities(&[e1]);
        ctx.memorize_entities(&[e2]);
        // 两个实体 normalized 相同，应去重为 1 个
        assert_eq!(ctx.recall_entities(EntityType::TimeRange).len(), 1);
    }

    #[test]
    fn task_management() {
        let mut ctx = ConversationContext::new("sess-1".into(), "user-1".into());
        let plan = TaskPlan {
            plan_id: "plan-1".into(),
            intent: "test".into(),
            user_query: "test".into(),
            steps: vec![
                TaskStep {
                    id: "step-1".into(),
                    name: "第一步".into(),
                    description: "".into(),
                    capability: "chat".into(),
                    risk: RiskLevel::Low,
                    status: StepStatus::Pending,
                    depends_on: vec![],
                    params: HashMap::with_hasher(RandomState::new()),
                    est_duration_sec: 5,
                },
            ],
            requires_overall_confirmation: false,
            parallel_groups: vec![],
            total_est_duration_sec: 5,
        };

        ctx.set_active_task(plan);
        assert!(ctx.active_task.is_some());
        assert_eq!(ctx.state, ConversationState::Executing);

        let ok = ctx.update_step_status("step-1", StepStatus::Running);
        assert!(ok);
        assert_eq!(
            ctx.active_task.as_ref().unwrap().steps[0].status,
            StepStatus::Running
        );

        let plan = ctx.complete_task();
        assert!(plan.is_some());
        assert_eq!(ctx.state, ConversationState::Idle);
        assert!(ctx.active_task.is_none());
    }

    #[test]
    fn conversation_summary_works() {
        let mut ctx = ConversationContext::new("sess-1".into(), "user-1".into());
        ctx.start_turn("你好");
        ctx.complete_turn("你好呀", None, 100);
        let summary = ctx.conversation_summary(5);
        assert!(summary.contains("用户: 你好"));
        assert!(summary.contains("AI: 你好呀"));
    }

    #[test]
    fn session_manager_creates_and_lists() {
        let mut mgr = SessionManager::new();
        let ctx = mgr.create_session("user-1");
        let sid = ctx.session_id.clone();
        assert!(mgr.get_session(&sid).is_some());
        assert_eq!(mgr.session_count(), 1);

        let sessions = mgr.list_user_sessions("user-1");
        assert_eq!(sessions.len(), 1);

        let deleted = mgr.delete_session(&sid);
        assert!(deleted);
        assert_eq!(mgr.session_count(), 0);
    }

    #[test]
    fn session_manager_multiple_sessions() {
        let mut mgr = SessionManager::new();
        mgr.create_session("user-1");
        mgr.create_session("user-1");
        mgr.create_session("user-2");
        assert_eq!(mgr.list_user_sessions("user-1").len(), 2);
        assert_eq!(mgr.list_user_sessions("user-2").len(), 1);
        assert_eq!(mgr.session_count(), 3);
    }

    #[test]
    fn max_turns_enforced() {
        let mut ctx = ConversationContext::new("sess-1".into(), "user-1".into())
            .with_max_turns(5);
        for i in 0..10 {
            ctx.start_turn(&format!("msg-{}", i));
            ctx.complete_turn(&format!("reply-{}", i), None, 10);
        }
        assert_eq!(ctx.turn_count(), 5);
        // 最早的应该被裁掉
        assert!(ctx.turns[0].user_message.starts_with("msg-5"));
    }

    #[test]
    fn contextual_entities_returns_high_confidence() {
        let mut ctx = ConversationContext::new("sess-1".into(), "user-1".into());
        let mut e = sample_entity(EntityType::Project, "项目A", "project_a");
        e.confidence = 0.95;
        ctx.memorize_entities(&[e]);
        let contextual = ctx.contextual_entities();
        assert!(!contextual.is_empty());
    }

    #[test]
    fn end_sets_state() {
        let mut ctx = ConversationContext::new("sess-1".into(), "user-1".into());
        ctx.end();
        assert_eq!(ctx.state, ConversationState::Ended);
    }
}
