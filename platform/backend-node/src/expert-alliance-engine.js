'use strict';

/**
 * 企业级「专家联盟处理引擎」(Expert Alliance Processing Engine)
 * ----------------------------------------------------------------
 * 定位：在现有 ExpertAlliance（专家个体能力）/ ExpertGraph（能力关系图谱）/
 *       Dispatcher（调度策略）/ LLMGateway（大模型网关）之上，提供一层
 *       "以算法驱动的联盟协作处理模式"。
 *
 * 处理流水线（参考 ai-engine 的算子/图谱/工作流算法风格）：
 *   classifyIntent → composeTeam → deliberate(并行咨询 + 辩论收敛)
 *   → synthesize(置信度加权综合) → qualityGate(质量门禁) → learn(反馈学习)
 *
 * 关键算法：
 *   - 意图多标签打分（关键词 + 专家类型 + 历史命中）
 *   - 最优组队：能力匹配分 + 协同增益(图谱边权) + 负载均衡(Dispatcher 指标) 多目标选择
 *   - 辩论收敛：加权表决 + 共识度(一致率/方差) + 少数派保留
 *   - 综合合成：结构化 JSON + 置信度加权
 *   - 质量门禁：置信度阈值 + 专家一致性 + 自洽性校验（可降级重试）
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const DATA_DIR = path.join(__dirname, '..', 'data');
const TRACE_FILE = 'alliance_traces.jsonl';

function readJSON(file, fallback) {
  try {
    const fp = path.join(DATA_DIR, file);
    if (!fs.existsSync(fp)) return fallback;
    const raw = fs.readFileSync(fp, 'utf8');
    return raw ? JSON.parse(raw) : fallback;
  } catch (e) {
    return fallback;
  }
}

/** 原子写 JSON（G5 金融级）：tmp + rename，崩溃不产生半写文件 */
function atomicWriteJSON(fp, data) {
  try {
    fs.mkdirSync(path.dirname(fp), { recursive: true });
    const tmp = fp + '.tmp';
    fs.writeFileSync(tmp, JSON.stringify(data, null, 2), 'utf8');
    fs.renameSync(tmp, fp);
  } catch (_e) { /* best-effort：反馈学习落盘失败不阻断主流程 */ }
}

function appendTrace(trace) {
  try {
    fs.mkdirSync(DATA_DIR, { recursive: true });
    fs.appendFileSync(path.join(DATA_DIR, TRACE_FILE), JSON.stringify(trace) + '\n', 'utf8');
  } catch (e) {
    // best-effort
  }
}

// 意图识别模式（A16 单一真相源 · AINA A3）：直接引用专家联盟域包 domain 层定义，
// 不经过编排层（expert-alliance.js），保持 application → domain 的最短依赖路径。
const { INTENT_PATTERNS } = require('./expert-alliance/domain/intent-patterns');
// 学习技能沉淀（G1 闭环：流程声明 writes 与实现一致）
const { synthesizeSkills, rankSkills } = require('./expert-alliance/domain/skill-synthesis');
const { SkillStore } = require('./expert-alliance/infrastructure/skill-store');

class ExpertAllianceEngine {
  constructor({ alliance, expertGraph, dispatcher, gateway, options = {} } = {}) {
    this.alliance = alliance;
    this.expertGraph = expertGraph;
    this.dispatcher = dispatcher;
    this.gateway = gateway;

    this.config = Object.assign({
      maxTeamSize: 4,
      minTeamSize: 1,
      enableDebate: true,
      debateRounds: 2,
      adaptiveDebate: true,         // 初始轮已收敛则跳过辩论轮（实测省 ~26s）
      debateMaxTokens: 900,         // 辩论轮令牌上限（短答快收敛，实测显著降延迟）
      consultTimeoutMs: 60000,      // 单专家咨询超时（超时隔离不阻断整条管线）
      consensusThreshold: 0.6,      // 一致性门禁
      confidenceThreshold: 0.55,    // 综合置信度门禁
      capabilityWeight: 1.0,
      synergyWeight: 0.4,
      loadBalanceWeight: 0.3,
      maxRetries: 1,
      timeoutMs: 120000,
      chiefModel: null              // 综合阶段可指定 model 覆盖
    }, options);

    // 运行时反馈：intent -> 高频命中专家，作为组队先验
    this.intentPriors = readJSON('alliance_intent_priors.json', {});
    // 学习技能库（独立资产 alliance_learned_skills.json，与 ai-integration 的 learned_skills.json 互不覆写）
    this.skillStore = new SkillStore(options.skillStore || {});
  }

