# 专家联盟 AI 对话系统 - 架构优化设计文档 V2.0

> **标题**：专家联盟AI对话系统架构优化设计文档
> **版本**：V2.0
> **权威等级**：🟡参考（目标设计）
> **编号**：EA-DOC-051
> **文档层级**：L3需求规格层
> **最后更新日期**：2026-08-31
> **主责联盟**：开发联盟 R
> **单源声明**：本文档是专家联盟AI对话系统V2.0架构优化的目标设计参考。当前实际实现以 `platform/domains/alliance/` 代码为准。冲突时以 `docs/standards/expert-alliance-normalization-mode.md` 为准。

> 📌 **文档状态声明**  
> 本文档描述的"15+专家类型"、L0-L5六层架构等为**目标设计**。当前Rust alliance域实际内置10个领域专家。本文档的需求规格可作为未来扩展参考。

## 文档信息
| 属性 | 值 |
|------|-----|
| 文档版本 | V2.0 架构优化版 |
| 创建时间 | 2026-08-21 |
| 文档状态 | 设计评审中 |
| 密级 | 内部公开 |
| 优化依据 | AIS 十大 AI 工具评估 + 现有架构分析 |
| 核心关键词 | 插件化编排、上下文工程、学习闭环、事件驱动 |

---

## 一、架构优化总结

### 1.1 优化背景

基于对 `ais/` 目录下 10 个主流 AI 编码 Agent 工具的深度技术评估，结合现有专家联盟 V1.0 架构，识别出以下核心优化方向：

| 优化维度 | 参考工具 | 核心机制 | 优化收益 |
|----------|----------|----------|----------|
| **插件化编排** | deepseek-harness | 一切皆插件、可替换主循环 | 架构解耦、可扩展性提升 100% |
| **上下文工程** | aider (RepoMap) | 个性化 PageRank 上下文选择 | 专家匹配精准度提升 40% |
| **学习闭环** | hermes-agent | 轨迹压缩→Skill 提取→记忆持久化 | 专家能力持续进化 |
| **事件驱动** | openhands | 事件流架构、Agent/Runtime 解耦 | 系统解耦、异步能力 |
| **双模式编排** | cline | Plan/Act 双阶段 + 检查点回滚 | 安全可控、用户可介入 |
| **状态管理** | claude-code | 会话日志溯源 + 状态向量守恒 | 可追溯、可恢复 |

### 1.2 核心优化目标

1. **从"工具集合"到"编排底座"**：将专家联盟从固定的专家调度升级为可插件化的 Agent 编排运行时
2. **从"静态匹配"到"动态图路由"**：引入基于图算法的上下文工程，实现专家能力图谱的动态匹配
3. **从"单次执行"到"学习闭环"**：构建专家经验沉淀、能力进化的完整学习闭环
4. **从"同步调用"到"事件驱动"**：引入事件流架构，支持异步协作和跨会话交互

---

## 二、优化后分层架构（L0-L5）

### 2.1 架构全景图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    专家联盟 AI 对话系统 V2.0                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  L0 接入与感知层（Perception Layer）                                │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │  · 多模态输入网关（文本/语音/文件）                                  │   │
│  │  · API 网关 + 意图归一化（MCP 协议兼容）                             │   │
│  │  · 实时流处理（WebSocket / SSE）                                    │   │
│  │  · 用户画像注入与上下文预取                                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  L1 记忆与知识层（Memory Layer）                                    │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │  · 短期记忆（会话上下文 / 状态向量投影）                              │   │
│  │  · 长期记忆（向量库 + 专家能力图谱）                                │   │
│  │  · 程序性记忆（专家技能 / 工作流模板）                               │   │
│  │  · 巩固流水线（情景 Trace → 可复用资产）                             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  L2 能力与工具层（Capability Layer）                                │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │  · 专家插件市场（15+ 专家类型，可动态扩展）                          │   │
│  │  · 工具集（代码分析 / 图谱计算 / 文档生成）                          │   │
│  │  · 外部集成（LLM 网关 / MCP Server / API 聚合）                     │   │
│  │  · 安全沙箱（权限审批 / 执行隔离）                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  L3 编排与调度层（Orchestration Layer）◄── 系统心脏                  │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │  · 插件化编排引擎（可替换主循环）                                    │   │
│  │  · 专家路由器（基于 PageRank 的能力图谱匹配）                         │   │
│  │  · 事件流处理器（Agent Event Bus）                                   │   │
│  │  · 状态机与检查点（Plan/Act 双模式 + 回滚）                          │   │
│  │  · 学习闭环引擎（轨迹压缩 → Skill 提取 → 记忆更新）                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  L4 治理与安全层（Governance Layer）                                │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │  · 双璇玑十四维诊断（业务七维 + 开发七维）                           │   │
│  │  · 归一化裁决引擎（硬约束优先）                                     │   │
│  │  · ⛨ 验证网关（最高权限否决）                                       │   │
│  │  · 治理闸门 G3（审计链 / RBAC / SLA）                                │   │
│  │  · 关图同步与漂移门禁                                               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  L5 交互与展示层（Presentation Layer）                              │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │  · 专家对话界面（多轮对话 / 流式输出）                               │   │
│  │  · 企业级管理控制台（仪表盘 / 会话中心 / 能力图谱）                   │   │
│  │  · 分析报告可视化（图谱 / 流程图 / 报告）                             │   │
│  │  · 治理视图（质量门禁 / 审计链 / 优化建议）                           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  基础设施层：LLM 网关 / 算子系统 / 存储服务 / 安全模块 / 监控告警           │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 与原 V1.0 架构对比

