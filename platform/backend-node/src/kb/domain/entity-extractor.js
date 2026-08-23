'use strict';

/**
 * 知识库域 · 全维实体抽取器（domain 层 · 纯函数 · 零 IO）
 * ------------------------------------------------------------------
 * 文档 → 结构化实体的归一化抽取，支撑"云端文档资源维度"图谱化：
 *   需求条目  REQ-xxx / 【需求N】 / 需求N： / "需求"句式
 *   业务规则  RULE-N / 【规则N】 / 规则N： / 必须|应当|禁止|不得 情态句
 *   架构节点  【架构】/ 架构： / 分层关键词（前端/后端/数据层/基础设施/网关…）
 *   模块定义  MODULE: / 模块： / ### 模块标题 / 依赖： 声明
 *   技术实体  复用 document-analyzer 的术语/系统名模式（口径统一）
 * 实体关系挖掘：同段共现（co_occurs）+ 同条目关联（same_context）
 * 域映射：实体 → 项目全息图谱业务域（评分制，注入域描述符即可测）
 */

const { extractEntitiesFromContent } = require('./document-analyzer');

// ============ 分段工具 ============

/** 按空行/标题分节（保留原文行号），供共现关系挖掘使用 */
function splitSections(content) {
  const lines = String(content || '').split('\n');
  const sections = [];
  let current = [];
  let startLine = 0;
  lines.forEach((line, i) => {
    const isBoundary = /^#{1,6}\s/.test(line.trim()) || (line.trim() === '' && current.length > 0);
    if (isBoundary) {
      if (current.some(l => l.trim() !== '')) sections.push({ startLine, text: current.join('\n') });
      current = [];
      startLine = i + 1;
      if (/^#{1,6}\s/.test(line.trim())) current.push(line);
    } else {
      current.push(line);
    }
  });
  if (current.some(l => l.trim() !== '')) sections.push({ startLine, text: current.join('\n') });
  return sections;
}

/** 实体 id 稳定化：小写/非字母数字折叠为 '-'，限长 48 字符（同实体跨文档归一） */
function slug(value) {
  return String(value || '')
    .toLowerCase()
    .trim()
    .replace(/[\s：:，,、（）()\[\]【】""'']+/g, '-')
    .replace(/[^\w\u4e00-\u9fa5-]/g, '')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 48);
}

// ============ 需求条目抽取 ============

/**
 * 需求条目抽取（多格式兼容）：
 *   REQ-001: 标题 | REQ-2026-014 标题
 *   【需求1】标题 | 【需求】标题
 *   需求1：标题 | 需求：标题
 *   ### 需求：标题（markdown 标题）
 */
function extractRequirementItems(content) {
  const text = String(content || '');
  const items = [];
  const seen = new Set();
  const push = (id, title, excerpt) => {
    const t = String(title || '').trim();
    if (!t || t.length < 2) return;
    const key = slug(t);
    if (!key || seen.has(key)) return;
    seen.add(key);
    items.push({
      id: id || `req:${key}`,
      title: t.slice(0, 80),
      excerpt: String(excerpt || '').trim().slice(0, 200),
      priority: inferPriority(t)
    });
  };

  // 形态一：REQ-xxx 编号（REQ-001: xxx / REQ-2026-014 xxx）
  const reqCode = /(?:^|\n)\s*(REQ[-\s]?\d{2,}(?:[-.\s]\d+)*)[：:\s]+(.+)/gi;
  let m;
  while ((m = reqCode.exec(text)) !== null) push(`req:${slug(m[1])}`, m[2].trim(), m[2].trim());

  // 形态二：【需求N】/ 【需求】
  const bracket = /【需求\s*\d*】\s*(.+)/g;
  while ((m = bracket.exec(text)) !== null) push(null, m[1].trim(), m[1].trim());

  // 形态三：需求N：/ 需求：（行首）
  const lineForm = /(?:^|\n)\s*需求\s*\d*\s*[：:]\s*(.+)/g;
  while ((m = lineForm.exec(text)) !== null) push(null, m[1].trim(), m[1].trim());

  // 形态四：markdown 标题（### 需求：xxx / ## 需求 xxx）
  const mdForm = /(?:^|\n)#{1,6}\s*需求[：:\s]*(.+)/g;
  while ((m = mdForm.exec(text)) !== null) push(null, m[1].trim(), m[1].trim());

  return items.slice(0, 60); // 上限护栏：单文档 60 条
}

