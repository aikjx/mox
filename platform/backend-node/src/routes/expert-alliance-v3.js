'use strict';

/**
 * 路由域：专家联盟 v3（7 服务架构 · M1 桩接）
 *
 * 路径前缀：/expert/v3/*
 *
 * M1 交付范围（D1-D3）：
 *   1. 任务创建与查询（内存存储，M3 外部化到 PG）
 *   2. 调度全流程：意图识别 → 专家匹配 → 计划生成（端到端 ≤150ms）
 *   3. 专家列表与能力矩阵（7 服务视图）
 *   4. 健康检查
 *
 * 后续 M2-M4 将逐步替换为 gRPC 调用 7 服务：
 *   scheduler / executor / fusion / registry / memory / agent / gateway
 *
 * 与 v2 路由（/experts/*）的区别：
 *   v2 = 单体 alliance 引擎直接调用
 *   v3 = 7 服务架构 API 契约（当前 M1 为内存桩，后续逐步接 gRPC）
 */

// ================== 内存任务存储（M1 桩，M3 替换为 PG） ==================
const tasks = new Map(); // task_id → Task
const taskIdCounter = { n: 0 };

function genTaskId() {
  taskIdCounter.n += 1;
  return 'task-' + Date.now().toString(36) + '-' + taskIdCounter.n.toString(36);
}

function nowIso() {
  return new Date().toISOString();
}

// ================== 7 服务架构元数据 ==================
const V3_SERVICES = [
  { id: 'gateway-http', name: 'HTTP 网关', port: 8080, status: 'planned', desc: 'REST/JSON-RPC/MCP/WS 接入 + 认证/限流/转码' },
  { id: 'gateway-grpc', name: 'gRPC 网关', port: 50051, status: 'planned', desc: '内部 gRPC + 服务间路由/负载均衡' },
  { id: 'alliance-scheduler', name: '调度服务', port: 50052, status: 'm1-stub', desc: '任务调度/专家匹配(合并图谱)/计划生成/案例检索' },
  { id: 'alliance-executor', name: '执行服务', port: 50053, status: 'planned', desc: 'DAG 执行引擎/节点调度/进度推送/人工干预' },
  { id: 'alliance-fusion', name: '融合服务', port: 50054, status: 'planned', desc: '6 种融合策略/质量评估/迭代精炼' },
  { id: 'expert-registry', name: '专家注册服务', port: 50055, status: 'planned', desc: '专家 CRUD/定义验证/健康检查/工具自动发现' },
  { id: 'expert-memory', name: '记忆服务', port: 50056, status: 'planned', desc: '统一记忆抽象/案例库/图谱学习/边权重更新' },
  { id: 'expert-agent', name: 'Agent 运行时', port: 50057, status: 'planned', desc: 'ReAct 循环/工具调用/AI 推理（无状态）' },
];

// 6 种协作模式
const COLLABORATION_MODES = [
  { id: 'parallel', name: '并行咨询', desc: '多专家同时独立分析' },
  { id: 'debate', name: '辩论对抗', desc: '专家间正反方辩论' },
  { id: 'hierarchical', name: '分层审批', desc: '初级分析→高级审核' },
  { id: 'pipeline', name: '流水线接力', desc: '专家按序传递处理' },
  { id: 'consensus', name: '共识投票', desc: '多轮投票达成共识' },
  { id: 'majority_vote', name: '多数投票', desc: '简单多数决' },
];

// 6 种融合策略
const FUSION_STRATEGIES = [
  { id: 'weighted_vote', name: '加权投票', desc: '按专家权重加权求和（默认）' },
  { id: 'majority_vote', name: '多数投票', desc: '简单多数决' },
  { id: 'concatenate', name: '拼接合并', desc: '多专家结果拼接整合' },
  { id: 'best_pick', name: '择优选择', desc: '选质量最高的单专家结果' },
  { id: 'debate_arbitration', name: '辩论仲裁', desc: '第三方仲裁辩论结果' },
  { id: 'llm_synthesis', name: 'LLM 合成', desc: '大模型综合多专家意见' },
];