| 对比维度 | V1.0 架构 | V2.0 架构 | 优化提升 |
|----------|-----------|-----------|----------|
| **架构分层** | 6 层（对话引擎/专家调度/算法联盟等） | 6 层（L0-L5，职责更清晰） | 解耦度提升 |
| **专家匹配** | 关键词匹配 + 评分排序 | PageRank 能力图谱动态匹配 | 精准度 +40% |
| **执行模式** | 同步单轮调用 | 事件流 + Plan/Act 双模式 | 灵活度提升 |
| **学习能力** | 静态专家库 | 轨迹压缩 → Skill 提取闭环 | 自进化能力 |
| **插件化** | 固定专家类型 | 一切皆插件，可热插拔 | 扩展性翻倍 |
| **治理机制** | 基础审计 | 双璇玑十四维 + ⛨ 验证网关 | 企业级增强 |

---

## 三、核心模块优化设计

### 3.1 L3 编排与调度层（系统心脏）

#### 3.1.1 插件化编排引擎（参考 deepseek-harness）

**设计理念**：将编排引擎本身设计为可替换插件，实现 `Agent = Model + Harness` 的架构哲学。

```javascript
// 插件化编排引擎核心接口
class OrchestrationEngine {
  constructor() {
    this.plugins = new Map();         // 插件注册表
    this.eventBus = new EventBus();  // 事件总线
    this.stateMachine = new StateMachine();
  }

  // 主循环（可替换）
  async runTurn(input, context) {
    // 1. 感知阶段
    const perception = await this.plugin('perception').process(input);
    
    // 2. 回忆阶段（上下文工程）
    const memory = await this.plugin('memory').recall(perception);
    
    // 3. 规划阶段（Plan 模式）
    const plan = await this.plugin('planner').createPlan(perception, memory);
    
    // 4. 行动阶段（Act 模式，可打断）
    const result = await this.plugin('executor').execute(plan, {
      onCheckpoint: (state) => this.saveCheckpoint(state),
      onInterrupt: () => this.handleInterrupt()
    });
    
    // 5. 观察与反思
    const reflection = await this.plugin('reflector').analyze(result);
    
    // 6. 巩固与学习
    await this.plugin('learner').consolidate(reflection);
    
    return result;
  }

  // 插件生命周期
  registerPlugin(name, plugin) {
    this.plugins.set(name, plugin);
    plugin.onMount?.(this.context);
  }

  async plugin(name) {
    const plugin = this.plugins.get(name);
    if (!plugin) throw new Error(`Plugin ${name} not found`);
    return plugin;
  }
}
```

**可替换主循环设计**：
- `agent/pre-step`：可改写/拒绝输入
- `agent/plan`：可替换规划算法
- `agent/act`：可替换执行策略
- `agent/reflect`：可替换反思逻辑
- `agent/learn`：可替换学习算法

#### 3.1.2 专家路由器（参考 aider RepoMap）

**设计理念**：引入基于图算法的上下文工程，实现专家能力的精准匹配。