/** 优先级推断：含 必须/核心/关键/紧急 → high；含 可选/建议/后期 → low；其余 normal */
function inferPriority(text) {
  const s = String(text || '');
  if (/(必须|核心|关键|紧急|重要)/.test(s)) return 'high';
  if (/(可选|建议|后期|未来|扩展)/.test(s)) return 'low';
  return 'normal';
}

// ============ 业务规则抽取 ============

/**
 * 业务规则抽取：
 *   RULE-1: xxx / 【规则N】xxx / 规则N：xxx（编号形态）
 *   含情态动词的陈述句（必须|应当|禁止|不得|需|确保|只能|不允许）
 * 严重度映射：禁止/不得/不允许 → critical；必须/确保 → high；应当/需 → normal
 */
function extractBusinessRules(content) {
  const text = String(content || '');
  const rules = [];
  const seen = new Set();
  const push = (id, rule) => {
    const r = String(rule || '').trim();
    if (!r || r.length < 4) return;
    const key = slug(r.slice(0, 60));
    if (!key || seen.has(key)) return;
    seen.add(key);
    rules.push({ id: id || `rule:${key}`, rule: r.slice(0, 160), severity: inferSeverity(r) });
  };

  // 编号形态
  let m;
  const codeForm = /(?:^|\n)\s*(?:RULE[-\s]?\d+|【规则\s*\d*】|规则\s*\d+\s*[：:])\s*(.+)/gi;
  while ((m = codeForm.exec(text)) !== null) push(null, m[1].trim());

  // 情态句形态：按句切分再过滤
  const sentences = text.split(/[。；;！!？?\n]/);
  sentences.forEach(s => {
    const t = s.trim();
    if (t.length >= 6 && t.length <= 120 && /(必须|应当|禁止|不得|不允许|只能|确保)/.test(t)) {
      push(null, t.replace(/^[•\-\*\d\.\s]+/, ''));
    }
  });

  return rules.slice(0, 60);
}

/** 严重度推断 */
function inferSeverity(rule) {
  const s = String(rule || '');
  if (/(禁止|不得|不允许)/.test(s)) return 'critical';
  if (/(必须|确保|只能)/.test(s)) return 'high';
  return 'normal';
}

// ============ 架构节点抽取 ============

const ARCH_LAYERS = [
  { layer: 'frontend', keywords: ['前端', '客户端', '浏览器', 'Web 端', 'Web端', 'UI 层', '视图层'] },
  { layer: 'backend', keywords: ['后端', '服务端', 'API 层', '业务层', '应用层', '接口层'] },
  { layer: 'data', keywords: ['数据层', '数据库', '存储层', '持久化', '数据资产', '缓存'] },
  { layer: 'infrastructure', keywords: ['基础设施', '部署', '运维', '容器', '网关', '消息队列', '中间件'] }
];

/**
 * 架构节点抽取：
 *   【架构】xxx / 架构：xxx / 架构节点：xxx（声明形态）
 *   分层关键词命中（推断形态，输出 layer 分类）
 */
function extractArchitectureNodes(content) {
  const text = String(content || '');
  const nodes = [];
  const seen = new Set();
  const push = (id, name, layer, desc) => {
    const n = String(name || '').trim();
    if (!n || n.length < 2) return;
    const key = slug(n);
    if (!key || seen.has(key)) return;
    seen.add(key);
    nodes.push({
      id: `arch:${key}`, name: n.slice(0, 60), layer: layer || 'general',
      desc: String(desc || '').trim().slice(0, 120)
    });
  };

  // 声明形态
  let m;
  const declForm = /(?:^|\n)\s*(?:【架构】|架构\s*节点?\s*[：:])\s*(.+)/g;
  while ((m = declForm.exec(text)) !== null) {
    const line = m[1].trim();
    push(null, line.split(/[,，。;；]/)[0], detectLayer(line), line);
  }

  // 分层关键词形态：命中行提取首个名词短语
  ARCH_LAYERS.forEach(({ layer, keywords }) => {
    keywords.forEach(kw => {
      const idx = text.indexOf(kw);
      if (idx === -1) return;
      const lineStart = text.lastIndexOf('\n', idx) + 1;
      const lineEnd = text.indexOf('\n', idx);
      const line = text.slice(lineStart, lineEnd === -1 ? text.length : lineEnd).trim();
      if (line && line.length >= 4) push(null, `${kw}`, layer, line.slice(0, 120));
    });
  });

  return nodes.slice(0, 30);
}

