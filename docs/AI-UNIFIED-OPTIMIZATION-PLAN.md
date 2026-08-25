# 算子统一系统（OUS）架构优化方案：引入 L4 Agentic 闭环与形式化状态机

> **版本**：v1.0  
> **日期**：2026-08-21  
> **目标**：参考《AI 统一智能系统架构（AUS）》v1.0 的 L4 编排层设计理念，对 OUS 的编排与优化层进行深度升级，引入**形式化状态机（FSM）**、**循环守卫（Guards）**、**人机协同（HITL）**与**记忆巩固（Consolidation）**机制，使 OUS 具备真正的 Agentic 闭环能力。

---

## 1. 架构演进总结与对比

### 1.1 OUS 架构全景（当前态）

OUS 当前以“插件化运行时 + 双璇玑十四维治理”为核心，分层清晰，但在**Agentic 闭环控制**和**运行时状态机显式化**方面仍有提升空间。

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                      接入层 (Ingress)                                          │
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                   插件运行时内核 (Cordis Runtime)                              │
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                  编排与优化层 (Orchestration)  ← 优化重点                       │
│  ┌─────────────────────┐    ┌─────────────────────┐    ┌────────────────────┐ │
│  │  flow-ai (最优求解)  │    │  ai-agent (工作流)   │    │ mox-expert(治理) │ │
│  └─────────────────────┘    └─────────────────────┘    └────────────────────┘ │
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                      算子内核 (Operator Core)                                  │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 AUS 架构核心亮点（参考态）

AUS 的核心突破在于**将 L4 编排层具象化为一个可审计、可中断、可自愈的 Agentic 循环**。

1.  **形式化状态机（FSM）**：将 Agent 的生命周期定义为一组有限状态（`IDLE`, `PERCEIVE`, `PLAN`, `ACT`, `REFLECT` 等），状态转移规则明确，便于调试和验证。
2.  **循环守卫（Loop Guards）**：引入三条硬性守卫（预算熔断、进展检测、高风险 HITL），从根本上解决 Agent 死循环和不可控问题。
3.  **记忆巩固（Memory Consolidation）**：将运行时的“情景 Trace”通过巩固流水线转化为“短期/长期/程序性记忆”，实现知识复利。
4.  **双模型路由**：明确区分“强模型（Strong Model，用于规划/反思）”与“轻模型（Light Model，用于执行）”的调度策略，平衡能力与成本。

### 1.3 架构差距分析与优化目标

| 维度 | OUS 当前态 | AUS 参考态 | 优化目标 |
|------|-----------|-----------|---------|
| **L4 编排** | `ai-agent` 内部调度，隐式循环 | 显式 `AgenticLoop`，形式化状态机 | 引入 `OUS-Engine Loop`，显式状态转换 |
| **循环守卫** | 主要依赖“璇玑治理闸门”（静态、后置） | 运行时动态守卫（熔断、HITL、预算） | 实现运行时 `OUS-Guard`：预算/守恒/权限/HITL |
| **人机协同** | 隐式（通过权限控制） | 显式 `HITL` 状态，强风险动作拦截 | 引入 `HITL_PAUSE` 状态，支持实时干预 |
| **记忆体系** | 短期记忆（会话）、长期记忆（图谱） | 情景 Trace → 三层次记忆巩固 | 完善 `TraceConsolidator`，强化程序性记忆生成 |

---

## 2. OUS L4 编排层优化设计

### 2.1 引入 `OUS-Engine Loop` (Agentic 闭环)

我们将在 `platform/services/ai-agent/src/` 中引入一个新的核心组件 `OUS-Engine`，作为系统的“心脏”。它将接管原本分散的 `ai-agent` 和 `mox-expert` 调度逻辑，形成一个统一的、受守卫保护的 Agentic 闭环。

#### 2.1.1 形式化状态机（FSM）

定义 OUS 编排层的状态集合与转移规则：

```rust
/// OUS Agentic Loop 形式化状态机
#[derive(Debug, Clone, PartialEq)]
pub enum EngineState {
    /// 空闲态，等待任务接入
    IDLE,
    /// 感知态：接收输入、解析意图、载入上下文
    PERCEIVE,
    /// 规划态：强模型生成执行计划（Plan）
    PLAN,
    /// 执行态：按计划调用工具/算子/其他 Agent
    ACT,
    /// 观察态：收集执行结果、构建观察（Observation）
    OBSERVE,
    /// 反思态：评估结果、计算进度、决定是否继续
    REFLECT,
    /// 人机协同态：触发守卫③，等待人工确认/授权
    HITL_PAUSE,
    /// 生成态：汇总最终结果、生成响应
    GENERATE,
    /// 巩固态：将过程 Trace 写入长期记忆/图谱
    CONSOLIDATE,
    /// 完成态：任务成功结束
    DONE,
    /// 中止态：因错误/守卫触发而强制结束
    ABORT,
}

/// 状态转移规则定义
pub struct EngineTransition {
    pub from: EngineState,
    pub to: EngineState,
    pub condition: TransitionCondition,
}

pub enum TransitionCondition {
    /// 任务成功接收
    TaskAccepted,
    /// 规划成功
    PlanGenerated,
    /// 执行步骤完成
    StepCompleted,
    /// 执行高风险操作，需人工介入
    HighRiskDetected,
    /// 结果符合预期，流程结束
    GoalReached,
    /// 需要继续迭代
    NeedMoreIteration,
    /// 触发任意守卫（预算/守恒/超时等）
    GuardTriggered,
    /// 人类批准
    HumanApproved,
    /// 人类拒绝或超时
    HumanDenied,
}
```

