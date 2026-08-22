'use strict';

/**
 * 图谱增强咨询（application 层 mixin 用例族）
 * ------------------------------------------------------------------
 * consultAtlas：AI 对话 + 项目全息图谱双引擎协作。
 * 流程：问题关键词 → 图谱资产检索 → 相关域结构化上下文 → 架构师专家
 *       咨询（图谱上下文注入）→ 返回专家回答 + 图谱证据链。
 * 依赖方向：application → 域外 project-atlas（只读查询，无环）。
 */

// 惰性 require（延迟边，避免加载环）
function _getAtlas() {
  return require('../../project-atlas');
}

/** 从问题构造图谱上下文（检索相关域 + 影响面） */
function buildAtlasContext(question) {
  const atlas = _getAtlas();
  const kw = String(question || '');
  const context = { matchedDomains: [], evidence: [] };

  // ① 按域 ID/名称直接匹配（问题里提到"专家联盟"、"知识库"等）
  for (const d of atlas.DOMAINS) {
    if (kw.includes(d.name) || kw.includes(d.id)) {
      const detail = atlas.getDomainDetail(d.id);
      if (detail) context.matchedDomains.push(detail);
    }
  }
  // ② 关键词资产检索兜底
  if (context.matchedDomains.length === 0) {
    const keywords = kw.split(/[\s,，。？?！!、]+/).filter(t => t.length >= 2);
    for (const k of keywords.slice(0, 4)) {
      const hits = atlas.searchAtlas(k);
      for (const n of hits.nodes.slice(0, 3)) {
        if (n.kind === 'domain' || n.kind === 'module') {
          const detail = atlas.getDomainDetail(n.id);
          if (detail && !context.matchedDomains.some(x => x.id === detail.id)) {
            context.matchedDomains.push(detail);
          }
        }
      }
    }
  }
  // ③ 证据链：每个匹配域的影响面
  for (const d of context.matchedDomains.slice(0, 3)) {
    const imp = atlas.impact(d.id);
    if (imp) {
      context.evidence.push({ seed: d.id, impactedCount: imp.total, impacted: imp.impacted.slice(0, 10) });
    }
  }
  return context;
}

/** 图谱上下文 → 专家可读的结构化文本 */
function renderAtlasContext(ctx) {
  if (!ctx.matchedDomains.length) return '';
  const lines = ['## 项目全息图谱上下文（机器检索的真实架构数据）'];
  for (const d of ctx.matchedDomains.slice(0, 4)) {
    lines.push(`### 业务域 [${d.id}] ${d.name}（代码: ${d.codePath}）`);
    lines.push(`核心功能: ${(d.keyFeatures || []).join('；')}`);
    if (d.engines.length) lines.push(`依赖引擎: ${d.engines.map(e => `${e.id}(${e.name})`).join(', ')}`);
    if (d.algorithms.length) lines.push(`实现算法: ${d.algorithms.map(a => `${a.id}(${a.name}, ${a.codePath})`).join('; ')}`);
    if (d.dataAssets.length) lines.push(`数据资产: ${d.dataAssets.map(x => x.file).join(', ')}`);
    if (d.docs.length) lines.push(`关联文档: ${d.docs.map(x => x.path).join(', ')}`);
  }
  for (const e of ctx.evidence) {
    lines.push(`影响面 [${e.seed}]: 波及 ${e.impactedCount} 个节点（${e.impacted.map(i => `${i.kind}:${i.id}`).join(', ')}）`);
  }
  return lines.join('\n');
}

/** 图谱增强咨询：架构师专家 + 全息图谱上下文 */
async function consultAtlas(question, options = {}) {
  const atlasContext = buildAtlasContext(question);
  const contextText = renderAtlasContext(atlasContext);

  const result = await this.consult('atlas-expert', [{ role: 'user', content: question }], {
    ...options,
    problemContext: contextText || '（图谱未直接命中，请基于引擎宇宙 17 引擎与 AINA-STD-001 架构规范回答）',
    includeExpertContext: true
  });

  return {
    ...result,
    atlas: {
      matchedDomainIds: atlasContext.matchedDomains.map(d => d.id),
      matchedDomains: atlasContext.matchedDomains.map(d => ({ id: d.id, name: d.name, codePath: d.codePath })),
      evidenceCount: atlasContext.evidence.reduce((s, e) => s + e.impactedCount, 0)
    }
  };
}

module.exports = { consultAtlas };