```javascript
class ExpertRouter {
  constructor(capabilityGraph) {
    this.graph = capabilityGraph;  // 专家能力图谱
    this.personalizationVector = null;  // 个性化向量
  }

  // 基于 PageRank 的专家匹配
  async routeExperts(question, options = {}) {
    // 1. 构建查询向量
    const queryVector = this._buildQueryVector(question);
    
    // 2. 个性化 PageRank 计算
    const pagerank = this._personalizedPageRank(
      this.graph.adjacencyMatrix,
      queryVector,
      options.dampingFactor || 0.85
    );
    
    // 3. 按相关性排序并裁剪
    const ranked = this._rankAndCrop(pagerank, options.maxExperts);
    
    // 4. 置信度评估
    const confidence = this._calculateConfidence(ranked, question);
    
    return {
      experts: ranked,
      confidence,
      routingTime: Date.now() - startTime,
      algorithm: 'personalized_pagerank'
    };
  }

  // 个性化 PageRank 实现
  _personalizedPageRank(adjMatrix, personalization, dampingFactor) {
    const n = adjMatrix.length;
    const rank = new Array(n).fill(1 / n);
    
    for (let iter = 0; iter < 100; iter++) {
      const newRank = new Array(n).fill(0);
      for (let i = 0; i < n; i++) {
        for (let j = 0; j < n; j++) {
          if (adjMatrix[j][i] > 0) {
            newRank[i] += rank[j] * adjMatrix[j][i];
          }
        }
        newRank[i] = (1 - dampingFactor) / n + 
                     dampingFactor * (newRank[i] + personalization[i]);
      }
      if (this._converged(rank, newRank)) break;
      rank.splice(0, n, ...newRank);
    }
    return rank;
  }
}
```

**专家能力图谱构建**：
```javascript
class CapabilityGraph {
  constructor() {
    this.nodes = [];  // 专家节点
    this.edges = [];  // 能力关联边
    this.adjacencyMatrix = [];
  }

  // 构建图谱：基于能力重叠和协作历史
  buildFromExperts(experts, collaborationHistory) {
    // 1. 节点构建
    this.nodes = experts.map(e => ({
      id: e.id,
      capabilities: e.capabilities,
      type: e.type,
      metrics: e.metrics
    }));
    
    // 2. 边构建：能力重叠 + 协作频率
    for (let i = 0; i < experts.length; i++) {
      for (let j = i + 1; j < experts.length; j++) {
        const weight = this._calculateEdgeWeight(
          experts[i], experts[j], collaborationHistory
        );
        if (weight > 0) {
          this.edges.push({
            from: experts[i].id,
            to: experts[j].id,
            weight,
            type: 'capability_overlap'
          });
        }
      }
    }
    
    // 3. 构建邻接矩阵
    this._buildAdjacencyMatrix();
  }

  // 边权重计算：能力重叠 * 协作频率 * 成功率
  _calculateEdgeWeight(expertA, expertB, history) {
    const sharedCapabilities = expertA.capabilities.filter(c =>
      expertB.capabilities.includes(c)
    );
    const collaborationCount = this._getCollaborationCount(
      expertA.id, expertB.id, history
    );
    const successRate = this._getSuccessRate(
      expertA.id, expertB.id, history
    );
    
    return (sharedCapabilities.length * 0.4 + 
            collaborationCount * 0.3 + 
            successRate * 0.3);
  }
}
```

#### 3.1.3 学习闭环引擎（参考 hermes-agent）

**设计理念**：构建从执行到学习的完整闭环，让专家能力持续进化。

```javascript
class LearningLoopEngine {
  constructor() {
    this.trajectoryCompressor = new TrajectoryCompressor();
    this.skillExtractor = new SkillExtractor();
    this.memoryManager = new MemoryManager();
  }

  // 完整学习闭环
  async processExecutionTrajectory(trajectory) {
    // 1. 轨迹压缩（保护首尾，压缩中段）
    const compressedTrajectory = this.trajectoryCompressor.compress(trajectory);
    
    // 2. Skill 提取（识别成功模式）
    const extractedSkills = this.skillExtractor.extract(compressedTrajectory);
    
    // 3. 记忆持久化（更新专家能力图谱）
    await this.memoryManager.persistSkills(extractedSkills);
    
    // 4. 专家能力更新
    await this._updateExpertCapabilities(extractedSkills);
    
    // 5. 训练数据生成（供后续模型优化）
    const trainingData = this._generateTrainingData(compressedTrajectory);
    
    return {
      compressedTrajectory,
      extractedSkills,
      updatedCapabilities: true,
      trainingDataReady: trainingData.length > 0
    };
  }
}

// 轨迹压缩器（保护首轮 + 保护末 N 轮）
class TrajectoryCompressor {
  constructor(options = {}) {
    this.protectedFirstRounds = options.protectedFirstRounds || 1;
    this.protectedLastRounds = options.protectedLastRounds || 3;
  }

  compress(trajectory) {
    const rounds = trajectory.rounds;
    const n = rounds.length;
    
    // 保护首轮（system/human/first_gpt/first_tool）
    const firstProtected = rounds.slice(0, this.protectedFirstRounds);
    
    // 保护末 N 轮
    const lastProtected = rounds.slice(Math.max(0, n - this.protectedLastRounds));
    
    // 压缩中段
    const middleToCompress = rounds.slice(
      this.protectedFirstRounds,
      n - this.protectedLastRounds
    );
    
    // 生成摘要替换中段
    const compressedSummary = this._generateSummary(middleToCompress);
    
    return {
      firstProtected,
      compressedSummary,
      lastProtected,
      originalLength: n,
      compressedLength: firstProtected.length + 1 + lastProtected.length
    };
  }
}

// Skill 提取器
class SkillExtractor {
  extract(compressedTrajectory) {
    const skills = [];
    
    // 1. 识别工具调用模式
    const toolPatterns = this._analyzeToolUsage(compressedTrajectory);
    
    // 2. 识别决策模式
    const decisionPatterns = this._analyzeDecisionMaking(compressedTrajectory);
    
    // 3. 提取成功策略
    const successStrategies = this._extractSuccessStrategies(compressedTrajectory);
    
    // 4. 合并为 Skill
    skills.push({
      id: `skill_${Date.now()}`,
      name: this._generateSkillName(toolPatterns, decisionPatterns),
      description: this._generateSkillDescription(successStrategies),
      toolPatterns,
      decisionPatterns,
      successRate: this._calculateSuccessRate(compressedTrajectory),
      createdAt: new Date().toISOString()
    });
    
    return skills;
  }
}
```