// ================== 路由注册 ==================
module.exports = function registerExpertAllianceV3Routes(ctx) {
  const { url, alliance, ok, fail, readBody, reg, getAllianceEngine, uid } = ctx;

  // ===== 1. 创建任务 =====
  reg('post', '/expert/v3/tasks', async (req, res) => {
    const body = await readBody(req);
    const query = String(body.query || body.question || body.message || '').trim();
    if (!query) return fail(res, 400, 'query 为必填（query/question/message），不能为空');

    const taskId = genTaskId();
    const task = {
      task_id: taskId,
      query,
      session_id: body.session_id || null,
      idempotency_key: body.idempotency_key || null,
      status: 'PENDING',
      created_at: nowIso(),
      updated_at: nowIso(),
      context: body.context || {},
      options: {
        enable_llm_debate: body.options?.enable_llm_debate || false,
        retry_on_c: body.options?.retry_on_c !== false,
        team_size: Math.max(2, Math.min(7, body.options?.team_size || 4)),
        enable_spread: body.options?.enable_spread !== false,
        collaboration_mode: body.options?.collaboration_mode || 'parallel',
        fusion_strategy: body.options?.fusion_strategy || 'weighted_vote',
        max_iterations: body.options?.max_iterations || 3,
        timeout_seconds: body.options?.timeout_seconds || 300,
        sensitive: body.options?.sensitive || false,
      },
      tenant_id: body.tenant_id || 'default',
      created_by: body.created_by || 'api',
      progress_percent: 0,
      error_message: null,
      // v3 扩展字段（调度后填充）
      intent: null,
      team: null,
      plan: null,
      scheduling_latency_ms: null,
      graph_used: false,
    };

    tasks.set(taskId, task);
    ok(res, task);
  });

  // ===== 2. 查询任务 =====
  reg('get', '/expert/v3/tasks/:id', (req, res, params) => {
    const task = tasks.get(params.id);
    if (!task) return fail(res, 404, 'Task not found: ' + params.id);
    ok(res, task);
  });

  // ===== 3. 列出任务（分页） =====
  reg('get', '/expert/v3/tasks', (req, res) => {
    const q = url.parse(req.url, true).query;
    const pageSize = Math.max(1, Math.min(100, parseInt(q.page_size || '20', 10) || 20));
    const pageToken = q.page_token || '';
    const statusFilter = q.status || '';

    let list = Array.from(tasks.values()).sort((a, b) => b.created_at.localeCompare(a.created_at));
    if (statusFilter) list = list.filter(t => t.status === statusFilter);

    // 简单分页（基于 offset，M3 替换为 cursor）
    const offset = pageToken ? parseInt(Buffer.from(pageToken, 'base64').toString() || '0', 10) : 0;
    const page = list.slice(offset, offset + pageSize);
    const nextOffset = offset + pageSize;
    const nextPageToken = nextOffset < list.length ? Buffer.from(String(nextOffset)).toString('base64') : '';

    ok(res, {
      tasks: page,
      page: { next_page_token: nextPageToken, total_count: list.length },
    });
  });

  // ===== 4. 调度任务（M1 核心验收：意图识别→专家匹配→计划生成，≤150ms） =====
  reg('post', '/expert/v3/tasks/:id/schedule', async (req, res, params) => {
    const start = Date.now();
    const task = tasks.get(params.id);
    if (!task) return fail(res, 404, 'Task not found: ' + params.id);
    if (task.status === 'EXECUTING' || task.status === 'COMPLETED') {
      return fail(res, 409, 'Task already in status: ' + task.status);
    }

    const body = await readBody(req).catch(() => ({}));
    const previewOnly = body.preview_only !== false; // 默认预览模式（不启动执行）

    try {
      const engine = getAllianceEngine();

      // --- Step 1: 意图识别（7 类 RRF 融合）---
      const intent = engine.classifyIntent(task.query);

      // --- Step 2: 专家匹配（基于意图 + 注册表）---
      const team = engine.composeTeam(task.query, intent, { teamSize: task.options.team_size });

      // --- Step 3: 生成执行计划（DAG 节点）---
      const plan = generatePlan(task, intent, team);

      // --- 更新任务状态 ---
      const latency = Date.now() - start;
      task.status = previewOnly ? 'SCHEDULED' : 'EXECUTING';
      task.intent = intent;
      task.team = team;
      task.plan = plan;
      task.scheduling_latency_ms = latency;
      task.graph_used = !intent.degraded;
      task.progress_percent = previewOnly ? 30 : 40;
      task.updated_at = nowIso();
      tasks.set(task.task_id, task);

      ok(res, {
        task: { task_id: task.task_id, status: task.status, progress_percent: task.progress_percent },
        intent: {
          intent_id: intent.intent_id,
          confidence: intent.conf,
          degraded: intent.degraded,
          rrf_scores: intent.rrf_scores,
        },
        team: {
          team_ids: team.team_ids || team.experts?.map(e => e.id) || [],
          forced_replacements: team.forced_replacements || [],
        },
        plan,
        scheduling_latency_ms: latency,
        graph_used: task.graph_used,
        preview_only: previewOnly,
        // 验收指标标记
        acceptance: {
          target_latency_ms: 150,
          actual_latency_ms: latency,
          passed: latency <= 150,
        },
      });
    } catch (e) {
      task.status = 'FAILED';
      task.error_message = e.message;
      task.updated_at = nowIso();
      tasks.set(task.task_id, task);
      fail(res, 500, 'Schedule failed: ' + e.message);
    }
  });

  // ===== 5. 仅匹配专家（快速接口） =====
  reg('post', '/expert/v3/match', async (req, res) => {
    const body = await readBody(req);
    const query = String(body.query || body.question || '').trim();
    if (!query) return fail(res, 400, 'query 为必填');

    try {
      const engine = getAllianceEngine();
      const intent = engine.classifyIntent(query);
      const team = engine.composeTeam(query, intent, { teamSize: body.team_size || 4 });
      ok(res, {
        intent: { intent_id: intent.intent_id, confidence: intent.conf, degraded: intent.degraded },
        team: {
          team_ids: team.team_ids || team.experts?.map(e => e.id) || [],
          forced_replacements: team.forced_replacements || [],
          reasoning_matrix: team.reasoning_matrix || {},
        },
      });
    } catch (e) {
      fail(res, 500, 'Match failed: ' + e.message);
    }
  });

  // ===== 6. 列出专家（v3 注册表视图） =====
  reg('get', '/expert/v3/experts', (req, res) => {
    const q = url.parse(req.url, true).query;
    const experts = alliance.listExperts({
      type: q.type,
      status: q.status,
      keyword: q.q,
    });
    ok(res, {
      experts,
      total: experts.length,
      registry_source: 'v2-alliance (M1 stub, M2 migrates to expert-registry service)',
    });
  });

  // ===== 7. 能力矩阵（7 服务 × 能力） =====
  reg('get', '/expert/v3/capabilities', (req, res) => {
    ok(res, {
      version: 'v3-M1',
      architecture: '7-service (gateway-http + gateway-grpc + scheduler + executor + fusion + registry + memory + agent)',
      services: V3_SERVICES,
      collaboration_modes: COLLABORATION_MODES,
      fusion_strategies: FUSION_STRATEGIES,
      intent_classes: ['math', 'logic', 'knowledge', 'code', 'chinese', 'timeliness', 'instruction'],
      dimensions: 14,
      current_milestone: 'M1 (stub + scheduler inline)',
      next_milestone: 'M2 (DAG executor + ReAct agent + 6 fusion strategies)',
    });
  });

  // ===== 8. 健康检查 =====
  reg('get', '/expert/v3/health', (req, res) => {
    const healthy = typeof alliance.listExperts === 'function';
    ok(res, {
      service: 'expert-alliance-v3',
      version: 'v3-M1',
      healthy,
      status: healthy ? 'ok' : 'degraded',
      tasks_total: tasks.size,
      services: V3_SERVICES.map(s => ({ id: s.id, status: s.status })),
      uptime_seconds: process.uptime() | 0,
      timestamp: nowIso(),
    });
  });

  // ===== 9. 取消任务 =====
  reg('post', '/expert/v3/tasks/:id/cancel', (req, res, params) => {
    const task = tasks.get(params.id);
    if (!task) return fail(res, 404, 'Task not found: ' + params.id);
    if (task.status === 'COMPLETED' || task.status === 'CANCELLED') {
      return fail(res, 409, 'Task already in terminal status: ' + task.status);
    }
    task.status = 'CANCELLED';
    task.updated_at = nowIso();
    tasks.set(task.task_id, task);
    ok(res, { cancelled: true, task: { task_id: task.task_id, status: task.status } });
  });
};

