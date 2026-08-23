'use strict';

/**
 * 项目全息图谱 · 需求归一化规则（domain 层 · 纯函数 · 零 IO）
 * ------------------------------------------------------------------
 * "业务流程与架构模块维度" 核心规则集，覆盖全流程归一化：
 *   需求归一化 IR 构建（ingest）→ 需求拆解（decompose）
 *   → 架构域映射（map）→ 模块拆分（split）→ 算法关联（bind）
 *   → 变更传播计划（propagate）
 * 闭环链：需求条目 ↔ 业务域 ↔ 引擎 ↔ 算法 ↔ 数据资产 ↔ 开发规范
 */

const { extractKeywords } = require('../../kb/domain/entity-extractor');

// ============ N1 需求归一化 IR ============

const CATEGORIES = {
  feature: ['新增', '开发', '实现', '支持', '增加', '提供', '上线'],
  bugfix: ['修复', '缺陷', 'bug', '故障', '异常', '问题修复'],
  optimization: ['优化', '性能', '提升', '加速', '降低延迟', '压缩'],
  architecture: ['架构', '重构', '拆分', '分层', '模块化', '迁移', '归一化']
};

/**
 * N1 需求归一化 IR：原始需求文本 → 结构化中间表示
 * { id, title, raw, category, keywords[], createdAt }
 * category 未显式提供时按关键词推断（优先 architecture > bugfix > optimization > feature > general）
 */
function buildRequirementIR({ title, content, category, id, now }) {
  const raw = String(content || '').trim();
  const titleText = String(title || '').trim() || (raw.split(/[。\n]/)[0] || '未命名需求').slice(0, 60);
  const keywords = extractKeywords(`${titleText} ${raw}`, 16);
  return {
    id: id || `req-${Date.now()}`,
    title: titleText,
    raw,
    category: normalizeCategory(category, `${titleText} ${raw}`),
    keywords,
    createdAt: now || new Date().toISOString()
  };
}

/** 类别归一化：显式合法值直取，否则关键词推断 */
function normalizeCategory(category, text) {
  const valid = Object.keys(CATEGORIES);
  if (category && valid.includes(category)) return category;
  const s = String(text || '').toLowerCase();
  if (CATEGORIES.architecture.some(k => s.includes(k.toLowerCase()))) return 'architecture';
  if (CATEGORIES.bugfix.some(k => s.includes(k.toLowerCase()))) return 'bugfix';
  if (CATEGORIES.optimization.some(k => s.includes(k.toLowerCase()))) return 'optimization';
  if (CATEGORIES.feature.some(k => s.includes(k.toLowerCase()))) return 'feature';
  return 'general';
}

// ============ N2 需求拆解 ============

/**
 * N2 需求拆解：IR.raw → 子需求条目（语句级）
 * 切分规则：中文句号/分号/换行；过滤长度 <6 的碎片；
 * 每条子需求附关键词（复用 kb 抽取口径，跨域算法单源）。
 */