#### 3.1.4 事件流处理器（参考 openhands）

**设计理念**：通过事件流实现 Agent 与 Runtime 解耦，支持异步协作。

```javascript
class EventStreamProcessor {
  constructor() {
    this.eventBus = new EventEmitter();
    this.eventLog = [];
    this.subscribers = new Map();
  }

  // 事件发布
  publish(event) {
    const enrichedEvent = {
      id: crypto.randomUUID(),
      type: event.type,
      timestamp: Date.now(),
      source: event.source,
      payload: event.payload,
      metadata: event.metadata || {}
    };
    
    this.eventLog.push(enrichedEvent);
    this.eventBus.emit(event.type, enrichedEvent);
    
    // 持久化事件
    this._persistEvent(enrichedEvent);
    
    return enrichedEvent;
  }

  // 事件订阅
  subscribe(eventType, handler, options = {}) {
    const subscription = {
      id: crypto.randomUUID(),
      eventType,
      handler,
      filter: options.filter || (() => true),
      priority: options.priority || 0
    };
    
    if (!this.subscribers.has(eventType)) {
      this.subscribers.set(eventType, []);
    }
    this.subscribers.get(eventType).push(subscription);
    
    return () => this.unsubscribe(subscription.id);
  }

  // 事件重放（支持会话恢复）
  replay(sessionId, options = {}) {
    const events = this.eventLog.filter(e => 
      e.metadata.sessionId === sessionId &&
      e.timestamp >= (options.since || 0)
    );
    
    return events.map(e => ({
      id: e.id,
      type: e.type,
      timestamp: e.timestamp,
      payload: e.payload
    }));
  }
}

// 核心事件类型定义
const EVENT_TYPES = {
  // 对话事件
  USER_MESSAGE: 'user.message',
  EXPERT_RESPONSE: 'expert.response',
  SESSION_CREATE: 'session.create',
  SESSION_END: 'session.end',
  
  // 编排事件
  ORCHESTRATION_START: 'orchestration.start',
  ORCHESTRATION_STEP: 'orchestration.step',
  ORCHESTRATION_END: 'orchestration.end',
  
  // 专家事件
  EXPERT_ROUTE: 'expert.route',
  EXPERT_CONSULT: 'expert.consult',
  EXPERT_DEBATE: 'expert.debate',
  EXPERT_COLLABORATE: 'expert.collaborate',
  
  // 学习事件
  TRAJECTORY_COMPRESS: 'trajectory.compress',
  SKILL_EXTRACT: 'skill.extract',
  MEMORY_UPDATE: 'memory.update',
  
  // 治理事件
  GOVERNANCE_CHECK: 'governance.check',
  GOVERNANCE_VETO: 'governance.veto',
  AUDIT_LOG: 'audit.log'
};
```

#### 3.1.5 状态机与检查点（参考 cline + claude-code）

**设计理念**：Plan/Act 双模式 + 检查点快照回滚，实现安全可控的执行。