  // ===================== 阶段一：意图识别 =====================
  classifyIntent(question) {
    const q = String(question || '').toLowerCase();
    const scores = INTENT_PATTERNS.map(p => {
      let score = 0;
      for (const kw of p.keywords) {
        if (q.includes(kw.toLowerCase())) score += 1;
      }
      return { intent: p.intent, score };
    }).filter(s => s.score > 0)
      .sort((a, b) => b.score - a.score);

    const total = scores.reduce((s, x) => s + x.score, 0) || 1;
    const ranked = scores.map(s => ({
      intent: s.intent,
      score: s.score,
      confidence: Math.round((s.score / total) * 100) / 100
    }));

    // 主意图置信度归一：若多个意图并列则整体置信度下降（多义性）
    const top = ranked[0] || { intent: 'general', score: 0, confidence: 0 };
    const multiModal = ranked.length > 1 && ranked[1].score === top.score;

    return {
      primary: top.intent,
      confidence: multiModal ? Math.round(top.confidence * 0.7 * 100) / 100 : top.confidence,
      candidates: ranked.slice(0, 3),
      ambiguous: multiModal,
      coverage: Math.min(1, total / 3)
    };
  }

  // ===================== 阶段二：最优组队 =====================
  /**
   * 多目标组队：
   *   score(e) = capabilityWeight * cap(e)
   *            + synergyWeight  * synergy(e | team)
   *            + loadBalanceWeight * (1 - load(e))
   * cap(e): 能力/意图匹配（包含主意图 + 候选意图 + 先验命中）
   * synergy: 与已选成员在 expertGraph 上的边权和（协同增益）
   * load: Dispatcher 当前负载（失败率/排队），越低越好
   */
  composeTeam(question, intent, options = {}) {
    const teamSize = Math.min(
      this.config.maxTeamSize,
      Math.max(this.config.minTeamSize, options.teamSize || 3)
    );
    const exclude = options.excludeIds instanceof Set ? options.excludeIds : new Set(options.excludeIds || []);
    const candidates = (this.alliance ? this.alliance.listExperts() : [])
      .filter(e => e.status === 'active')
      .filter(e => !exclude.has(e.id)); // G3 重试换血：排除首次团队成员

    if (candidates.length === 0) {
      return { team: [], score: 0, reason: 'no_active_experts' };
    }

    const prior = (this.intentPriors[intent.primary] || {}).hits || {};
    const intentsOfInterest = new Set([
      intent.primary,
      ...intent.candidates.map(c => c.intent)
    ]);

    const capScore = (expert) => {
      const caps = (expert.capabilities || []).map(c => c.toLowerCase());
      const q = question.toLowerCase();
      let s = 0;
      for (const cap of caps) if (q.includes(cap)) s += 2;
      if (intentsOfInterest.has(expert.type)) s += 3;
      s += (prior[expert.id] || 0) * 0.5; // 历史先验
      const m = expert.metrics || {};
      s += (m.success_rate || 0.7) * 1.5 + (m.avg_confidence || 0.6) * 1.0;
      return s;
    };

    const loadScore = (expert) => {
      if (!this.dispatcher || !this.dispatcher.getLoadMetrics) return 0;
      const lm = this.dispatcher.getLoadMetrics(expert.id);
      if (!lm) return 0;
      const fail = lm.failureRate || 0;
      const queue = lm.queued || 0;
      return Math.min(1, fail * 0.7 + Math.min(queue / 10, 1) * 0.3);
    };

    const team = [];
    const teamSynergy = {};
    let remaining = candidates.slice();

    while (team.length < teamSize && remaining.length > 0) {
      let best = null;
      let bestScore = -Infinity;

      for (const e of remaining) {
        const cap = capScore(e);
        let synergy = 0;
        for (const member of team) {
          const edge = this.expertGraph
            ? (this.expertGraph.edges || []).find(ed =>
                (ed.source === e.id && ed.target === member.id) ||
                (ed.source === member.id && ed.target === e.id))
            : null;
          if (edge) synergy += edge.weight || 0;
        }
        const load = loadScore(e);
        const score =
          this.config.capabilityWeight * cap +
          this.config.synergyWeight * synergy -
          this.config.loadBalanceWeight * load;

        if (score > bestScore) {
          bestScore = score;
          best = e;
          teamSynergy[e.id] = synergy;
        }
      }

      if (!best) break;
      team.push(best);
      remaining = remaining.filter(e => e.id !== best.id);
    }

    const totalSynergy = team.reduce((s, e) => s + (teamSynergy[e.id] || 0), 0);

    // 安全类强制保障（G4 · EAF-STD-001 §4 阶段二）：安全意图必须优先安全专家入队。
    // 常规评分未选入时替换末位成员；无安全专家时显式记录（不静默）。
    let securityNote = null;
    if (intent.primary === 'security') {
      const hasSecurity = team.some(e => (e.type || '').toLowerCase().includes('security'));
      if (!hasSecurity) {
        const securityExpert = candidates.find(e =>
          (e.type || '').toLowerCase().includes('security') && !team.includes(e)
        );
        if (securityExpert) {
          if (team.length > 1) team[team.length - 1] = securityExpert; // 替换末位保规模
          else team[0] = securityExpert;
          securityNote = `安全类问题已强制选入安全专家 ${securityExpert.name}`;
        } else {
          securityNote = '安全类问题但注册表中无安全专家，已按常规评分组队（建议补充安全专家）';
        }
      }
    }

    return {
      team: team.map(e => ({
        id: e.id,
        name: e.name,
        type: e.type,
        capabilities: e.capabilities,
        synergy: teamSynergy[e.id] || 0
      })),
      team_size: team.length,
      total_synergy: totalSynergy,
      security_note: securityNote,
      dispatch_strategy: this.dispatcher ? this.dispatcher.strategy : 'unknown'
    };
  }

