'use strict';

/**
 * 辩论综合算法（domain 层 · 纯算法）
 * ------------------------------------------------------------------
 * 包含：多轮辩论综合结论生成、跨专家关键词提取（中文 2~3 字滑窗 n-gram）、
 * 共识提取（≥半数共现）、分歧提取（两两 Jaccard 最小对）、最终建议生成。
 * 历史教训（A20）：原硬编码模板话术与实际辩论内容无关，已改为真实文本提取。
 */

/**
 * 中文+英文关键词提取（停用词过滤 + 2~3 字滑窗）。
 * 与 expert-alliance-engine._keywords 算法一致。
 * @param {string} text 响应文本
 * @returns {Set<string>} 关键词集合
 */
function keywordsOf(text) {
  const stop = new Set(['的', '了', '和', '与', '是', '在', '我们', '可以', '需要', '建议', '应该', '一种', '通过', '进行', '以及', '或者', 'the', 'a', 'to', 'of', 'and', 'is', '系统', '方案', '问题', '分析', '采用', '引入', '保证', '优先', '解决']);
  const out = new Set();
  const segments = String(text || '').match(/[一-龥]+|[a-zA-Z]{3,}/g) || [];
  for (const seg of segments) {
    if (/[a-zA-Z]/.test(seg)) {
      if (!stop.has(seg.toLowerCase())) out.add(seg.toLowerCase());
      continue;
    }
    for (let size = 2; size <= 3; size++) {
      for (let i = 0; i + size <= seg.length; i++) {
        const w = seg.slice(i, i + size);
        if (!stop.has(w)) out.add(w);
      }
    }
  }
  return out;
}

/**
 * 提取跨多数专家的共性关键词作为真实共识主题。
 * @param {Array<{expert:string, response:string, confidence:number}>} responses
 * @returns {string}
 */
function extractConsensus(responses) {
  if (!responses.length) return '暂无足够数据形成共识。';
  const wordSets = responses.map(r => keywordsOf(r.response));
  const freq = new Map();
  wordSets.forEach(ws => ws.forEach(w => freq.set(w, (freq.get(w) || 0) + 1)));
  const threshold = Math.max(2, Math.ceil(responses.length / 2));
  const topics = [...freq.entries()]
    .filter(([, c]) => c >= threshold)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([w]) => w);
  if (!topics.length) {
    return `各专家表述侧重不同，未提取到跨多数（≥${threshold} 位）的显式共性关键词，建议以置信度最高专家意见为主参考。`;
  }
  return topics.map((t, i) => `${i + 1}. 多数专家（≥${threshold} 位）共同关注：${t}`).join('\n');
}

/**
 * 两两 Jaccard 相似度最低的一对即主要分歧，输出独有关键词。
 * @param {Array<{expert:string, response:string}>} responses
 * @returns {string}
 */
function extractDivergences(responses) {
  if (responses.length < 2) return '参与专家不足两位，无分歧可析。';
  const wordSets = responses.map(r => keywordsOf(r.response));
  let minSim = 1;
  let minPair = [0, 1];
  for (let i = 0; i < responses.length; i++) {
    for (let j = i + 1; j < responses.length; j++) {
      const a = wordSets[i], b = wordSets[j];
      const inter = [...a].filter(w => b.has(w)).length;
      const union = new Set([...a, ...b]).size || 1;
      const sim = inter / union;
      if (sim < minSim) { minSim = sim; minPair = [i, j]; }
    }
  }
  const [x, y] = minPair;
  const onlyX = [...wordSets[x]].filter(w => !wordSets[y].has(w)).slice(0, 3);
  const onlyY = [...wordSets[y]].filter(w => !wordSets[x].has(w)).slice(0, 3);
  const parts = [`${responses[x].expert} 与 ${responses[y].expert} 观点重合度最低（Jaccard=${Math.round(minSim * 100)}/100）`];
  if (onlyX.length) parts.push(`${responses[x].expert} 独有关键词：${onlyX.join('、')}`);
  if (onlyY.length) parts.push(`${responses[y].expert} 独有关键词：${onlyY.join('、')}`);
  return parts.join('；');
}

/**
 * 基于置信度最高专家 + 共性交叉验证 + 分歧裁决生成最终建议。
 * @param {Array<{expert:string, confidence:number}>} responses
 * @returns {string}
 */
function generateFinalRecommendation(responses) {
  if (!responses.length) return '暂无可参考的专家意见。';
  const topConf = responses.reduce((a, b) => ((b.confidence || 0.6) >= (a.confidence || 0.6) ? b : a));
  return `1. **优先采纳**：置信度最高的 ${topConf.expert}（${topConf.confidence || 0.6}）意见作为主方案
2. **交叉验证**：以其余 ${responses.length - 1} 位专家意见中的共性主题（见"核心共识"）做交叉印证
3. **分歧裁决**：对"分歧观点"中列出的独有关键词，结合具体业务场景权衡取舍
4. **落地节奏**：先在单点验证，再依据 ${responses.length} 位专家共同识别的风险控制点逐步推广`;
}

/**
 * 汇总多轮辩论历史生成综合结论（Markdown）。
 * @param {Array<{results:Array<{expert:Object, response:string, confidence:number, success:boolean}>}>} history
 * @returns {string}
 */
function synthesizeDebate(history) {
  const allResponses = [];
  history.forEach(round => {
    round.results.forEach(r => {
      if (r.success) {
        allResponses.push({ expert: r.expert.name, response: r.response, confidence: r.confidence || 0.6 });
      }
    });
  });

  return `## 多专家辩论综合结论

基于 ${history.length} 轮辩论，共 ${allResponses.length} 位专家参与讨论。

### 核心共识
${extractConsensus(allResponses)}

### 分歧观点
${extractDivergences(allResponses)}

### 最终建议
${generateFinalRecommendation(allResponses)}

### 专家贡献
${allResponses.map(r => `- **${r.expert}**: 提供了专业分析`).join('\n')}`;
}

module.exports = { keywordsOf, extractConsensus, extractDivergences, generateFinalRecommendation, synthesizeDebate };