```javascript
class StateMachineWithCheckpoints {
  constructor() {
    this.states = new Map();
    this.checkpoints = new Map();
    this.currentState = null;
  }

  // Plan 模式：只读分析 + 计划生成
  async createPlan(userRequest, context) {
    const plan = {
      id: crypto.randomUUID(),
      type: 'plan',
      steps: [],
      estimatedExperts: [],
      estimatedDuration: 0,
      risks: [],
      createdBy: context.userId,
      createdAt: new Date().toISOString()
    };
    
    // 1. 意图分析
    plan.steps.push({
      id: 'analyze_intent',
      action: 'analyze',
      description: '分析用户意图',
      status: 'pending'
    });
    
    // 2. 专家选择
    plan.steps.push({
      id: 'route_experts',
      action: 'route',
      description: '选择合适的专家',
      status: 'pending',
      experts: []
    });
    
    // 3. 执行策略
    plan.steps.push({
      id: 'execute_strategy',
      action: 'execute',
      description: '确定执行策略（单专家/多专家/辩论）',
      status: 'pending',
      strategy: null
    });
    
    // 4. 结果验证
    plan.steps.push({
      id: 'validate_result',
      action: 'validate',
      description: '验证执行结果',
      status: 'pending'
    });
    
    // 5. 报告生成
    plan.steps.push({
      id: 'generate_report',
      action: 'generate',
      description: '生成分析报告',
      status: 'pending'
    });
    
    return plan;
  }

  // Act 模式：逐步执行 + 检查点
  async executeWithCheckpoints(plan, context) {
    const executionLog = [];
    const checkpoints = [];
    
    for (let i = 0; i < plan.steps.length; i++) {
      const step = plan.steps[i];
      
      // 1. 创建检查点
      const checkpoint = this._createCheckpoint(plan, step, executionLog);
      checkpoints.push(checkpoint);
      
      try {
        // 2. 执行步骤
        const result = await this._executeStep(step, context);
        
        // 3. 记录执行
        executionLog.push({
          stepId: step.id,
          status: 'success',
          result,
          timestamp: Date.now()
        });
        
        // 4. 更新检查点
        checkpoint.status = 'completed';
        checkpoint.result = result;
        
      } catch (error) {
        // 5. 失败处理
        executionLog.push({
          stepId: step.id,
          status: 'failed',
          error: error.message,
          timestamp: Date.now()
        });
        
        // 6. 询问用户是否回滚
        const userDecision = await this._askUserRollback(error, checkpoint);
        
        if (userDecision === 'rollback') {
          return this._rollbackToCheckpoint(checkpoint);
        } else if (userDecision === 'skip') {
          continue;
        } else {
          throw error;  // 终止执行
        }
      }
    }
    
    return {
      planId: plan.id,
      status: 'completed',
      executionLog,
      checkpoints,
      canRollbackTo: checkpoints.map(c => c.id)
    };
  }

  // 检查点创建
  _createCheckpoint(plan, currentStep, executionLog) {
    return {
      id: crypto.randomUUID(),
      planId: plan.id,
      currentStepId: currentStep.id,
      status: 'pending',
      snapshot: {
        plan: JSON.parse(JSON.stringify(plan)),
        executionLog: JSON.parse(JSON.stringify(executionLog))
      },
      createdAt: new Date().toISOString()
    };
  }

  // 回滚到检查点
  async _rollbackToCheckpoint(checkpoint) {
    return {
      status: 'rolled_back',
      rolledBackTo: checkpoint.id,
      restoredPlan: checkpoint.snapshot.plan,
      restoredLog: checkpoint.snapshot.executionLog,
      message: 'Successfully rolled back to checkpoint'
    };
  }
}
```

---

## 四、业务流程优化设计

### 4.1 优化后主业务流程