  // ===================== 阶段三：并行咨询 + 辩论收敛 =====================
  /**
   * 1) 并行向 team 各专家咨询（带超时与隔离）
   * 2) 若开启辩论，则进行 debateRounds 轮交叉评审：每轮把他人意见摘要回喂，
   *    专家可坚持/修正；最终用加权表决计算共识度
   */
  async deliberate(question, team, context = {}, options = {}) {
    const rounds = [];
    const messages = [
      { role: 'user', content: question }
    ];
    let degraded = null;

    // 1) 首轮并行咨询
    let consultResults = await this._parallelConsult(team, messages, context);
    rounds.push({ round: 0, type: 'initial', results: consultResults });

    // G6 降级链（EAF-STD-001 §4 降级路径#1）：咨询引擎不可用（全部专家失败）
    //   → 单专家直答重试一次（团队首位 + 精简上下文），仍失败则保留失败结果回归主流
    if (consultResults.length > 0 && consultResults.every(r => r.error)) {
      const soloResults = await this._parallelConsult([team[0]], messages, context, { tag: 'solo-fallback' });
      degraded = {
        from: 'multi-consult',
        to: 'single-expert-consult',
        reason: '全部专家咨询失败，已降级为单专家直答重试'
      };
      rounds.push({ round: 0.5, type: 'degraded-solo', results: soloResults, reason: degraded.reason });
      consultResults = soloResults;
    }

    let finalResults = consultResults;

    // 2) 辩论收敛（自适应：初始轮已收敛则跳过，实测高频问题省 ~26s）
    //    G6 降级链：辩论轮全部失败（辩论引擎不可用）→ 回退初始轮直答形态，
    //    回归主流（共识计算 → 综合合成的输出契约保持一致）
    if (this.config.enableDebate && (options.enableDebate !== false)) {
      const initialConsensus = this._consensus(consultResults);
      const converged = this.config.adaptiveDebate
        && initialConsensus.agreement >= this.config.consensusThreshold;

      if (converged) {
        rounds.push({ round: 1, type: 'debate-skipped', reason: `初始共识度 ${initialConsensus.agreement} 已达阈值，跳过辩论轮` });
      } else {
        for (let r = 1; r <= this.config.debateRounds; r++) {
          const othersDigest = consultResults.map(c => ({
            expert: c.expertName,
            stance: c.response ? c.response.slice(0, 400) : c.error
          }));
          const debateMsgs = [
            { role: 'user', content: question },
            { role: 'assistant', content: '【其他专家观点】\n' + JSON.stringify(othersDigest, null, 2) },
            { role: 'user', content: '请基于其他专家观点审视自己的结论：若认同请强化，若存在分歧请明确反驳并给出依据。保持你的专业立场。' }
          ];
          const roundResults = await this._parallelConsult(team, debateMsgs, context, { tag: 'debate', maxTokens: this.config.debateMaxTokens });
          rounds.push({ round: r, type: 'debate', results: roundResults });

          // 辩论轮全部失败：辩论引擎不可用 → 回退初始轮结果（保住有效意见）
          if (roundResults.length > 0 && roundResults.every(x => x.error)) {
            degraded = {
              from: 'debate',
              to: 'single-round-consult',
              reason: `第 ${r} 轮辩论全部专家失败（辩论引擎不可用），已回退初始轮直答形态`
            };
            rounds.push({ round: r + 0.5, type: 'debate-degraded', reason: degraded.reason });
            break; // 辩论通道已不可用，继续轮次无意义
          }
          finalResults = roundResults;

          // 逐轮收敛检测：本轮辩论后共识已达阈值则提前终止（省后续辩论轮）
          const roundConsensus = this._consensus(roundResults);
          if (roundConsensus.agreement >= this.config.consensusThreshold && r < this.config.debateRounds) {
            rounds.push({ round: r + 1, type: 'debate-converged', reason: `第 ${r} 轮辩论后共识度 ${roundConsensus.agreement} 已达阈值，提前收敛` });
            break;
          }
        }
      }
    }

    const consensus = this._consensus(finalResults);

    return {
      rounds: rounds.length,
      rounds_detail: rounds, // 轮次明细（含降级/收敛标记，供 trace 审计）
      initial: consultResults,
      final: finalResults,
      consensus,
      degraded
    };
  }

