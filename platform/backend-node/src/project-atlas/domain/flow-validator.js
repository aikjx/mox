'use strict';

/**
 * EAF-STD-001 流程建模不变式校验（domain 层 · 纯函数零 IO）
 * ------------------------------------------------------------------
 * 任何模块按 EAF-STD-001 标准向通用 AI 知识图谱注册业务流程前，
 * 必须通过本校验（对应标准 §3.3 建模不变式 + §5 W9 检查族）：
 *
 *   V1 流程身份：id 非空、格式合法、不与既有流程冲突（除非覆盖语义）
 *   V2 归属唯一：domain 必须指向图谱真实业务域（§3.3-1）
 *   V3 步骤结构：steps ≥3、id 流程内唯一、每步有名称（§3.3-3）
 *   V4 迁移有效：transitions 引用的步骤必须存在（§3.3-4 前置）
 *   V5 委托真实：每步 engine（若声明）必须指向真实引擎（§3.3-2）
 *   V6 数据注册：reads/writes 必须指向已注册数据资产（§3.3-5）
 *   V7 连通可达：存在入口或为闭环，入口/锚点可达全部步骤（§3.3-4）
 *   V8 迁移类型：type ∈ {next, degrade}
 *
 * 输出：{ valid, errors: [{rule, message}] }——拒绝时逐条指名，不静默放行。
 */

const FLOW_ID_RE = /^[a-z][a-z0-9-]*$/;

/**
 * 校验单条流程定义。
 * @param {object} flow 待注册流程（{id,name,domain,steps[],transitions[]}）
 * @param {object} ctx 已登记视图上下文 {
 *   domainIds: Set<string>,        // 合法业务域 id（含 auto 层）
 *   engineIds: Set<string>,        // 合法引擎 id
 *   dataFiles: Set<string>,        // 已注册数据资产文件名
 *   flowIds: Set<string>,          // 既有流程 id（冲突检测）
 *   allowOverwrite: boolean        // 同 id 覆盖语义（默认拒绝）
 * }
 */
function validateFlow(flow, ctx) {
  const errors = [];
  const err = (rule, message) => errors.push({ rule, message });
  const { domainIds, engineIds, dataFiles, flowIds, allowOverwrite = false } = ctx || {};

  // V1 流程身份
  if (!flow || typeof flow !== 'object') {
    err('V1', '流程必须为对象');
    return { valid: false, errors };
  }
  if (!flow.id || typeof flow.id !== 'string') err('V1', '流程 id 为必填字符串');
  else if (!FLOW_ID_RE.test(flow.id)) err('V1', `流程 id 格式非法（须匹配 ${FLOW_ID_RE}）: ${flow.id}`);
  else if (flowIds && flowIds.has(flow.id) && !allowOverwrite) err('V1', `流程 id 已存在: ${flow.id}`);
  if (!flow.name || typeof flow.name !== 'string') err('V1', '流程 name 为必填字符串');

  // V2 归属唯一
  if (!flow.domain) err('V2', '流程 domain 为必填');
  else if (domainIds && !domainIds.has(flow.domain)) err('V2', `归属域不存在于图谱: ${flow.domain}`);

  // V3 步骤结构
  const steps = Array.isArray(flow.steps) ? flow.steps : null;
  if (!steps) err('V3', 'steps 必须为数组');
  else {
    if (steps.length < 3) err('V3', `步骤数不足（${steps.length} < 3）`);
    const stepIds = new Set();
    steps.forEach((s, i) => {
      if (!s || !s.id || typeof s.id !== 'string') err('V3', `第 ${i} 步缺少 id`);
      else if (stepIds.has(s.id)) err('V3', `步骤 id 重复: ${s.id}`);
      else stepIds.add(s.id);
      if (!s.name) err('V3', `步骤 ${s.id || i} 缺少 name`);
    });

    // V4 迁移有效 + V8 迁移类型
    const transitions = Array.isArray(flow.transitions) ? flow.transitions : [];
    if (steps.length >= 3 && transitions.length < steps.length - 1) {
      err('V4', `迁移边不足（${transitions.length} < 步骤数-1=${steps.length - 1}）`);
    }
    transitions.forEach((t, i) => {
      if (!t || !t.from || !t.to) err('V4', `第 ${i} 条迁移边缺少 from/to`);
      else {
        if (!stepIds.has(t.from)) err('V4', `迁移边 from 步骤不存在: ${t.from}`);
        if (!stepIds.has(t.to)) err('V4', `迁移边 to 步骤不存在: ${t.to}`);
        if (t.type !== 'next' && t.type !== 'degrade') {
          err('V8', `迁移边 type 非法（须 next|degrade）: ${t.type}`);
        }
      }
    });

    // V5 委托真实
    if (engineIds) {
      steps.forEach(s => {
        if (s && s.engine && !engineIds.has(s.engine)) {
          err('V5', `步骤 ${s.id} 委托引擎不存在: ${s.engine}`);
        }
      });
    }

    // V6 数据注册
    if (dataFiles) {
      steps.forEach(s => {
        if (!s) return;
        [...(s.reads || []), ...(s.writes || [])].forEach(f => {
          if (!dataFiles.has(f)) err('V6', `步骤 ${s.id} 数据依赖未注册: ${f}`);
        });
      });
    }

    // V7 连通可达（入口或闭环锚点 BFS 全可达）
    if (stepIds.size > 0 && transitions.every(t => t && t.from && t.to)) {
      const inDeg = Object.fromEntries([...stepIds].map(id => [id, 0]));
      transitions.forEach(t => { if (stepIds.has(t.to)) inDeg[t.to]++; });
      const entries = [...stepIds].filter(id => inDeg[id] === 0);
      const isLoop = entries.length === 0 && steps.length > 0;
      const starts = isLoop ? [steps[0].id] : entries;
      const next = Object.fromEntries([...stepIds].map(id => [id, []]));
      transitions.forEach(t => { if (stepIds.has(t.from) && stepIds.has(t.to)) next[t.from].push(t.to); });
      const seen = new Set(starts); const q = [...starts];
      while (q.length) {
        const c = q.shift();
        for (const n of next[c]) if (!seen.has(n)) { seen.add(n); q.push(n); }
      }
      const unreachable = [...stepIds].filter(id => !seen.has(id));
      if (unreachable.length > 0) {
        err('V7', `${isLoop ? '闭环锚点' : '入口'}=[${starts.join(',')}] 不可达步骤: ${unreachable.join(',')}`);
      }
    }
  }

  return { valid: errors.length === 0, errors };
}

module.exports = { validateFlow };