#### 2.1.2 循环守卫（Loop Guards）实现

为 OUS 引擎引入三道动态守卫，防止 Agent 失控：

```rust
pub struct EngineGuard {
    pub max_steps: u32,
    pub current_steps: u32,
    pub budget_remaining: f64,
    pub progress_history: Vec<f64>,
    pub risk_level: RiskLevel,
}

impl EngineGuard {
    /// 检查所有守卫
    pub fn check(&mut self, context: &EngineContext) -> GuardResult {
        // 守卫①：步数/预算熔断
        if self.current_steps >= self.max_steps || self.budget_remaining <= 0.0 {
            return GuardResult::Triggered(GuardType::BudgetExhausted);
        }

        // 守卫②：进展检测（连续 N 步无显著提升）
        if self.is_stagnant() {
            return GuardResult::Triggered(GuardType::StagnationDetected);
        }

        // 守卫③：高风险动作强制 HITL
        if self.risk_level == RiskLevel::HIGH {
            return GuardResult::Triggered(GuardType::HighRiskAction);
        }

        GuardResult::Passed
    }

    fn is_stagnant(&self) -> bool {
        // 逻辑：如果最后 3 步的 progress_score 方差低于阈值，则视为停滞
        if self.progress_history.len() >= 3 {
            let recent: Vec<&f64> = self.progress_history.iter().rev().take(3).collect();
            let variance = calculate_variance(recent);
            variance < 0.01 // 阈值可调
        } else {
            false
        }
    }
}
```

### 2.2 人机协同（HITL）机制

基于 AUS 的启发，OUS 将在运行时支持显式的**暂停与干预**：

1.  **触发条件**：
    *   当 `EngineGuard` 检测到 `HighRiskAction`（如：执行删除、修改核心配置、调用外部付费 API）。
    *   当用户主动请求“人工审核”。
2.  **机制实现**：
    *   引擎进入 `HITL_PAUSE` 状态，将当前计划 `Plan` 和上下文 `Context` 序列化，推送给前端系统管理区 (`frontend-ui` `/admin?tab=hitl`)。
    *   前端展示风险详情（例如：“即将删除 1000 条历史知识图谱节点”），等待管理员审批。
    *   管理员可选择：`批准（APPROVE）`、`拒绝（DENY）`、`修改后批准（MODIFY_APPROVE）`。
    *   引擎收到指令后，从 `HITL_PAUSE` 恢复执行（或转入 `ABORT`）。

### 2.3 记忆巩固流水线（Trace Consolidation）

OUS 当前的记忆分散在会话 (`ai-agent`) 和图谱 (`kg-hub`) 中，缺乏统一的“淬炼”过程。

优化方案：
在引擎的 `CONSOLIDATE` 阶段，新增 `TraceConsolidator` 组件。

```rust
pub struct TraceConsolidator;

impl TraceConsolidator {
    /// 将完整的运行 Trace 转化为多层记忆
    pub fn consolidate(&self, trace: &EngineTrace) -> ConsolidationResult {
        // 1. 情景记忆 (Episodic Memory): 完整 Trace 存入短期存储 (Session Store)
        self.save_episodic(trace);

        // 2. 语义记忆 (Semantic Memory): 调用 LLM 抽取知识、实体、关系，更新图谱
        let kg_updates = self.extract_to_knowledge_graph(trace);
        
        // 3. 程序性记忆 (Procedural Memory): 识别任务模式，若为高频/成功模式，则固化为算子模板
        if self.is_high_success_pattern(trace) {
            let new_operator = self.distill_to_operator(trace);
            self.publish_to_marketplace(new_operator);
        }

        ConsolidationResult { kg_updates, new_operators }
    }
}
```

---

## 3. 代码落地计划

### Phase 1: 状态机与守卫 (Runtime Upgrade)
*   **文件**: `platform/services/ai-agent/src/engine/`
*   **任务**:
    *   [ ] 创建 `state_machine.rs`: 定义 `EngineState` 和 `EngineGuard`。
    *   [ ] 创建 `engine_loop.rs`: 实现 `run()` 主循环，串联感知-规划-执行-反思。
    *   [ ] 引入 `OUS-Guard` 到 `EngineContext`。

### Phase 2: HITL 机制集成
*   **文件**: `platform/gateway/runtime/src/handlers/`
*   **任务**:
    *   [ ] 创建 `hitl.rs`: WebSocket 接口，向前端推送 HITL 请求。
    *   [x] 更新 `frontend-ui` 系统管理区: 增加“HITL 审核”面板（`/admin?tab=hitl`，原 frontend-admin-ui 已裁撤并入）。

### Phase 3: 记忆巩固闭环
*   **文件**: `platform/services/kg-hub/src/consolidator.rs`
*   **任务**:
    *   [ ] 实现 `TraceConsolidator`。
    *   [ ] 对接 `ai-agent` 的 Trace 输出。

---

## 4. 总结

通过引入 AUS 架构的 **L4 Agentic 闭环**设计，OUS 将从一个“静态的算子编排平台”进化为一个“动态的、自我反思的智能体系统”。

1.  **可靠性提升**：动态守卫（预算/停滞/风险）将确保 Agent 在复杂任务中不会失控或陷入死循环。
2.  **可控性增强**：HITL 机制使企业管理员可以在关键节点“接管” Agent 的行为，满足合规要求。
3.  **知识复利**：记忆巩固机制让 Agent 的每一次成功执行都能转化为系统的长期能力（图谱更新/算子沉淀）。

**下一步**：基于此方案，开始实施 Phase 1 代码落地。