  async _parallelConsult(team, messages, context, meta = {}) {
    const tag = meta.tag || 'consult';
    const timeoutMs = meta.timeoutMs || this.config.consultTimeoutMs;
    const tasks = team.map(async (member) => {
      const t0 = Date.now();
      try {
        if (!this.alliance) {
          return { expertId: member.id, expertName: member.name, response: '(联盟未启用，模拟)', duration: 0, error: null };
        }
        // 单专家超时隔离：挂起的 LLM 调用只损失该专家，不阻断整条管线（银行级可用性）
        const consultPromise = this.alliance.consult(member.id, messages, {
          problemContext: context.background,
          businessConstraints: context.constraints,
          tag,
          maxTokens: meta.maxTokens
        });
        const timeoutPromise = new Promise((_, rej) => {
          // 守卫定时器保持事件循环存活直至触发（unref 会在空转时让进程提前退出，守卫失效）
          setTimeout(() => rej(new Error(`专家咨询超时（${timeoutMs}ms，已隔离）`)), timeoutMs);
        });
        const res = await Promise.race([consultPromise, timeoutPromise]);
        return {
          expertId: member.id,
          expertName: member.name,
          expertType: member.type,
          response: res.response || null,
          confidence: res.metadata ? (res.metadata.confidence || 0.6) : 0.6,
          duration: Date.now() - t0,
          error: res.error || null,
          metadata: res.metadata || {}
        };
      } catch (e) {
        return { expertId: member.id, expertName: member.name, response: null, duration: Date.now() - t0, error: e.message };
      }
    });

    return Promise.all(tasks);
  }

  /**
   * 共识度计算：
   *   - 有效意见数 / 总数
   *   - 立场一致率：基于关键词重叠的启发式（网关深度聚敛在 synthesize 阶段完成）
   * 返回 { agreement, validCount, conflict, summary }
   */
  _consensus(results) {
    const valid = results.filter(r => r.response && !r.error);
    const validCount = valid.length;
    const total = results.length || 1;

    let agreement = 0;
    let conflict = [];

    // 启发式一致率：以首条有效意见为基准，统计其他意见与其关键词重合度
    if (validCount > 0) {
      const baseWords = Array.from(this._keywords(valid[0].response));
      let overlapSum = 0;
      for (let i = 1; i < validCount; i++) {
        const w = this._keywords(valid[i].response);
        const inter = baseWords.filter(x => w.has(x)).length;
        const union = new Set([...baseWords, ...w]).size || 1;
        overlapSum += inter / union;
      }
      agreement = validCount === 1 ? 0.5 : Math.round((overlapSum / (validCount - 1)) * 100) / 100;
    }

    if (valid.length >= 2) {
      const first = valid[0].response.slice(0, 80);
      const last = valid[valid.length - 1].response.slice(0, 80);
      if (first !== last && agreement < this.config.consensusThreshold) {
        conflict.push({ between: [valid[0].expertName, valid[valid.length - 1].expertName], note: '立场存在分歧，已保留少数派意见' });
      }
    }

    return {
      validCount,
      total: results.length,
      agreement,
      score: agreement, // 契约统一别名：外部（API/MCP）一律读 consensus.score，修复历史 undefined
      conflict,
      consensusReached: agreement >= this.config.consensusThreshold || validCount <= 1
    };
  }