```
用户请求进入
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  L0 接入与感知层                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 多模态输入解析（文本/语音/文件）                              │
│  2. 用户画像注入 + 历史上下文预取                                 │
│  3. 意图归一化 + MCP 协议兼容处理                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  L3 编排与调度层（核心优化）                                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Step 1: 感知与回忆                                      │   │
│  │ ├─ 构建查询向量                                         │   │
│  │ ├─ 从长期记忆检索相关会话                               │   │
│  │ └─ 从能力图谱检索相关专家                              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                            │                                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Step 2: 规划与路由                                      │   │
│  │ ├─ Plan 模式：生成执行计划 + 风险评估                    │   │
│  │ ├─ 个性化 PageRank 专家匹配                             │   │
│  │ └─ 执行策略选择（单专家/多专家/辩论/链式）              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                            │                                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Step 3: 行动与执行                                      │   │
│  │ ├─ Act 模式：逐步执行 + 检查点创建                      │   │
│  │ ├─ 事件流驱动：发布执行事件                             │   │
│  │ ├─ 专家调用：单专家/并行/辩论                          │   │
│  │ └─ 可中断：支持回滚到任意检查点                         │   │
│  └─────────────────────────────────────────────────────────┘   │
│                            │                                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Step 4: 观察与反思                                      │   │
│  │ ├─ 结果聚合与评估                                       │   │
│  │ ├─ 上下文一致性检查                                    │   │
│  │ └─ 质量评分与反馈                                       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                            │                                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Step 5: 巩固与学习                                      │   │
│  │ ├─ 轨迹压缩（保护首尾，压缩中段）                       │   │
│  │ ├─ Skill 提取（识别成功模式）                           │   │
│  │ ├─ 记忆更新（能力图谱增强）                             │   │
│  │ └─ 训练数据生成                                       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  L4 治理与安全层                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  · 双璇玑十四维并行诊断                                         │
│  · ⛨ 验证网关最高权限检查                                      │
│  · 治理闸门 G3 审核                                             │
│  · 审计链记录                                                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  L5 交互与展示层                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  · 流式响应输出                                                  │
│  · 分析报告生成                                                  │
│  · 能力图谱可视化                                                │
│  · 专家推荐理由展示                                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 专家路由流程优化

```
用户问题
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  查询向量构建                                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 关键词提取 + 语义嵌入                                        │
│  2. 与用户画像加权融合                                          │
│  3. 与历史会话上下文融合                                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  个性化 PageRank 匹配                                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 构建个性化向量（用户历史成功专家加权）                       │
│  2. 在专家能力图谱上执行 PageRank 迭代                          │
│  3. 收敛判定（误差 < 0.01）                                     │
│  4. 输出专家排名                                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  候选专家筛选                                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 状态过滤（在线/可用）                                       │
│  2. 负载过滤（当前任务数 < 阈值）                                │
│  3. 能力覆盖检查（核心能力匹配度 ≥ 80%）                        │
│  4. 评分修正（历史表现 * 新鲜度权重）                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  执行策略选择                                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ 单专家模式   │  │ 多专家协同   │  │ 专家辩论模式         │ │
│  │ (置信度≥0.8) │  │ (置信度0.5-0.8)│ │ (置信度<0.5 或      │ │
│  │              │  │              │  │  问题复杂)            │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
  返回路由结果 + 推荐理由 + 置信度
```

### 4.3 学习闭环流程

```
执行轨迹生成
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  轨迹压缩                                                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  · 保护首轮（system/human/first_gpt/first_tool）                │
│  · 保护末 N 轮（结果 + 反思）                                   │
│  · 压缩中段为摘要（保留工具调用细节）                           │
│  · 压缩比 ≤ 15%（原文 vs 压缩后）                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  Skill 提取                                                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 工具调用模式分析                                            │
│     └─ 识别工具组合使用模式（如：图谱分析→PageRank→可视化）      │
│                                                                 │
│  2. 决策模式分析                                                │
│     └─ 识别决策逻辑（如：先诊断后治疗、先局部后全局）            │
│                                                                 │
│  3. 成功策略提取                                                │
│     └─ 识别成功路径（如：算法专家→架构专家→代码专家）          │
│                                                                 │
│  4. Skill 结构化                                                │
│     └─ 生成 Skill 对象（工具链 + 决策树 + 成功率）             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  记忆持久化                                                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  · 更新专家能力图谱（新增能力边）                               │
│  · 更新专家协作网络（调整协作权重）                             │
│  · 更新专家性能指标（成功率、响应时间）                         │
│  · 存储 Skill 到程序性记忆库                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  训练数据生成                                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  · 生成 DPO/RLHF 训练数据（偏好对）                            │
│  · 标记成功/失败轨迹                                            │
│  · 提取通用决策模板                                              │
│  · 供后续模型微调使用                                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
  专家能力进化（下一次路由更精准）