/** 分层归属推断 */
function detectLayer(text) {
  const s = String(text || '');
  for (const { layer, keywords } of ARCH_LAYERS) {
    if (keywords.some(kw => s.includes(kw))) return layer;
  }
  return 'general';
}

// ============ 模块定义抽取 ============

/**
 * 模块定义抽取：
 *   MODULE: name - desc | 模块：name
 *   ### 模块名（markdown 标题，含"模块"字样或后续有"依赖："声明）
 *   依赖：a, b（同段落内声明 → 计入 deps）
 */
function extractModuleDefinitions(content) {
  const text = String(content || '');
  const modules = [];
  const seen = new Set();
  const push = (name, desc, deps) => {
    const n = String(name || '').trim().replace(/^[#\s]+/, '');
    if (!n || n.length < 2) return;
    const key = slug(n);
    if (!key || seen.has(key)) return;
    seen.add(key);
    modules.push({
      id: `moddef:${key}`, name: n.slice(0, 60),
      desc: String(desc || '').trim().slice(0, 120),
      deps: (deps || []).map(d => String(d).trim()).filter(Boolean).slice(0, 10)
    });
  };

  const sections = splitSections(text);
  let m;

  // 形态一：MODULE: name - desc（标准结构化声明）
  const modCode = /(?:^|\n)\s*(?:MODULE|模块)\s*[：:]\s*([^\n-【】()（）]{2,40})(?:\s*[-—]\s*([^\n]+))?/g;
  while ((m = modCode.exec(text)) !== null) {
    const deps = extractDeps(m[2] || '');
    push(m[1], (m[2] || '').split(/依赖|deps/i)[0], deps);
  }

  // 形态二：markdown 标题（### 模块名 / ### xxx 模块）
  const mdForm = /(?:^|\n)#{1,6}\s*(.{2,40}(?:模块|引擎|服务|子系统))\s*$/g;
  while ((m = mdForm.exec(text)) !== null) {
    const line = m.index;
    // 查找标题后 3 行内的依赖声明
    const after = text.slice(m.index, m.index + 300);
    push(m[1], '', extractDeps(after));
    void line;
  }

  return modules.slice(0, 30);
}

/** "依赖：a, b" / "依赖 a、b" 声明解析 */
function extractDeps(text) {
  const m = /依赖[：:，,\s]+([^\n。；;]{2,120})/.exec(String(text || ''));
  if (!m) return [];
  return m[1].split(/[,，、;；\s]+/).filter(x => x.length >= 2).slice(0, 10);
}

// ============ 关键词提取（归一化流水线复用） ============

const STOPWORDS = new Set([
  '的', '了', '和', '与', '或', '及', '在', '为', '对', '从', '被', '把', '让', '向', '是',
  '将', '由', '以', '并', '而', '则', '之', '其', '该', '此', '这些', '那些', '进行', '实现',
  '提供', '支持', '包含', '包括', '需要', '可以', '能够', '以及', '一个', '每个', '所有',
  '系统', '功能', '能力', '内容', '相关', '对应', '同时', '通过', '使用', '基于', '完成'
]);

/**
 * 关键词提取：切词 → 停用词过滤 → 词频排序
 * 中文连续段按 2-4 字滑窗无字典不切，此处采用「标点切分 + 短语保留」策略：
 * 输出为短语级关键词（保持业务语义完整）。
 */
function extractKeywords(text, limit = 12) {
  const phrases = String(text || '')
    .split(/[\s，。；;：:、,.\-—…！!？?（）()\[\]【】""''\/|·]+/)
    .map(p => p.trim())
    .filter(p => p.length >= 2 && p.length <= 12 && !STOPWORDS.has(p) && !/^\d+$/.test(p));
  const freq = new Map();
  phrases.forEach(p => freq.set(p, (freq.get(p) || 0) + 1));
  return [...freq.entries()]
    .sort((a, b) => b[1] - a[1] || b[0].length - a[0].length)
    .slice(0, limit)
    .map(([k]) => k);
}

// ============ 归一化实体总抽取 ============

/**
 * 全维实体总抽取：需求/规则/架构/模块 + 技术实体（术语/系统名复用既有口径）
 * 输出统一实体形态：{ id, type, label, description, attributes }
 */
function extractAllEntities(content) {
  const requirements = extractRequirementItems(content);
  const rules = extractBusinessRules(content);
  const architecture = extractArchitectureNodes(content);
  const modules = extractModuleDefinitions(content);
  const technical = extractEntitiesFromContent(content);

  const entities = [];

  requirements.forEach(r => entities.push({
    id: r.id, type: 'requirement', label: r.title,
    description: r.excerpt || r.title,
    attributes: { priority: r.priority, sourceType: 'requirement' }
  }));

  rules.forEach(r => entities.push({
    id: r.id, type: 'business_rule', label: r.rule.slice(0, 40),
    description: r.rule,
    attributes: { severity: r.severity, sourceType: 'rule' }
  }));

  architecture.forEach(a => entities.push({
    id: a.id, type: 'architecture', label: a.name,
    description: a.desc || a.name,
    attributes: { layer: a.layer, sourceType: 'architecture' }
  }));

  modules.forEach(md => entities.push({
    id: md.id, type: 'module_def', label: md.name,
    description: md.desc || md.name,
    attributes: { deps: md.deps, sourceType: 'module' }
  }));

  // 技术实体（术语 + 系统名 → kind 合并为 technical_term）
  technical.filter(t => t && t.value).forEach(t => {
    const key = slug(t.value);
    if (!key) return;
    entities.push({
      id: `term:${key}`, type: 'technical_term', label: t.value,
      description: t.context || t.value,
      attributes: { sourceType: 'technical', kind: t.kind }
    });
  });

  // 去重（同 id 保留首个）
  const seen = new Set();
  return entities.filter(e => {
    if (!e.id || seen.has(e.id)) return false;
    seen.add(e.id);
    return true;
  }).slice(0, 200); // 护栏：单文档 200 实体
}

// ============ 实体关系挖掘 ============

/**
 * 实体关系挖掘：同节（section）共现 → co_occurs；
 * 同为同一需求条目衍生的实体 → same_context。
 * 返回 [{ source, target, type, evidence }]（source/target 为实体 id）
 */
function mineEntityRelations(entities, content) {
  const relations = [];
  if (!entities || entities.length < 2) return relations;
  const sections = splitSections(content);
  const pairSeen = new Set();
  const key = (a, b) => (a < b ? `${a}|${b}` : `${b}|${a}`);

  sections.forEach(section => {
    const present = entities.filter(e => {
      const label = e.label || '';
      return label && section.text.includes(label);
    });
    // 同节共现：实体两两建边（上限防爆炸：每节前 12 个实体）
    const cap = present.slice(0, 12);
    for (let i = 0; i < cap.length; i++) {
      for (let j = i + 1; j < cap.length; j++) {
        const k = key(cap[i].id, cap[j].id);
        if (pairSeen.has(k)) continue;
        pairSeen.add(k);
        relations.push({
          source: cap[i].id, target: cap[j].id, type: 'co_occurs',
          evidence: section.text.trim().slice(0, 80)
        });
      }
    }
  });

  return relations.slice(0, 300); // 护栏：单文档 300 条关系
}

// ============ 实体 → 业务域映射 ============

/**
 * 实体映射到项目全息图谱业务域（评分制）：
 *   域名包含实体关键词（10）｜域名分词重合（5/词）｜核心功能命中（3/词）｜codePath 命中（2/词）
 *   阈值 ≥ 5 才建立映射，每实体最多 2 个域
 * domains 形态：[{ id, name, keyFeatures, codePath }]（由调用方注入，域层可测）
 */
function matchEntityToDomains(entity, domains) {
  const text = `${entity.label || ''} ${entity.description || ''}`;
  const kws = extractKeywords(text, 6);
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
    .filter(x => x.score >= 5)
    .sort((a, b) => b.score - a.score)
    .slice(0, 2);
}

module.exports = {
  // 分段与工具
  splitSections, slug, extractKeywords,
  // 单类抽取
  extractRequirementItems, extractBusinessRules,
  extractArchitectureNodes, extractModuleDefinitions,
  // 总抽取与关系挖掘
  extractAllEntities, mineEntityRelations, matchEntityToDomains
};