// ================== 计划生成器（M1 桩，M2 升级为 DAG 拓扑调度） ==================

/**
 * 生成执行计划（DAG 节点 + 依赖关系）
 *
 * M1 实现：基于 6 阶段管线生成线性 DAG（Intent→Team→Debate→Synthesize→Gate→Learn）
 * M2 升级：支持并行/条件/循环节点的完整 DAG 拓扑调度
 */
function generatePlan(task, intent, team) {
  const expertIds = team.team_ids || team.experts?.map(e => e.id) || [];
  const planId = 'plan-' + Date.now().toString(36);
  const stages = [];

  // Stage 1: Intent（已完成，标记为 completed）
  stages.push({
    stage_id: 'stage-intent',
    name: '意图识别',
    phase: 'INTENT',
    expert_id: null,
    depends_on: [],
    estimated_ms: 20,
    config: { intent_id: intent.intent_id, confidence: intent.conf },
  });

  // Stage 2: Team（已完成）
  stages.push({
    stage_id: 'stage-team',
    name: '专家组队',
    phase: 'TEAM',
    expert_id: null,
    depends_on: ['stage-intent'],
    estimated_ms: 30,
    config: { team_size: expertIds.length, experts: expertIds },
  });

  // Stage 3: Debate（并行咨询，每个专家一个子节点）
  const debateNodeIds = [];
  for (let i = 0; i < expertIds.length; i++) {
    const nid = 'stage-debate-' + i;
    debateNodeIds.push(nid);
    stages.push({
      stage_id: nid,
      name: '专家咨询: ' + expertIds[i],
      phase: 'DEBATE',
      expert_id: expertIds[i],
      depends_on: ['stage-team'],
      estimated_ms: 100 + i * 10,
      config: { collaboration_mode: task.options.collaboration_mode },
    });
  }

  // Stage 4: Synthesize（融合，依赖所有 debate 节点）
  stages.push({
    stage_id: 'stage-synthesize',
    name: '结果融合',
    phase: 'SYNTHESIZE',
    expert_id: null,
    depends_on: debateNodeIds,
    estimated_ms: 50,
    config: { fusion_strategy: task.options.fusion_strategy },
  });

  // Stage 5: Gate（质量门禁）
  stages.push({
    stage_id: 'stage-gate',
    name: '质量门禁',
    phase: 'GATE',
    expert_id: null,
    depends_on: ['stage-synthesize'],
    estimated_ms: 15,
    config: { retry_on_c: task.options.retry_on_c },
  });

  // Stage 6: Learn（指标学习）
  stages.push({
    stage_id: 'stage-learn',
    name: '指标学习',
    phase: 'LEARN',
    expert_id: null,
    depends_on: ['stage-gate'],
    estimated_ms: 25,
    config: { update_graph_weights: true },
  });

  const totalEstimated = stages.reduce((sum, s) => sum + s.estimated_ms, 0);

  return {
    task_id: task.task_id,
    plan_id: planId,
    expert_ids: expertIds,
    stages,
    estimated_total_ms: totalEstimated,
    collaboration_mode: task.options.collaboration_mode,
    fusion_strategy: task.options.fusion_strategy,
    generated_at: nowIso(),
    dag_summary: {
      total_nodes: stages.length,
      parallel_nodes: debateNodeIds.length,
      critical_path: ['stage-intent', 'stage-team', ...debateNodeIds.slice(0, 1), 'stage-synthesize', 'stage-gate', 'stage-learn'],
    },
  };
}