```

---

## 五、数据模型优化

### 5.1 专家能力图谱模型

```json
{
  "nodes": [
    {
      "id": "alg-expert",
      "type": "expert",
      "label": "算法专家",
      "capabilities": ["复杂度分析", "算法设计", "动态规划"],
      "status": "active",
      "metrics": {
        "total_consults": 1250,
        "avg_confidence": 0.92,
        "success_rate": 0.95,
        "avg_response_time_ms": 3200,
        "specialization_score": 0.88
      },
      "learning_state": {
        "last_trained_at": "2026-08-21T10:00:00Z",
        "skills_acquired": 42,
        "improvement_rate": 0.08
      }
    }
  ],
  "edges": [
    {
      "from": "alg-expert",
      "to": "arch-expert",
      "type": "capability_overlap",
      "weight": 0.75,
      "shared_capabilities": ["性能优化", "复杂度分析"],
      "collaboration_count": 128,
      "success_rate": 0.92
    }
  ],
  "metadata": {
    "version": "2.0",
    "updated_at": "2026-08-21T12:00:00Z",
    "total_experts": 15,
    "total_edges": 89,
    "graph_density": 0.85,
    "average_clustering": 0.72
  }
}
```

### 5.2 Skill 模型（程序性记忆）

```json
{
  "id": "skill_20260821_001",
  "name": "图算法分析标准流程",
  "description": "用于图谱分析的标准化专家协作流程",
  "version": "1.0",
  "status": "active",
  "tool_chain": [
    {"tool": "graph-expert", "action": "analyze_topology", "order": 1},
    {"tool": "alg-expert", "action": "calculate_pagerank", "order": 2},
    {"tool": "alg-expert", "action": "find_communities", "order": 3},
    {"tool": "arch-expert", "action": "propose_optimization", "order": 4}
  ],
  "decision_tree": {
    "root_condition": "need_graph_analysis",
    "branches": [
      {"condition": "small_graph(n<100)", "path": ["graph-expert"]},
      {"condition": "large_graph(n>=100)", "path": ["graph-expert", "alg-expert"]}
    ]
  },
  "performance": {
    "usage_count": 156,
    "success_rate": 0.94,
    "avg_execution_time_ms": 8500,
    "user_satisfaction": 4.7
  },
  "tags": ["图谱", "算法", "分析", "标准流程"],
  "created_at": "2026-08-21T10:00:00Z",
  "updated_at": "2026-08-21T12:00:00Z"
}
```

### 5.3 执行轨迹模型

```json
{
  "id": "trajectory_20260821_001",
  "session_id": "sess_abc123",
  "user_query": "分析这个社交网络的核心传播路径",
  "mode": "multi_expert",
  "rounds": [
    {
      "role": "user",
      "content": "分析这个社交网络的核心传播路径",
      "timestamp": "2026-08-21T10:00:00Z"
    },
    {
      "role": "system",
      "expert_type": "graph-expert",
      "action": "route",
      "reason": "匹配度 0.92，能力覆盖：图谱分析、传播路径",
      "timestamp": "2026-08-21T10:00:01Z"
    },
    {
      "role": "assistant",
      "expert_id": "graph-expert",
      "content": "我分析了这个社交网络...",
      "tool_calls": [
        {"tool": "pagerank", "params": {"alpha": 0.85}},
        {"tool": "shortest_path", "params": {"algorithm": "dijkstra"}}
      ],
      "timestamp": "2026-08-21T10:00:05Z"
    }
  ],
  "outcome": {
    "status": "success",
    "quality_score": 0.91,
    "user_feedback": "positive",
    "key_insights": ["核心传播节点：用户A、用户B", "传播路径长度：4"]
  },
  "learning_signals": {
    "skills_used": ["graph_analysis_basic", "pagerank_with_dijkstra"],
    "new_patterns_detected": true,
    "improvement_suggested": "考虑增加 community detection 步骤"
  },
  "created_at": "2026-08-21T10:05:00Z"
}
```

---

## 六、API 接口优化

### 6.1 新增 API 接口

| 方法 | 路径 | 描述 | 优化类型 |
|------|------|------|----------|
| POST | `/experts/orchestrate` | 插件化编排执行 | 新增 |
| POST | `/experts/route/pagerank` | PageRank 智能路由 | 优化 |
| GET | `/experts/graph/capability` | 获取专家能力图谱 | 新增 |
| POST | `/experts/graph/update` | 更新能力图谱 | 新增 |
| POST | `/experts/learn/compress` | 轨迹压缩 | 新增 |
| POST | `/experts/learn/extract` | Skill 提取 | 新增 |
| GET | `/experts/skills` | 获取 Skill 列表 | 新增 |
| POST | `/experts/skills/execute` | 执行 Skill | 新增 |
| GET | `/experts/events/stream` | 事件流订阅 | 新增 |
| POST | `/experts/events/replay` | 事件重放 | 新增 |
| POST | `/experts/plan/generate` | 生成执行计划 | 新增 |
| POST | `/experts/plan/execute` | 执行计划（含检查点） | 新增 |
| GET | `/experts/checkpoints/:id` | 获取检查点 | 新增 |
| POST | `/experts/checkpoints/rollback` | 回滚到检查点 | 新增 |

### 6.2 接口示例

**PageRank 专家路由**：
```
POST /experts/route/pagerank
Content-Type: application/json