function decomposeRequirement(ir) {
  const sentences = String(ir.raw || '')
    .split(/[。；;！!？?\n]+/)
    .map(s => s.trim())
    .filter(s => s.length >= 6 && !/^[#>*\-\s\d\.、）)]+$/.test(s));

  return sentences.slice(0, 30).map((text, i) => ({
    id: `st-${i + 1}`,
    text,
    keywords: extractKeywords(text, 8),
    priority: inferStatementPriority(text)
  }));
}

/** 子需求优先级：必须/核心/关键 → high；可选/建议 → low */
function inferStatementPriority(text) {
  const s = String(text || '');
  if (/(必须|核心|关键|紧急|不可)/.test(s)) return 'high';
  if (/(可选|建议|后期|尽量)/.test(s)) return 'low';
  return 'normal';
}

// ============ N3 架构域映射 ============

/**
 * N3 语句 → 业务域映射（评分制，与实体域映射同口径）：
 *   域名包含关键词（10）｜域名分词重合（5）｜核心功能命中（3）｜codePath 命中（2）
 * 阈值 ≥ 4 建立映射，每语句保留 top-3 候选。
 * domains 形态：[{ id, name, keyFeatures, codePath, engines }]
 */
function matchStatementToDomains(statement, domains) {
  const kws = statement.keywords || [];
  if (kws.length === 0) return [];
  return (domains || []).map(d => {
    const name = String(d.name || '');
    const features = (d.keyFeatures || []).join(' ');
    const codePath = String(d.codePath || '');
    let score = 0;
    const matched = [];
    kws.forEach(kw => {
      if (name.includes(kw)) { score += 10; matched.push(kw); return; }
      if (name.split(/[\s/·]+/).some(seg => seg.includes(kw) || kw.includes(seg))) { score += 5; matched.push(kw); return; }
      if (features.includes(kw)) { score += 3; matched.push(kw); return; }
      if (codePath.includes(kw)) { score += 2; matched.push(kw); }
    });
    return { domainId: d.id, domainName: name, score, matchedKeywords: [...new Set(matched)] };
  })
    .filter(x => x.score >= 4)
    .sort((a, b) => b.score - a.score)
    .slice(0, 3);
}

/** 全部子需求映射：[{ statement, matches[], best }]（无匹配 → matches=[] 标记待建域） */
function mapStatementsToDomains(statements, domains) {
  return statements.map(st => {
    const matches = matchStatementToDomains(st, domains);
    return { statement: st, matches, best: matches[0] || null };
  });
}

// ============ N4 模块拆分 ============

/**
 * N4 模块拆分计划：按映射结果分组 → 每域产出模块落地建议
 *   已有引擎 → 引擎承接方案（独立迭代/复用/部署建议）
 *   无匹配语句 → 新模块建议（mod- 前缀，语句关键词为种子功能）
 */
function planModuleSplit(mappings, domains, enginesById) {
  const domainById = new Map((domains || []).map(d => [d.id, d]));
  const groups = new Map();

  mappings.forEach(({ statement, best }) => {
    if (!best) return;
    if (!groups.has(best.domainId)) groups.set(best.domainId, []);
    groups.get(best.domainId).push({ statementId: statement.id, text: statement.text, score: best.score });
  });

  const plans = [...groups.entries()].map(([domainId, stmts]) => {
    const d = domainById.get(domainId) || { name: domainId, engines: [] };
    const engines = (d.engines || []).map(eid => enginesById.get(eid)).filter(Boolean);
    return {
      domainId, domainName: d.name,
      statementCount: stmts.length,
      statements: stmts,
      engines: engines.map(e => ({ id: e.id, name: e.name, codePath: e.codePath })),
      suggestion: engines.length > 0
        ? `由既有引擎承接：${engines.map(e => e.name).join('、')}（独立迭代/独立复用/独立部署）`
        : '该域未声明引擎，建议补充引擎绑定或新建模块',
      moduleType: engines.length > 0 ? 'existing' : 'new-engine-required'
    };
  }).sort((a, b) => b.statementCount - a.statementCount);

  // 无匹配语句 → 新模块建议
  const unmatched = mappings.filter(m => !m.best).map(m => ({
    statementId: m.statement.id, text: m.statement.text, keywords: m.statement.keywords
  }));
  const newModules = groupUnmatched(unmatched);

  return { plans, newModules, unmatchedCount: unmatched.length };
}

/** 无匹配语句聚类：按关键词首词分组合并为新模块建议 */
function groupUnmatched(unmatched) {
  const bySeed = new Map();
  unmatched.forEach(u => {
    const seed = (u.keywords[0] || 'general').slice(0, 20);
    if (!bySeed.has(seed)) bySeed.set(seed, []);
    bySeed.get(seed).push(u);
  });
  return [...bySeed.entries()].slice(0, 8).map(([seed, items]) => ({
    moduleId: `mod-${seed.toLowerCase().replace(/[^\w\u4e00-\u9fa5]+/g, '-')}`,
    name: `${seed}模块（建议新建）`,
    seedFeature: seed,
    statements: items.map(i => i.statementId),
    features: [...new Set(items.flatMap(i => i.keywords))].slice(0, 8),
    rationale: `${items.length} 条子需求无既有域承接，建议按种子功能"${seed}"拆分新模块`
  }));
}

// ============ N5 算法关联 ============

/**
 * N5 算法绑定：映射命中的引擎 → 引擎实现的算法（consumers 反推）
 * algorithms 形态：[{ id, name, principle, consumers[] }]
 */
function bindAlgorithmsForPlans(plans, algorithms) {
  const algoByConsumer = new Map();
  (algorithms || []).forEach(a => {
    (a.consumers || []).forEach(c => {
      if (!algoByConsumer.has(c)) algoByConsumer.set(c, []);
      algoByConsumer.get(c).push(a);
    });
  });
  return plans.map(p => ({
    domainId: p.domainId, domainName: p.domainName,
    engines: p.engines.map(e => e.id),
    algorithms: p.engines.flatMap(e => (algoByConsumer.get(e.id) || [])
      .map(a => ({ id: a.id, name: a.name, principle: a.principle })))
  })).filter(x => x.algorithms.length > 0);
}

// ============ N6 变更传播计划 ============

/**
 * N6 变更传播：影响面分析结果 → 结构化传播计划
 * 分组动作语义：算法/引擎/数据 → 高优先（回归验证）；
 *   文档 → 中优先（同步更新）；流程步骤 → 中优先（复验）；项目 → 通知。
 */
function buildPropagationPlan(impactResult, nodeById) {
  const { seed, reachableNodes } = impactResult;
  const actions = [];
  (reachableNodes || []).forEach(id => {
    const node = nodeById.get(id);
    if (!node) return;
    const kind = node.kind;
    const base = {
      target: id, kind, name: node.name || id,
      codePath: node.codePath || null
    };
    if (kind === 'algorithm') actions.push({ ...base, action: '算法验证：单源实现回归测试', priority: 'high' });
    else if (kind === 'engine') actions.push({ ...base, action: '引擎回归：契约探活 + 切换演练', priority: 'high' });
    else if (kind === 'data') actions.push({ ...base, action: '数据评估：迁移/兼容性影响确认', priority: 'high' });
    else if (kind === 'doc') actions.push({ ...base, action: '文档同步：更新对应章节', priority: 'medium' });
    else if (kind === 'flow_step') actions.push({ ...base, action: '流程复验：步骤与降级链重新演练', priority: 'medium' });
    else if (kind === 'domain' || kind === 'module') actions.push({ ...base, action: '模块影响评估：功能/接口/测试范围', priority: 'medium' });
    else if (kind === 'project') actions.push({ ...base, action: '项目通知：干系人同步变更', priority: 'low' });
  });
  const high = actions.filter(a => a.priority === 'high').length;
  return {
    seed,
    seedNode: nodeById.get(seed) ? { id: seed, kind: nodeById.get(seed).kind, name: nodeById.get(seed).name } : null,
    actions: actions.sort((a, b) => PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority]),
    summary: {
      impacted: actions.length, high, medium: actions.filter(a => a.priority === 'medium').length,
      low: actions.filter(a => a.priority === 'low').length
    }
  };
}

const PRIORITY_ORDER = { high: 0, medium: 1, low: 2 };

// ============ N7 运行记录校验 ============

/**
 * N7 归一化运行记录校验（落盘前不变式）：
 *   R1 id 非空且唯一｜R2 type 合法｜R3 requirement 型必须含语句
 *   R4 映射域必须存在于域清单｜R5 propagation 型必须含传播计划
 */
function validateNormalizationRun(run, existingIds, domainIds) {
  const errors = [];
  if (!run.id) errors.push({ rule: 'N7-R1', msg: '运行记录缺少 id' });
  else if (existingIds.has(run.id)) errors.push({ rule: 'N7-R1', msg: `运行 id 重复: ${run.id}` });
  if (!['requirement', 'propagation'].includes(run.type)) {
    errors.push({ rule: 'N7-R2', msg: `type 非法: ${run.type}（须 requirement|propagation）` });
  }
  if (run.type === 'requirement') {
    if (!run.statements || run.statements.length === 0) errors.push({ rule: 'N7-R3', msg: '需求运行必须包含拆解语句' });
    (run.mappings || []).forEach(m => (m.matches || []).forEach(x => {
      if (!domainIds.has(x.domainId)) errors.push({ rule: 'N7-R4', msg: `映射引用幽灵域: ${x.domainId}` });
    }));
  }
  if (run.type === 'propagation' && (!run.plan || !run.plan.actions)) {
    errors.push({ rule: 'N7-R5', msg: '传播运行必须包含传播计划' });
  }
  return { valid: errors.length === 0, errors };
}

module.exports = {
  buildRequirementIR, decomposeRequirement,
  matchStatementToDomains, mapStatementsToDomains,
  planModuleSplit, bindAlgorithmsForPlans,
  buildPropagationPlan, validateNormalizationRun
};