  _keywords(text) {
    // 中文 2~3 字滑窗 n-gram（与 expert-alliance._keywordsOf 一致）：
    // 原机械 [一-龥]{2,4} 匹配会把 "微服务架构" 切成 "微服务架"，共识启发式失效。
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

  // ===================== 阶段四：综合合成 =====================
  /**
   * 置信度加权综合：以各专家 confidence 为权重，驱动网关生成结构化最终报告。
   * 结构：{ synthesis, key_insights[], recommendations[], risks[], confidence, contributors[] }
   */
  async synthesize(question, deliberation, intent, context = {}) {
    const opinions = deliberation.final
      .filter(r => r.response && !r.error)
      .map(r => ({
        expert: r.expertName,
        type: r.expertType,
        confidence: r.confidence || 0.6,
        opinion: r.response.slice(0, 600)
      }));

    if (!this.gateway || !this.gateway.activeProvider) {
      // 无网关降级：直接拼接加权摘要
      const weighted = opinions
        .sort((a, b) => b.confidence - a.confidence)
        .map(o => `【${o.expert}｜置信度${o.confidence}】${o.opinion.slice(0, 200)}`)
        .join('\n');
      return {
        synthesis: '（AI 综合不可用，以下为专家意见加权拼接）\n' + weighted,
        key_insights: [],
        recommendations: [],
        risks: [],
        confidence: this._weightedConfidence(opinions),
        contributors: opinions.map(o => o.expert),
        ai_powered: false
      };
    }

    const prompt = `你是专家联盟首席分析师。基于以下多位专家的分析（已标注置信度），
为问题提供最终结构化综合报告。

问题：${question}
主意图：${intent.primary}（置信度 ${intent.confidence}）
${context.background ? '背景：' + context.background : ''}

专家意见（按置信度加权参考）：
${opinions.map(o => `[${o.expert}｜${o.type}｜置信度${o.confidence}] ${o.opinion}`).join('\n\n')}

返回严格 JSON：
{
  "synthesis": "综合结论（2-4段）",
  "key_insights": ["关键洞察1", "关键洞察2"],
  "recommendations": ["建议1", "建议2"],
  "risks": ["风险1"],
  "confidence": 0.0-1.0
}`;

    try {
      const resp = await this.gateway.chat({
        messages: [
          { role: 'system', content: '你是企业级专家联盟首席分析师，输出必须为中文严格 JSON，且综合须覆盖多数专家共识并保留少数派关键分歧。' },
          { role: 'user', content: prompt }
        ],
        model: this.config.chiefModel || undefined
      });
      const parsed = this._extractJSON(resp.content || resp);
      const finalConf = this._blendConfidence(parsed.confidence, this._weightedConfidence(opinions));
      return {
        synthesis: parsed.synthesis || '',
        key_insights: parsed.key_insights || [],
        recommendations: parsed.recommendations || [],
        risks: parsed.risks || [],
        confidence: finalConf,
        contributors: opinions.map(o => o.expert),
        ai_powered: true
      };
    } catch (e) {
      return {
        synthesis: '综合失败：' + e.message,
        key_insights: [],
        recommendations: [],
        risks: [],
        confidence: this._weightedConfidence(opinions),
        contributors: opinions.map(o => o.expert),
        ai_powered: false,
        error: e.message
      };
    }
  }

  _weightedConfidence(opinions) {
    if (opinions.length === 0) return 0.5;
    const sum = opinions.reduce((s, o) => s + (o.confidence || 0.6), 0);
    return Math.round((sum / opinions.length) * 100) / 100;
  }

  // 网关综合置信度与各专家平均置信度融合，避免单点虚高
  _blendConfidence(chief, weighted) {
    const c = typeof chief === 'number' ? chief : weighted;
    return Math.round(Math.min(1, (c * 0.6 + weighted * 0.4)) * 100) / 100;
  }

  // ===================== 阶段五：质量门禁 =====================
  /**
   * 返回 passed / level / reasons：
   *   - A 级：高置信 + 高共识
   *   - B 级：达标
   *   - C 级：降级但可用（触发重试建议）
   *   - D 级：不通过（需人工/重路由）
   */
  qualityGate(synthesis, deliberation, intent) {
    const reasons = [];
    const conf = synthesis.confidence || 0;
    const agreement = deliberation.consensus ? deliberation.consensus.agreement : 0.5;
    const validCount = deliberation.consensus ? deliberation.consensus.validCount : 0;

    if (conf < this.config.confidenceThreshold) {
      reasons.push(`综合置信度 ${conf} 低于门禁 ${this.config.confidenceThreshold}`);
    }
    if (validCount < 1) {
      reasons.push('无有效专家意见');
    }
    if (deliberation.consensus && !deliberation.consensus.consensusReached) {
      reasons.push(`专家共识度 ${agreement} 未达 ${this.config.consensusThreshold}，存在分歧`);
    }

    let level = 'D';
    let passed = false;
    if (reasons.length === 0) {
      level = conf >= 0.8 && agreement >= 0.7 ? 'A' : 'B';
      passed = true;
    } else if (conf >= this.config.confidenceThreshold * 0.8 && validCount >= 1) {
      level = 'C';
      passed = true; // 放行但标记需复核
    }

    return {
      passed,
      level,
      confidence: conf,
      agreement,
      valid_count: validCount,
      reasons,
      retry_suggested: level === 'C' || level === 'D'
    };
  }

  // ===================== 阶段六：反馈学习 =====================
  /**
   * 将本次意图 → 命中专家回写到先验，并更新专家 metrics（置信度/成功率）。
   * 接收外部反馈（用户点赞/采纳）时可传入 feedback { expertId, score }
   * G1：质量门禁通过的处理沉淀学习技能（alliance_learned_skills.json）
   * G5：先验落盘改原子写（tmp + rename，崩溃不产生半写文件）
   */
  learn(question, intent, team, deliberation, synthesis, gate = null, feedback = null) {
    // 1) 意图先验
    const prior = this.intentPriors[intent.primary] || { hits: {} };
    for (const m of team) {
      prior.hits[m.id] = (prior.hits[m.id] || 0) + 1;
    }
    this.intentPriors[intent.primary] = prior;
    atomicWriteJSON(path.join(DATA_DIR, 'alliance_intent_priors.json'), this.intentPriors);

    // 2) 学习技能沉淀（domain 纯函数去重强化 → 仓储原子持久化）
    const { records } = synthesizeSkills(
      { question, intent, team, deliberation, synthesis, gate },
      this.skillStore.all()
    );
    if (records.length > 0) this.skillStore.save(this.skillStore.all());

    // 3) 专家 metrics 回写
    if (this.alliance && this.alliance.recordConsultMetric) {
      for (const r of deliberation.final) {
        const adopted = feedback && feedback.expertId === r.expertId;
        this.alliance.recordConsultMetric(r.expertId, {
          success: !r.error,
          confidence: r.confidence || 0.6,
          duration: r.duration || 0,
          adopted: adopted || false
        });
      }
    }
  }

  /** 学习技能视图（按强化次数排序，供路由/组队先验参考） */
  getLearnedSkills(limit = 20) {
    return rankSkills(this.skillStore.all(), limit);
  }

  getSkillStats() {
    return this.skillStore.stats();
  }

  // ===================== 统一编排入口 =====================
  /**
   * 企业级联盟处理主入口。返回完整可观测 trace。
   * options: { teamSize, context:{background,constraints}, feedback, enableDebate }
   */
  async process(question, options = {}) {
    // 空问题快速失败：不再让全管线（实测 34s）跑在空输入上
    const q = String(question || '').trim();
    if (!q) {
      return {
        trace: { trace_id: 'n/a', question: '', stages: [], error: 'question 为空：请提供要咨询的问题' },
        success: false,
        error: 'question 为空：请提供要咨询的问题'
      };
    }
    const traceId = crypto.randomBytes(8).toString('hex');
    const t0 = Date.now();
    const trace = {
      trace_id: traceId,
      question: String(question).slice(0, 200),
      started_at: new Date().toISOString(),
      stages: []
    };

    // 阶段标记：at = 距起点的绝对时间点，duration_ms = 本阶段自身耗时（企业级审计需两者）
    let lastMark = t0;
    const mark = (name, data) => {
      const now = Date.now();
      trace.stages.push({ stage: name, at: now - t0, duration_ms: now - lastMark, ...data });
      lastMark = now;
    };

    try {
      // 阶段一
      const intent = this.classifyIntent(question);
      mark('intent', { primary: intent.primary, confidence: intent.confidence });
      trace.intent = intent;

      // 阶段二
      const teamPlan = this.composeTeam(question, intent, { teamSize: options.teamSize });
      mark('team', { size: teamPlan.team_size, synergy: teamPlan.total_synergy });
      trace.team = teamPlan;

      if (!teamPlan.team || teamPlan.team.length === 0) {
        trace.error = '组队失败：无可用专家';
        trace.completed_at = new Date().toISOString();
        appendTrace(trace);
        return this._wrap(trace, { success: false, error: trace.error });
      }

      // 阶段三~五（核心管线，mark 在真实阶段边界打点：审计耗时精确到阶段）
      const core = await this._runCore(question, intent, teamPlan, options, mark);
      let { teamPlan: finalTeamPlan, deliberation, synthesis, gate } = core;

      // G3 门禁 C 级重试闭环：retry_suggested 首次被真实消费——
      // 单次重路由组队（换血：排除首次团队）重跑管线，取门禁更优者
      let retry = null;
      if (gate.retry_suggested && gate.level === 'C' && options.disableRetry !== true) {
        const firstTeamIds = new Set(teamPlan.team.map(m => m.id));
        const retryPlan = this.composeTeam(question, intent, {
          teamSize: options.teamSize,
          excludeIds: firstTeamIds
        });
        if (retryPlan.team && retryPlan.team.length > 0) {
          const retryCore = await this._runCore(question, intent, retryPlan, options);
          retry = {
            attempted: true,
            team: retryPlan.team.map(m => m.id),
            gate_first: gate.level,
            gate_retry: retryCore.gate.level,
            adopted: this._gateRank(retryCore.gate) > this._gateRank(gate)
          };
          if (retry.adopted) {
            finalTeamPlan = retryPlan; deliberation = retryCore.deliberation;
            synthesis = retryCore.synthesis; gate = retryCore.gate;
          }
          mark('retry', retry);
        }
      }
      trace.team = finalTeamPlan;
      trace.deliberation = deliberation;
      trace.synthesis = synthesis;
      trace.gate = gate;
      trace.retry = retry;

      // 阶段六：反馈学习（默认执行，外部反馈可选）
      this.learn(question, intent, finalTeamPlan.team, deliberation, synthesis, gate, options.feedback || null);

      trace.success = true;
      trace.completed_at = new Date().toISOString();
      trace.total_duration_ms = Date.now() - t0;
      appendTrace(trace);

      return this._wrap(trace, {
        success: true,
        trace_id: traceId,
        intent,
        team: finalTeamPlan.team.map(m => ({ id: m.id, name: m.name, type: m.type })),
        consensus: deliberation.consensus,
        synthesis,
        gate,
        retry,
        total_duration_ms: Date.now() - t0
      });
    } catch (e) {
      trace.error = e.message;
      trace.success = false;
      trace.completed_at = new Date().toISOString();
      trace.total_duration_ms = Date.now() - t0;
      appendTrace(trace);
      return this._wrap(trace, { success: false, error: e.message });
    }
  }

  /** 阶段三~五核心管线（G3 重试复用同一管线，保证契约一致） */
  async _runCore(question, intent, teamPlan, options, mark = null) {
    const deliberation = await this.deliberate(
      question, teamPlan.team, options.context || {}, { enableDebate: options.enableDebate }
    );
    if (mark) mark('deliberate', {
      rounds: deliberation.rounds,
      valid: deliberation.consensus.validCount,
      degraded: deliberation.degraded ? deliberation.degraded.to : undefined
    });
    const synthesis = await this.synthesize(question, deliberation, intent, options.context || {});
    if (mark) mark('synthesize', { confidence: synthesis.confidence, ai: synthesis.ai_powered });
    const gate = this.qualityGate(synthesis, deliberation, intent);
    if (mark) mark('quality_gate', { level: gate.level, passed: gate.passed });
    return { teamPlan, deliberation, synthesis, gate };
  }

  /** 门禁级别序：A(3) > B(2) > C(1) > D(0)——重试采纳判定 */
  _gateRank(gate) {
    return { A: 3, B: 2, C: 1, D: 0 }[gate.level] || 0;
  }

  _wrap(trace, payload) {
    return Object.assign({ trace }, payload);
  }

  // ===================== Trace 审计查询（G2 审计闭环） =====================
  /**
   * 读取最近 trace（JSONL 尾部窗口）：limit 条倒序。
   * 窗口约束：最多读 TAIL_MAX_BYTES 字节（防大文件全量加载），
   * 首行可能被截断为半行，解析失败自动跳过。
   */
  queryTraces(limit = 20) {
    const traces = this._readTraceTail();
    return traces.slice(0, Math.max(1, Math.min(limit, 200)));
  }

  /** 按 trace_id 精确回查（企业级：任何一次咨询可完整回溯） */
  queryTrace(traceId) {
    if (!traceId) return null;
    return this._readTraceTail().find(t => t.trace_id === traceId) || null;
  }

  /** trace 聚合统计（可观测性：成功率/耗时/门禁级别/意图分布） */
  traceStats() {
    const traces = this._readTraceTail(500);
    if (traces.length === 0) {
      return { total_in_window: 0, note: '暂无轨迹（首次咨询后产生）' };
    }
    const ok = traces.filter(t => t.success);
    const durations = ok.map(t => t.total_duration_ms || 0).filter(d => d > 0);
    const levels = {}, intents = {};
    for (const t of traces) {
      const lv = t.gate ? t.gate.level : (t.success ? 'N/A' : 'ERR');
      levels[lv] = (levels[lv] || 0) + 1;
      const it = t.intent ? t.intent.primary : 'unknown';
      intents[it] = (intents[it] || 0) + 1;
    }
    return {
      total_in_window: traces.length,
      success_rate: Math.round((ok.length / traces.length) * 100) / 100,
      avg_duration_ms: durations.length ? Math.round(durations.reduce((a, b) => a + b, 0) / durations.length) : 0,
      p95_duration_ms: durations.length ? durations.sort((a, b) => a - b)[Math.floor(durations.length * 0.95)] || 0 : 0,
      gate_levels: levels,
      intent_distribution: intents,
      window: `最近 ${traces.length} 条`
    };
  }

  /** JSONL 尾部窗口读取（内部） */
  _readTraceTail(maxLines = 200) {
    try {
      const fp = path.join(DATA_DIR, TRACE_FILE);
      if (!fs.existsSync(fp)) return [];
      const TAIL_MAX_BYTES = 2 * 1024 * 1024; // 2MB 窗口上限
      const size = fs.statSync(fp).size;
      const readLen = Math.min(size, TAIL_MAX_BYTES);
      const fd = fs.openSync(fp, 'r');
      const buf = Buffer.alloc(readLen);
      fs.readSync(fd, buf, 0, readLen, size - readLen);
      fs.closeSync(fd);
      const lines = buf.toString('utf8').split('\n').filter(Boolean);
      // 首行可能为半行（窗口起点截断）：验证失败即丢弃
      const out = [];
      for (const line of lines) {
        try { out.push(JSON.parse(line)); } catch (_e) { /* 半行跳过 */ }
      }
      // 若发生截断且首行解析成功也可能是巧合完整行——窗口语义只保证"最近"
      return out.slice(-maxLines).reverse(); // 倒序：最新在前
    } catch (_e) {
      return [];
    }
  }

  _extractJSON(text) {
    if (!text) return {};
    const m = String(text).match(/\{[\s\S]*\}/);
    if (m) {
      try { return JSON.parse(m[0]); } catch { return {}; }
    }
    return {};
  }
}

// ===================== 单例管理 =====================
let instance = null;

function getAllianceEngine(deps = {}) {
  if (!instance) {
    const alliance = deps.alliance || require('./expert-alliance').getAlliance();
    const expertGraph = deps.expertGraph || (() => {
      try { return require('./expert-graph').getExpertGraph(alliance); } catch { return null; }
    })();
    const dispatcher = deps.dispatcher || (() => {
      try { return require('./expert-dispatcher').getDispatcher(alliance); } catch { return null; }
    })();
    const gateway = deps.gateway || (() => {
      try { return require('./llm-gateway').getGateway(); } catch { return null; }
    })();
    instance = new ExpertAllianceEngine({ alliance, expertGraph, dispatcher, gateway, options: deps.options || {} });
  }
  return instance;
}

module.exports = { ExpertAllianceEngine, getAllianceEngine };