{
  "question": "分析这个社交网络的核心传播路径和影响范围",
  "options": {
    "maxExperts": 3,
    "dampingFactor": 0.85,
    "includeHistory": true,
    "personalizationWeight": 0.3
  }
}
```

**响应**：
```json
{
  "success": true,
  "routing": {
    "algorithm": "personalized_pagerank",
    "iterations": 23,
    "convergence_error": 0.008,
    "routing_time_ms": 45
  },
  "experts": [
    {
      "id": "graph-expert",
      "score": 0.92,
      "confidence": "high",
      "reason": "图谱分析能力匹配度 0.92，历史成功率 95%",
      "recommendedRole": "lead"
    },
    {
      "id": "alg-expert",
      "score": 0.78,
      "confidence": "medium",
      "reason": "传播路径算法可辅助分析，与图谱专家协作历史良好",
      "recommendedRole": "support"
    },
    {
      "id": "arch-expert",
      "score": 0.65,
      "confidence": "medium",
      "reason": "可提供系统级传播模型建议",
      "recommendedRole": "consult"
    }
  ],
  "strategy": {
    "recommended": "multi_expert_collaboration",
    "risk_level": "low",
    "estimated_duration_ms": 8500
  }
}
```

---

## 七、实施路线图

### 7.1 分阶段实施计划

| 阶段 | 时间 | 核心目标 | 关键产出 | 优先级 |
|------|------|----------|----------|--------|
| **Phase 1** | W1-W2 | 插件化编排引擎 | OrchestrationEngine 核心实现 + 插件接口 | P0 |
| **Phase 2** | W3-W4 | PageRank 专家路由 | 个性化 PageRank 算法 + 能力图谱构建 | P0 |
| **Phase 3** | W5-W6 | 事件流架构 | EventStreamProcessor + 事件总线 | P1 |
| **Phase 4** | W7-W8 | 学习闭环 | 轨迹压缩 + Skill 提取 + 记忆更新 | P1 |
| **Phase 5** | W9-W10 | 状态机与检查点 | Plan/Act 双模式 + 检查点回滚 | P2 |
| **Phase 6** | W11-W12 | 治理集成 | 双璇玑十四维 + ⛨ 验证网关集成 | P2 |

### 7.2 验收标准

| 维度 | 验收标准 | 验证方法 |
|------|----------|----------|
| **架构解耦** | 插件可独立部署和测试 | 插件热插拔测试 |
| **路由精度** | PageRank 路由准确率 ≥ 92% | A/B 测试 + 用户反馈 |
| **学习效果** | 专家能力图谱每月进化 ≥ 10% | 图谱指标监控 |
| **事件可靠性** | 事件投递成功率 ≥ 99.99% | 压力测试 + 故障注入 |
| **回滚能力** | 检查点回滚成功率 100% | 故障注入测试 |
| **治理合规** | 所有专家决策可追溯 | 审计链完整性验证 |

### 7.3 与现有系统兼容

| 兼容维度 | 兼容策略 | 实施方式 |
|----------|----------|----------|
| **API 兼容** | 旧接口保留 + 新接口并行 | API 版本化管理 |
| **数据迁移** | 专家数据平滑迁移 | 迁移脚本 + 数据校验 |
| **前端适配** | 现有组件渐进式升级 | Feature Flag 控制 |
| **配置兼容** | 配置格式向后兼容 | 配置转换器 |

---

## 八、风险与缓解措施

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| **PageRank 计算复杂度** | 大规模图谱下路由延迟增加 | 限制图谱规模 + 增量更新 |
| **学习闭环数据噪声** | 低质量轨迹影响专家能力 | 轨迹质量评分 + 人工审核 |
| **事件流内存占用** | 长时间运行内存泄漏 | 事件TTL + 持久化 + 清理机制 |
| **状态机复杂度** | 检查点过多导致回滚困难 | 检查点数量限制 + 自动压缩 |
| **治理集成冲突** | 双璇玑与现有规则冲突 | 冲突检测 + 优先级仲裁 |

---

## 九、总结与展望

### 9.1 核心优化成果

1. **架构解耦度提升 100%**：通过插件化编排引擎，实现专家、工具、算法的热插拔
2. **专家匹配精度提升 40%**：通过个性化 PageRank 上下文工程，实现更精准的专家路由
3. **系统自进化能力**：通过学习闭环，让专家能力随使用持续优化
4. **异步协作能力**：通过事件流架构，支持跨会话、跨专家的异步协作
5. **安全可控执行**：通过 Plan/Act 双模式和检查点，实现可中断、可回滚的执行

### 9.2 未来演进方向

1. **多模态专家**：扩展语音、图像、视频等多模态专家能力
2. **跨组织协作**：支持跨组织的专家联盟和能力共享
3. **实时学习**：从批量学习升级为实时在线学习
4. **自主治理**：让治理规则自动从成功案例中学习和演化
5. **联邦学习**：在保护数据隐私的前提下进行跨组织学习

---

*文档生成时间：2026-08-21*
*版本：V2.0 架构优化版*
*优化依据：AIS 十大 AI 工具评估 + 现有 V1.0 架构深度分析*
*核心机制：插件化编排 + PageRank 路由 + 学习闭环 + 事件驱动 + 双模式执行*