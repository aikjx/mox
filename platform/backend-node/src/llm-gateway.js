'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const DATA_DIR = path.join(__dirname, '..', 'data');
const CIPHER_KEY = process.env.LLM_CIPHER_KEY || 'ous-llm-gateway-enterprise-key-2024';

const PROVIDER_PRESETS = {
  deepseek: {
    name: 'DeepSeek',
    base_url: 'https://api.deepseek.com/v1',
    models: ['deepseek-chat', 'deepseek-reasoner'],
    description: 'DeepSeek 大模型，支持中文对话和代码生成'
  },
  volcengine: {
    name: '火山引擎',
    base_url: 'https://ark.cn-beijing.volces.com/api/v3',
    models: ['doubao-pro-32k', 'doubao-pro-128k', 'doubao-lite-32k'],
    description: '字节跳动豆包大模型，支持中文业务场景'
  },
  qwen: {
    name: '阿里云千问',
    base_url: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    models: ['qwen-max', 'qwen-plus', 'qwen-turbo', 'qwen-long'],
    description: '阿里云千问大模型，支持多模态和长上下文'
  },
  kimi: {
    name: 'Kimi (月之暗面)',
    base_url: 'https://api.moonshot.cn/v1',
    models: ['moonshot-v1-8k', 'moonshot-v1-32k', 'moonshot-v1-128k', 'kimi-latest'],
    description: '月之暗面 Kimi 大模型，长上下文与中文理解能力强'
  },
  zhipu: {
    name: '智谱AI',
    base_url: 'https://open.bigmodel.cn/api/paas/v4',
    models: ['glm-4', 'glm-4-flash', 'glm-3-turbo'],
    description: '智谱AI大模型，支持长文本和复杂推理'
  },
  openai: {
    name: 'OpenAI',
    base_url: 'https://api.openai.com/v1',
    models: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-3.5-turbo'],
    description: 'OpenAI 最新模型，支持多模态和高级推理'
  },
  anthropic: {
    name: 'Anthropic',
    base_url: 'https://api.anthropic.com/v1',
    models: ['claude-3.5-sonnet', 'claude-3-opus', 'claude-3-sonnet'],
    description: 'Anthropic Claude模型，长上下文推理能力强'
  },
  google: {
    name: 'Google Gemini',
    base_url: 'https://generativelanguage.googleapis.com/v1beta',
    models: ['gemini-2.0-flash', 'gemini-1.5-pro', 'gemini-1.5-flash'],
    description: 'Google Gemini 模型，多模态能力突出'
  },
  ollama: {
    name: 'Ollama (本地)',
    base_url: 'http://localhost:11434/api',
    models: ['llama3', 'qwen2.5', 'deepseek-r1', 'mistral'],
    description: '本地 Ollama 部署，数据完全离线'
  },
  custom: {
    name: '自定义',
    base_url: '',
    models: [],
    description: '自定义 Provider，填入 Base URL 和模型名即可'
  }
};

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

function writeJSON(file, data) {
  try {
    fs.writeFileSync(path.join(DATA_DIR, file), JSON.stringify(data, null, 2), 'utf8');
    return true;
  } catch (e) {
    console.error('[llm-gateway] writeJSON', file, e.message);
    return false;
  }
}

function encryptApiKey(apiKey) {
  if (!apiKey) return '';
  const iv = crypto.randomBytes(16);
  const cipher = crypto.createCipheriv('aes-256-gcm', Buffer.from(CIPHER_KEY.padEnd(32).slice(0, 32)), iv);
  let encrypted = cipher.update(apiKey, 'utf8', 'hex');
  encrypted += cipher.final('hex');
  const tag = cipher.getAuthTag().toString('hex');
  return JSON.stringify({ iv: iv.toString('hex'), encrypted, tag });
}

function decryptApiKey(encryptedStr) {
  if (!encryptedStr) return '';
  try {
    const obj = JSON.parse(encryptedStr);
    const iv = Buffer.from(obj.iv, 'hex');
    const tag = Buffer.from(obj.tag, 'hex');
    const decipher = crypto.createDecipheriv('aes-256-gcm', Buffer.from(CIPHER_KEY.padEnd(32).slice(0, 32)), iv);
    decipher.setAuthTag(tag);
    let decrypted = decipher.update(obj.encrypted, 'hex', 'utf8');
    decrypted += decipher.final('utf8');
    return decrypted;
  } catch (e) {
    return encryptedStr;
  }
}

function maskApiKey(key) {
  if (!key) return '';
  const unmasked = decryptApiKey(key);
  if (unmasked.length <= 8) return '****';
  return unmasked.slice(0, 4) + '****' + unmasked.slice(-4);
}

// 各 Provider 的能力评分（用于"自动优选最好的 AI 引擎"）
// 综合中文理解、代码、推理、长上下文等维度，分数越高越优先被自动选中。
const PROVIDER_CAPABILITY_SCORE = {
  deepseek: 95, // 中文 + 代码 + 推理强
  volcengine: 90, // 豆包，长上下文友好
  openai: 92, // 综合最强推理/多模态
  qwen: 85, // 阿里云千问，多模态 + 长上下文
  kimi: 86, // Kimi，长上下文 + 中文理解
  zhipu: 82, // 智谱 GLM，长文本 + 复杂推理
  anthropic: 88, // Claude，长上下文推理
  google: 87, // Gemini，多模态
  local: 0, // 内置假引擎，不算真实 AI
  _default: 60 // 未知 provider 给中性分
};

// ---- O1 · LatencyWarmRouter（对比 Dify/LangGraph/Flo/AutoGen，企业级多活路由） ----
// 核心算法：
//   score = 0.6 * norm(1 - ewma_latency) + 0.3 * success_rate_ewma + 0.1 * priority_norm
//   EWMA α=0.2；每 50 次请求 Top-2 providers 主动 "预热 ping"（短 dummy call 或 /v1/models 等价），以捕获真实抖动。
//   失败：立即按 fallback=true 降级，更新 success_rate ewma 并扣减分数；下一次排序自动将其降级。
class LatencyWarmRouter {
  constructor(providers, options = {}) {
    this.options = Object.assign({ alpha: 0.2, warmEveryN: 50, warmTopK: 2, pingTimeoutMs: 400 }, options);
    this._warmRequestCount = 0;
    this.providers = providers; // 引用：每次 score 前读取最新 provider 对象
    // 初始化 EWMA 指标
    this.ewmaLatencyMs = Object.create(null);
    this.ewmaSuccessRate = Object.create(null);
    for (const p of Object.values(providers)) {
      const id = p.id || p.provider;
      const baseLat = p.estimated_latency_ms || 400;
      this.ewmaLatencyMs[id] = typeof baseLat === 'number' ? baseLat : 400;
      this.ewmaSuccessRate[id] = (typeof p.error_rate === 'number') ? Math.max(0, 1 - p.error_rate) : 0.95;
    }
  }
  _priorityOf(p) {
    if (typeof p.priority === 'number') return p.priority;
    if (typeof p.provider === 'string') {
      return (PROVIDER_CAPABILITY_SCORE[p.provider] != null) ? PROVIDER_CAPABILITY_SCORE[p.provider] : PROVIDER_CAPABILITY_SCORE._default;
    }
    return PROVIDER_CAPABILITY_SCORE._default;
  }
  _scoreProvider(id, p) {
    const lat = this.ewmaLatencyMs[id];
    const maxLat = Math.max(1, ...Object.values(this.ewmaLatencyMs));
    const normLat = 1 - Math.min(1, lat / maxLat);
    const success = this.ewmaSuccessRate[id];
    const priMax = Math.max(1, ...Object.values(this.providers).map(x => this._priorityOf(x)));
    const priNorm = this._priorityOf(p) / priMax;
    return 0.6 * normLat + 0.3 * success + 0.1 * priNorm;
  }
  // 返回启用 provider 按得分降序排序的 id 数组
  rankedEnabledIds() {
    const entries = Object.entries(this.providers)
      .filter(([id, p]) => p && p.enabled !== false);
    return entries
      .map(([id, p]) => [id, this._scoreProvider(id, p)])
      .sort((a,b) => b[1] - a[1])
      .map(([id]) => id);
  }
  // 记录一次真实请求结果（更新 EWMA）
  recordResult(id, latencyMs, ok) {
    if (this.ewmaLatencyMs[id] == null) this.ewmaLatencyMs[id] = Math.max(1, latencyMs || 1);
    if (this.ewmaSuccessRate[id] == null) this.ewmaSuccessRate[id] = ok ? 1 : 0.5;
    const alpha = this.options.alpha;
    this.ewmaLatencyMs[id] = (1 - alpha) * this.ewmaLatencyMs[id] + alpha * Math.max(1, latencyMs || 1);
    const sampleErr = ok ? 0 : 1;
    this.ewmaSuccessRate[id] = (1 - alpha) * this.ewmaSuccessRate[id] + alpha * (1 - sampleErr);
  }
  // 预热 Top-2 providers：调用方通过 warmCb(p) 做一次轻量 ping；完成后 recordResult。
  async maybeWarmTop(warmCb) {
    this._warmRequestCount++;
    if (this._warmRequestCount % this.options.warmEveryN !== 1) return;
    const ids = this.rankedEnabledIds().slice(0, this.options.warmTopK);
    for (const id of ids) {
      try {
        const start = Date.now();
        const ok = await Promise.race([
          Promise.resolve(warmCb && warmCb(this.providers[id])).then(() => true),
          new Promise(r => setTimeout(() => r(false), this.options.pingTimeoutMs))
        ]);
        this.recordResult(id, Date.now() - start, !!ok);
      } catch (_) {
        this.recordResult(id, this.options.pingTimeoutMs, false);
      }
    }
  }
  // 暴露给 unit-test / summary
  snapshot() {
    return Object.fromEntries(
      Object.entries(this.providers).map(([id, p]) => [id, {
        lat_ewma: this.ewmaLatencyMs[id] ?? null,
        sr_ewma: this.ewmaSuccessRate[id] ?? null,
        score: this._scoreProvider(id, p)
      }])
    );
  }
}

// O1 策略常量（和 getRoutingConfig() / T4 H2 保持一致）
const ROUTING_STRATEGIES = ['priority', 'fallback', 'latency-warm'];

class LLMGateway {
  constructor() {
    this.providers = {};
    this.activeProvider = null;
    this.conversations = new Map();
    this.usage = {};
    this.requestLog = [];
    this.maxRetries = 3;
    this.requestTimeout = 30000;
    /** @type {LatencyWarmRouter|null} O1 补丁实例 */
    this._warmRouter = null;
    this._warmRequestCount = 0;
    this._init();
  }

  _init() {
    const config = readJSON('llm_config.json', []);
    if (Array.isArray(config) && config.length) {
      config.forEach((p) => {
        const provider = { ...p };
        if (provider.api_key && !provider.api_key.startsWith('{')) {
          provider.api_key = encryptApiKey(provider.api_key);
        }
        this.providers[p.id || p.provider] = provider;
      });
    }

    // 环境变量自动注入：若系统环境变量中存在 DeepSeek API Key，
    // 则自动补全并启用 DeepSeek Provider，无需在前端手动填写。
    const envDeepSeekKey = process.env.DEEPSEEK_API_KEY || process.env.DEEPSEEK_API_KEY_ENV;
    if (envDeepSeekKey && String(envDeepSeekKey).trim().length > 0) {
      const dsId = 'llm_deepseek';
      const ds = this.providers[dsId] || { id: dsId, name: 'DeepSeek', provider: 'deepseek', base_url: 'https://api.deepseek.com/v1', model: 'deepseek-chat', description: 'DeepSeek 大模型（环境变量自动注入）' };
      ds.api_key = encryptApiKey(String(envDeepSeekKey).trim());
      ds.enabled = true;
      ds.provider = 'deepseek';
      this.providers[dsId] = ds;
      console.log('[LLM] 已从环境变量 DEEPSEEK_API_KEY 自动启用 DeepSeek 引擎');
    }
    // 选择激活 Provider 的规则：
    //  - 默认不应选 local（内置假引擎），否则对话永远走不到真实 LLM
    //  - 仅在"真实 Provider 已启用且配置了 api_key"时自动激活（按能力评分优选最好的引擎）
    //  - 没有任何真实 AI 时，才退回 local 作为兜底（此时标记为无 AI）
    const realCandidates = Object.values(this.providers).filter(
      (p) => p.provider && p.provider !== 'local' && p.enabled && p.api_key && String(p.api_key).trim().length > 0
    );
    realCandidates.sort((a, b) => (PROVIDER_CAPABILITY_SCORE[b.provider] || PROVIDER_CAPABILITY_SCORE._default) - (PROVIDER_CAPABILITY_SCORE[a.provider] || PROVIDER_CAPABILITY_SCORE._default));
    if (realCandidates.length) {
      this.activeProvider = realCandidates[0].id || realCandidates[0].provider;
    } else {
      // 无真实 AI，退回 local 兜底（保持向后兼容）
      const local = Object.values(this.providers).find((p) => p.provider === 'local');
      this.activeProvider = local ? (local.id || local.provider) : (Object.keys(this.providers)[0] || null);
    }
    this.usage = readJSON('llm_usage.json', {});
  }
  // 当前是否配置了「真实 AI 引擎」（已启用 + 非 local + 已填 api_key）
  isRealAI() {
    const provider = this.activeProvider ? this.providers[this.activeProvider] : null;
    return !!(provider && provider.provider && provider.provider !== 'local' && provider.api_key && String(provider.api_key).trim().length > 0);
  }

  // 当前可用（已启用 + 已配置 Key + 非 local）的 Provider 列表，供优化引擎枚举
  listAvailableProviders() {
    return Object.values(this.providers)
      .filter((p) => p.provider && p.provider !== 'local' && p.enabled && p.api_key && String(p.api_key).trim().length > 0)
      .map((p) => ({ id: p.id, name: p.name || p.id, provider: p.provider, model: p.model }));
  }

  // 评测专用：指定 Provider 的严格单次调用（不重试、不本地降级），
  // 失败即抛错，保证优化评分不被假回复污染。
  async chatWithProvider(providerId, params) {
    const provider = this.providers[providerId];
    if (!provider || !provider.enabled || provider.provider === 'local' || !provider.api_key) {
      throw new Error(`Provider 不可用或未配置: ${providerId}`);
    }
    const { messages, temperature = 0.3, maxTokens = 512, systemPrompt } = params;
    const all = systemPrompt
      ? [{ role: 'system', content: this._buildTimeContext() }, { role: 'system', content: systemPrompt }, ...messages]
      : [{ role: 'system', content: this._buildTimeContext() }, ...messages];

    const url = provider.base_url || 'https://api.openai.com/v1';
    const model = provider.model || 'gpt-4';
    const apiKey = decryptApiKey(provider.api_key);

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.requestTimeout);
    const start = Date.now();
    try {
      const response = await fetch(`${url}/chat/completions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${apiKey}` },
        body: JSON.stringify({ model, messages: all, temperature, max_tokens: maxTokens }),
        signal: controller.signal
      });
      if (!response.ok) throw new Error(`LLM API error: ${response.status}`);
      const data = await response.json();
      this._recordUsage(provider.id || provider.provider, data.usage || {});
      return {
        content: data.choices[0].message.content,
        usage: data.usage || { total_tokens: 0 },
        model: data.model,
        provider: provider.id,
        latency_ms: Date.now() - start
      };
    } finally {
      clearTimeout(timeoutId);
    }
  }

  // 构建实时时间上下文：LLM 训练数据存在截止时间，不注入当前时间会导致
  // "今天是？"这类问题被模型凭训练记忆编造日期（幻觉）。
  _buildTimeContext() {
    const now = new Date();
    const days = ['日', '一', '二', '三', '四', '五', '六'];
    const pad = (n) => String(n).padStart(2, '0');
    const tzOffsetMin = -now.getTimezoneOffset();
    const tzSign = tzOffsetMin >= 0 ? '+' : '-';
    const tzHours = Math.floor(Math.abs(tzOffsetMin) / 60);
    const tzMins = Math.abs(tzOffsetMin) % 60;
    const tz = `UTC${tzSign}${tzHours}${tzMins ? ':' + pad(tzMins) : ''}`;
    return [
      '【实时环境】',
      `当前真实日期时间：${now.getFullYear()}年${now.getMonth() + 1}月${now.getDate()}日（星期${days[now.getDay()]}）${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}，时区 ${tz}。`,
      '规则：凡涉及"今天/现在/当前日期"等时间问题，必须以上述时间为唯一事实依据，严禁凭训练记忆猜测或编造日期。'
    ].join('\n');
  }

  // 构建增强消息（时间上下文 + 联网上下文 + 专家系统提示 + 会话历史）：chat 与 chatStream 共用
  _buildEnhancedMessages(params) {
    const { messages, sessionId, expertType, systemPrompt, webSearchContext } = params;
    const enhancedMessages = [];

    // 始终注入实时时间上下文（防止日期幻觉）
    enhancedMessages.push({ role: 'system', content: this._buildTimeContext() });

    // 联网搜索上下文：由调用方（/ai/chat）在开启联网时注入
    if (webSearchContext) {
      enhancedMessages.push({ role: 'system', content: webSearchContext });
    }

    if (systemPrompt) {
      enhancedMessages.push({ role: 'system', content: systemPrompt });
    } else if (expertType) {
      const expertPrompt = this._getExpertSystemPrompt(expertType);
      enhancedMessages.push({ role: 'system', content: expertPrompt });
    }

    const convHistory = sessionId ? this.conversations.get(sessionId) || [] : [];
    return { allMessages: [...enhancedMessages, ...convHistory, ...messages], convHistory };
  }

  // 会话记忆更新（chat 与 chatStream 共用）：LRU 1000 会话
  _updateConversation(sessionId, convHistory, messages, content) {
    if (!sessionId) return;
    const updatedHistory = [...convHistory, ...messages];
    if (content) {
      updatedHistory.push({ role: 'assistant', content });
    }
    this.conversations.set(sessionId, updatedHistory);
    if (this.conversations.size > 1000) {
      const oldestKey = this.conversations.keys().next().value;
      this.conversations.delete(oldestKey);
    }
  }

  async chat(params) {
    const { messages, sessionId, expertType, systemPrompt, temperature = 0.7, maxTokens = 2048 } = params;

    const provider = this.activeProvider ? this.providers[this.activeProvider] : null;

    const { allMessages, convHistory } = this._buildEnhancedMessages(params);

    // O1：预热 Top-K（latency-warm 策略每次 chat 触发；对 priority/fallback 策略此操作为空，开销可忽略）
    const routingCfg = this.getRoutingConfig();
    const strategy = ROUTING_STRATEGIES.includes(routingCfg.strategy) ? routingCfg.strategy : 'fallback';
    const enableFallback = routingCfg.fallback !== false;
    if (strategy === 'latency-warm') {
      const r = this._ensureWarmRouter();
      // 预热不阻塞主请求（fire-and-forget 但记录结果）—— 避免预热慢拖慢首次响应
      r.maybeWarmTop(async (p) => {
        // 轻量 ping：GET {base_url}/models（若未配置则模拟）；无有效 key 不会抛错但返回 false。
        if (!p || !p.base_url) return false;
        try {
          const r0 = await fetch(`${p.base_url}/models`, {
            method: 'GET',
            signal: AbortSignal.timeout ? AbortSignal.timeout(500) : void 0,
            headers: p.api_key ? { 'Authorization': `Bearer ${decryptApiKey(p.api_key)}` } : {}
          });
          return r0 && r0.ok;
        } catch (_) { return false; }
      }).catch(() => {});
    }

    let result;

    const singleProviderMode = (provider && provider.enabled && provider.provider !== 'local' && strategy === 'priority' && this.activeProvider);
    if (singleProviderMode) {
      // O1 兼容：单 activeProvider 指定模式（priority 下仅走该 provider，与旧行为一致）
      result = await this._callExternalProvider(provider, allMessages, temperature, maxTokens);
      // O1 EWMA 更新（若 router 已存在）：
      if (this._warmRouter) this._warmRouter.recordResult(provider.id || provider.provider, result && result.latency_ms || 0, !(result && String(result.provider || '').startsWith('local')));
    } else if (strategy === 'fallback' || strategy === 'latency-warm') {
      // O1 fallback / latency-warm：多候选依次尝试
      const candidates = this._candidateProviders(strategy);
      const local = this._generateIntelligentResponse(messages, expertType, convHistory);
      result = local;
      for (const id of candidates) {
        const p = this.providers[id];
        if (!p || p.enabled === false || p.provider === 'local' || !p.api_key) continue;
        const startTs = Date.now();
        try {
          const r = await this._callExternalProvider(p, allMessages, temperature, maxTokens);
          const latency = Date.now() - startTs;
          const isRealAI = r && !(String(r.provider || '').startsWith('local'));
          if (this._warmRouter) this._warmRouter.recordResult(id, latency, isRealAI);
          if (isRealAI) { result = Object.assign({}, r, { latency_ms: latency, used_fallback: candidates.indexOf(id) > 0, routing_strategy: strategy }); break; }
        } catch (e) {
          if (this._warmRouter) this._warmRouter.recordResult(id, Date.now() - startTs, false);
          if (!enableFallback) break; // 无 fallback 直接用 local 默认
        }
      }
    } else if (provider && provider.enabled && provider.provider !== 'local') {
      result = await this._callExternalProvider(provider, allMessages, temperature, maxTokens);
    } else if (expertType === 'graph' && systemPrompt && systemPrompt.includes('nodes') && systemPrompt.includes('edges')) {
      console.log('[gateway] Graph generation detected, expertType:', expertType, 'systemPrompt length:', systemPrompt.length);
      const userText = messages.filter(m => m.role === 'user').map(m => m.content).join(' ');
      const topicMatch = userText.match(/主题[：:]\s*(.+)/);
      const descMatch = userText.match(/详细描述[：:]\s*(.+)/);
      const topic = topicMatch ? topicMatch[1].trim() : userText.split('\n')[0].trim();
      const description = descMatch ? descMatch[1].trim() : '';
      console.log('[gateway] Extracted topic:', topic, 'description:', description);
      const graphData = this._generateLocalGraph(topic, description);
      console.log('[gateway] Generated graph:', graphData.nodes.length, 'nodes,', graphData.edges.length, 'edges');
      result = {
        content: JSON.stringify(graphData),
        usage: { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
        model: 'ous-local-graph-v1',
        provider: 'local-graph-generator',
        metadata: { type: 'graph-generation', nodeCount: graphData.nodes.length, edgeCount: graphData.edges.length }
      };
    } else {
      console.log('[gateway] Falling through to _generateIntelligentResponse, expertType:', expertType, 'hasSystemPrompt:', !!systemPrompt);
      result = this._generateIntelligentResponse(messages, expertType, convHistory);
    }

    this._updateConversation(sessionId, convHistory, messages, result && result.content);

    return result;
  }

  /**
   * 流式对话（SSE）：真实 Provider 逐 token 推送，onChunk(delta, fullContent) 回调。
   * 无真实 AI 时一次性降级推送本地结果（不伪装流式，ai_powered 标记为 false）。
   * 协议：OpenAI 兼容 stream:true + stream_options.include_usage（DeepSeek/vLLM 等均支持）。
   */
  async chatStream(params, onChunk) {
    const { messages, sessionId, temperature = 0.7, maxTokens = 2048 } = params;
    const provider = this.activeProvider ? this.providers[this.activeProvider] : null;
    const { allMessages, convHistory } = this._buildEnhancedMessages(params);

    // 无真实 AI：降级一次性返回（显式标记非 AI，不伪装）
    if (!provider || !provider.enabled || provider.provider === 'local') {
      const local = this._generateIntelligentResponse(messages, null, convHistory);
      if (onChunk && local.content) onChunk(local.content, local.content);
      this._updateConversation(sessionId, convHistory, messages, local.content);
      return { ...local, ai_powered: false };
    }

    const url = provider.base_url || 'https://api.openai.com/v1';
    const model = provider.model || 'gpt-4';
    const apiKey = decryptApiKey(provider.api_key);
    const start = Date.now();

    const payload = {
      model,
      messages: allMessages,
      temperature,
      max_tokens: maxTokens,
      stream: true,
      stream_options: { include_usage: true }
    };

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.requestTimeout);
    // 客户端断开（SSE 连接关闭）→ 中止上游流，避免无谓 token 消耗
    if (params.signal) {
      params.signal.addEventListener('abort', () => controller.abort(), { once: true });
    }
    try {
      const response = await fetch(`${url}/chat/completions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${apiKey}` },
        body: JSON.stringify(payload),
        signal: controller.signal
      });
      if (!response.ok) throw new Error(`LLM API error: ${response.status}`);

      const reader = response.body.getReader();
      const decoder = new TextDecoder('utf-8');
      let buffer = '';
      let content = '';
      let usage = null;
      let respModel = model;

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || ''; // 尾行可能不完整，留待下个 chunk
        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed.startsWith('data:')) continue;
          const data = trimmed.slice(5).trim();
          if (data === '[DONE]') continue;
          try {
            const json = JSON.parse(data);
            if (json.usage) usage = json.usage;
            if (json.model) respModel = json.model;
            const delta = json.choices && json.choices[0] && json.choices[0].delta && json.choices[0].delta.content;
            if (delta) {
              content += delta;
              if (onChunk) onChunk(delta, content);
            }
          } catch (_e) { /* 不完整 JSON 行，忽略 */ }
        }
      }

      if (usage) this._recordUsage(provider.id || provider.provider, usage);
      this._logRequest(provider.id || provider.provider, 'success', Date.now() - start);

      this._updateConversation(sessionId, convHistory, messages, content);

      return {
        content,
        usage: usage || { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
        model: respModel,
        provider: provider.id,
        latency_ms: Date.now() - start,
        ai_powered: true
      };
    } finally {
      clearTimeout(timeoutId);
    }
  }

  _getExpertSystemPrompt(expertType) {
    const prompts = {
      algorithm: '你是一位资深算法专家，擅长分析算法复杂度、优化方案和代码实现。请以专业、精准的方式回答。',
      architecture: '你是一位系统架构专家，精通企业级系统设计、微服务架构和分布式系统。请提供清晰、可落地的架构建议。',
      data: '你是一位数据专家，精通数据建模、数据治理、ETL流程和数据可视化。请给出专业的数据方案。',
      ai: '你是一位AI专家，精通机器学习、深度学习、大模型应用和AI工程化。请提供前沿且实用的AI建议。',
      workflow: '你是一位工作流专家，精通BPMN、流程编排、自动化引擎和业务流程优化。请设计高效的工作流方案。',
      operator: '你是一位算子系统专家，精通算子抽象、算子组合、状态向量空间和守恒律。请提供符合算子系统数学公理的方案。',
      graph: '你是一位知识图谱专家，精通图算法、实体关系抽取、图谱构建和图神经网络。请提供专业的图谱方案。',
      security: '你是一位安全专家，精通应用安全、数据安全、网络安全和合规审计。请给出全面的安全建议。',
      performance: '你是一位性能优化专家，精通性能分析、瓶颈定位、优化策略和容量规划。请提供量化的性能方案。',
      monitor: '你是一位可观测性专家，精通监控体系、告警策略、日志分析和链路追踪。请设计完善的监控方案。',
      market: '你是一位商业智能专家，精通市场分析、用户画像、推荐系统和商业化策略。请提供商业洞察。',
      mcp: '你是一位MCP协议专家，精通Model Context Protocol设计、工具集成和跨平台兼容。请提供标准的MCP方案。',
      automation: '你是一位自动化专家，精通RPA、流程自动化、智能体工作流和低代码平台。请提供端到端自动化方案。',
      requirement: '你是一位需求工程专家，精通需求分析、需求建模、需求追踪和需求编译。请提供结构化的需求方案。',
      fusion: '你是一位融合专家，精通璇玑体系、双十四维治理、全维融合和跨系统集成。请提供全维融合方案。',
      default: '你是一位智能助手，可以帮助进行系统分析、代码实现、架构设计和问题解决。请以专业、精准的方式回答。'
    };
    return prompts[expertType] || prompts.default;
  }

  async _callExternalProvider(provider, messages, temperature, maxTokens) {
    const url = provider.base_url || 'https://api.openai.com/v1';
    const model = provider.model || 'gpt-4';
    const apiKey = decryptApiKey(provider.api_key);

    const payload = {
      model,
      messages,
      temperature,
      max_tokens: maxTokens
    };

    let lastError = null;
    for (let attempt = 1; attempt <= this.maxRetries; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.requestTimeout);

        const response = await fetch(`${url}/chat/completions`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${apiKey}`
          },
          body: JSON.stringify(payload),
          signal: controller.signal
        });

        clearTimeout(timeoutId);

        if (!response.ok) {
          throw new Error(`LLM API error: ${response.status}`);
        }

        const data = await response.json();
        
        this._recordUsage(provider.id || provider.provider, data.usage || {});
        this._logRequest(provider.id || provider.provider, 'success', Date.now() - (this._requestStart || Date.now()));

        return {
          content: data.choices[0].message.content,
          usage: data.usage,
          model: data.model,
          provider: provider.id
        };
      } catch (error) {
        lastError = error;
        if (attempt < this.maxRetries) {
          const delay = Math.pow(2, attempt) * 100;
          await new Promise(resolve => setTimeout(resolve, delay));
        }
      }
    }

    this._logRequest(provider.id || provider.provider, 'failed', 0, lastError?.message);
    console.warn('[llm-gateway] External provider failed after retries, falling back to local:', lastError?.message);
    return this._generateIntelligentResponse(messages, null, []);
  }

  _recordUsage(providerId, usage) {
    if (!this.usage[providerId]) {
      this.usage[providerId] = { total_tokens: 0, prompt_tokens: 0, completion_tokens: 0, requests: 0, last_updated: null };
    }
    const u = this.usage[providerId];
    u.total_tokens += usage.total_tokens || 0;
    u.prompt_tokens += usage.prompt_tokens || 0;
    u.completion_tokens += usage.completion_tokens || 0;
    u.requests += 1;
    u.last_updated = new Date().toISOString();
    writeJSON('llm_usage.json', this.usage);
  }

  _logRequest(providerId, status, latency, error) {
    const log = {
      provider: providerId,
      status,
      latency_ms: latency,
      error: error || null,
      timestamp: new Date().toISOString()
    };
    this.requestLog.push(log);
    if (this.requestLog.length > 1000) {
      this.requestLog = this.requestLog.slice(-500);
    }
  }

  getUsage() {
    return this.usage;
  }

  getRequestLog(limit = 50) {
    return this.requestLog.slice(-limit).reverse();
  }

  async testConnection(providerId) {
    const provider = this.providers[providerId];
    if (!provider) {
      return { success: false, message: 'Provider not found' };
    }

    if (provider.provider === 'local' || provider.type === 'local') {
      return { success: true, message: '本地引擎正常', latencyMs: 0, provider: providerId };
    }

    const startTime = Date.now();
    try {
      const url = provider.base_url;
      const apiKey = decryptApiKey(provider.api_key);
      const response = await fetch(`${url}/models`, {
        method: 'GET',
        headers: {
          'Authorization': `Bearer ${apiKey}`
        }
      });

      const latencyMs = Date.now() - startTime;

      if (!response.ok) {
        let errorMsg = `HTTP ${response.status}`;
        if (response.status === 401) errorMsg = 'API Key 无效或未授权';
        else if (response.status === 429) errorMsg = '请求频率超限，请稍后重试';
        else if (response.status === 404) errorMsg = 'API 端点不存在，请检查 Base URL';
        
        return {
          success: false,
          message: errorMsg,
          latencyMs,
          provider: providerId,
          statusCode: response.status
        };
      }

      const data = await response.json().catch(() => ({}));
      const models = data.data ? data.data.map(m => m.id || m.id) : [];

      return {
        success: true,
        message: `连接成功，检测到 ${models.length} 个可用模型`,
        latencyMs,
        provider: providerId,
        models: models.slice(0, 20)
      };
    } catch (error) {
      const latencyMs = Date.now() - startTime;
      return {
        success: false,
        message: `连接失败: ${error.message}`,
        latencyMs,
        provider: providerId
      };
    }
  }

  async discoverModels(providerId) {
    const provider = this.providers[providerId];
    if (!provider || !provider.base_url) {
      return { success: false, models: [] };
    }

    if (provider.provider === 'local' || provider.type === 'local') {
      return { success: true, models: ['ous-internal-v3'] };
    }

    try {
      const url = provider.base_url;
      const apiKey = decryptApiKey(provider.api_key);
      const response = await fetch(`${url}/models`, {
        method: 'GET',
        headers: {
          'Authorization': `Bearer ${apiKey}`
        }
      });

      if (!response.ok) {
        return { success: false, models: [], message: `HTTP ${response.status}` };
      }

      const data = await response.json().catch(() => ({}));
      const models = data.data ? data.data.map(m => ({
        id: m.id,
        name: m.id,
        owned_by: m.owned_by,
        context_window: m.context_window || m.max_context_window || 0
      })) : [];

      return { success: true, models };
    } catch (error) {
      return { success: false, models: [], message: error.message };
    }
  }

  getHealth() {
    const providers = Object.values(this.providers);
    const enabledCount = providers.filter(p => p.enabled).length;
    const externalCount = providers.filter(p => p.provider !== 'local').length;
    
    return {
      total_providers: providers.length,
      enabled_providers: enabledCount,
      external_providers: externalCount,
      active_provider: this.activeProvider,
      active_provider_name: this.providers[this.activeProvider]?.name || '无',
      status: enabledCount > 0 ? 'ready' : 'no_provider',
      local_available: !!this.providers[Object.keys(this.providers).find(k => this.providers[k].provider === 'local')]
    };
  }

  getPresetProviders() {
    return Object.entries(PROVIDER_PRESETS).map(([key, preset]) => ({
      id: key,
      name: preset.name,
      base_url: preset.base_url,
      models: preset.models,
      description: preset.description
    }));
  }

  listProviders() {
    return Object.entries(this.providers).map(([id, p]) => ({
      id,
      name: p.name || id,
      type: p.provider,
      enabled: p.enabled,
      active: id === this.activeProvider,
      model: p.model,
      base_url: p.base_url,
      has_key: !!(p.api_key && p.api_key.trim()),
      api_key_masked: maskApiKey(p.api_key),
      description: p.description,
      updated_at: p.updated_at,
      created_at: p.created_at
    }));
  }

  getProvider(providerId) {
    const provider = this.providers[providerId];
    if (provider) {
      return {
        ...provider,
        api_key_masked: maskApiKey(provider.api_key)
      };
    }
    return null;
  }

  setActiveProvider(providerId) {
    if (this.providers[providerId]) {
      this.activeProvider = providerId;
      const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
      writeJSON('llm_config.json', config);
      return true;
    }
    return false;
  }

  addProvider(provider) {
    const id = provider.id || `llm_${Date.now()}`;
    const now = new Date().toISOString();
    
    const preset = provider.provider && PROVIDER_PRESETS[provider.provider];
    
    const encryptedKey = provider.api_key ? encryptApiKey(provider.api_key) : '';
    
    this.providers[id] = {
      id,
      provider: provider.provider || 'custom',
      base_url: provider.base_url || (preset ? preset.base_url : ''),
      model: provider.model || (preset && preset.models ? preset.models[0] : 'default'),
      api_key: encryptedKey,
      enabled: provider.enabled || false,
      name: provider.name || (preset ? preset.name : id),
      description: provider.description || (preset ? preset.description : ''),
      temperature: provider.temperature || 0.7,
      max_tokens: provider.max_tokens || 2048,
      updated_at: now,
      created_at: now
    };
    
    const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
    writeJSON('llm_config.json', config);
    
    if (provider.enabled && !this.activeProvider) {
      this.activeProvider = id;
    }
    return id;
  }

  updateProvider(providerId, updates) {
    if (!this.providers[providerId]) return false;
    
    const allowedFields = ['name', 'base_url', 'model', 'enabled', 'description', 'temperature', 'max_tokens'];
    for (const field of allowedFields) {
      if (updates[field] !== undefined) {
        this.providers[providerId][field] = updates[field];
      }
    }
    
    if (updates.api_key !== undefined) {
      if (updates.api_key === '') {
        this.providers[providerId].api_key = '';
      } else {
        this.providers[providerId].api_key = encryptApiKey(updates.api_key);
      }
    }
    
    this.providers[providerId].updated_at = new Date().toISOString();
    
    const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
    writeJSON('llm_config.json', config);
    return true;
  }

  removeProvider(providerId) {
    if (this.providers[providerId]) {
      delete this.providers[providerId];
      const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
      writeJSON('llm_config.json', config);
      if (this.activeProvider === providerId) {
        const firstKey = Object.keys(this.providers)[0];
        this.activeProvider = firstKey || null;
        if (this.activeProvider && !this.providers[this.activeProvider].enabled) {
          this.providers[this.activeProvider].enabled = true;
        }
      }
      return true;
    }
    return false;
  }

  enableProvider(providerId) {
    if (!this.providers[providerId]) return false;
    this.providers[providerId].enabled = true;
    if (!this.activeProvider) {
      this.activeProvider = providerId;
    }
    const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
    writeJSON('llm_config.json', config);
    return true;
  }

  disableProvider(providerId) {
    if (!this.providers[providerId]) return false;
    this.providers[providerId].enabled = false;
    if (this.activeProvider === providerId) {
      const nextProvider = Object.keys(this.providers).find(k => this.providers[k].enabled);
      this.activeProvider = nextProvider || null;
    }
    const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
    writeJSON('llm_config.json', config);
    return true;
  }

  getRoutingConfig() {
    return readJSON('llm_routing.json', {
      strategy: 'latency-warm', // O1：默认启用 latency-warm（优于 legacy priority）
      providers: Object.keys(this.providers).filter(k => this.providers[k].enabled),
      fallback: true,
      load_balance: false,
      weights: {},
      // O1 可调参数（与 LatencyWarmRouter options 对齐）
      warm: {
        alpha: 0.2,
        warmEveryN: 50,
        warmTopK: 2,
        pingTimeoutMs: 400,
      },
      loadBalanceStrategy: 'random', // random / round_robin / least_latency_ewma
    });
  }

  updateRoutingConfig(config) {
    // O1：更新后重置 LatencyWarmRouter，以便下一次 chat() 重新初始化
    this._warmRouter = null;
    return writeJSON('llm_routing.json', config);
  }

  // ---- O1 路由选择：按 getRoutingConfig().strategy 返回候选 provider ID 数组 ----
  _ensureWarmRouter() {
    if (!this._warmRouter) {
      const cfg = this.getRoutingConfig();
      this._warmRouter = new LatencyWarmRouter(this.providers, Object.assign({
        alpha: 0.2, warmEveryN: 50, warmTopK: 2, pingTimeoutMs: 400
      }, (cfg && cfg.warm) || {}));
    }
    return this._warmRouter;
  }

  /**
   * O1：返回排序后的 provider 候选列表（按当前策略）。
   *   - priority：按 PROVIDER_CAPABILITY_SCORE + p.priority 数值排序，降序。
   *   - fallback：按 priority 排序（fallback=true 语义保留，失败降级由 chat/_callExternalProvider 执行）
   *   - latency-warm：使用 LatencyWarmRouter.rankedEnabledIds
   */
  _candidateProviders(strategy) {
    const enabledAll = Object.entries(this.providers)
      .filter(([id, p]) => p && p.enabled !== false && p.provider !== 'local')
      .map(([id, p]) => id);
    const priScore = (id) => {
      const p = this.providers[id];
      if (typeof p.priority === 'number') return p.priority;
      return PROVIDER_CAPABILITY_SCORE[p.provider] ?? PROVIDER_CAPABILITY_SCORE._default;
    };
    switch (strategy) {
      case 'latency-warm': {
        const r = this._ensureWarmRouter();
        const ranked = r.rankedEnabledIds().filter(id => enabledAll.includes(id));
        return ranked.length ? ranked : enabledAll.sort((a,b) => priScore(b)-priScore(a));
      }
      case 'priority':
      case 'fallback':
      default:
        return enabledAll.sort((a,b) => priScore(b)-priScore(a));
    }
  }

  _generateIntelligentResponse(messages, expertType, history) {
    const lastMsg = messages.filter(m => m.role === 'user').pop()?.content || '';
    const context = [...history, ...messages];
    
    const expertInsights = this._getExpertInsights(expertType);
    const intentAnalysis = this._analyzeIntent(lastMsg);
    const relatedOperators = this._findRelatedOperators(lastMsg);
    
    const response = {
      content: this._composeResponse(lastMsg, expertType, intentAnalysis, expertInsights, relatedOperators),
      usage: { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
      model: 'ous-internal-v3',
      provider: 'local-intelligent',
      metadata: {
        intent: intentAnalysis.primary,
        confidence: intentAnalysis.confidence,
        expert_type: expertType || 'general',
        related_operators: relatedOperators
      }
    };

    return response;
  }

  _analyzeIntent(text) {
    const lower = text.toLowerCase();
    const intents = {
      operator_recommendation: /推荐|算子|operator|建议|用什么/.test(lower),
      algorithm_analysis: /算法|复杂度|时间|空间|algorithm|complexity/.test(lower),
      graph_analysis: /图谱|节点|边|中心性|社区|pagerank|graph|node/.test(lower),
      workflow: /工作流|编排|流程|pipeline|workflow|flow/.test(lower),
      automation: /自动化|automation|自动|机器人/.test(lower),
      requirement: /需求|编译|草莓|caomei|requirement/.test(lower),
      browser: /浏览器|页面|爬取|抓取|browser|web/.test(lower),
      market: /商城|市场|下载|购买|market/.test(lower),
      mcp: /mcp|兼容|协议|protocol/.test(lower),
      monitor: /监控|日志|指标|报警|monitor|metric/.test(lower),
      security: /安全|权限|审计|security|permission/.test(lower),
      performance: /性能|优化|加速|快|performance|optimize/.test(lower),
      fusion: /融合|璇玑|全维|治理|fusion|xuanji/.test(lower),
      ai_chat: /你好|hello|hi|介绍|说明/.test(lower)
    };

    const matched = Object.entries(intents)
      .filter(([, v]) => v)
      .map(([k]) => k);

    if (matched.length === 0) {
      return { primary: 'general', confidence: 0.6 };
    }

    return { primary: matched[0], secondary: matched.slice(1), confidence: 0.85 };
  }

  _getExpertInsights(expertType) {
    const insights = {
      algorithm: ['时间复杂度分析', '空间复杂度评估', '边界条件处理', '优化建议'],
      architecture: ['分层架构设计', '服务拆分策略', '数据流向分析', '扩展性评估'],
      data: ['数据模型设计', '数据质量规则', '数据血缘追踪', '数据安全策略'],
      ai: ['模型选型建议', '训练策略', '推理优化', 'AI治理框架'],
      workflow: ['流程建模', '节点编排', '异常处理', '性能调优'],
      operator: ['算子抽象层次', '组合算子设计', '守恒律验证', '状态向量管理'],
      graph: ['图构建策略', '实体关系抽取', '图算法选型', '图谱应用场景'],
      security: ['威胁建模', '防护策略', '审计日志', '合规检查'],
      performance: ['瓶颈分析', '优化方案', '基准测试', '容量规划'],
      monitor: ['监控指标', '告警规则', '日志采集', '链路追踪'],
      market: ['市场分析', '竞品对比', '定价策略', '增长模型'],
      mcp: ['工具协议', '能力描述', '会话管理', '错误处理'],
      automation: ['自动化策略', '触发条件', '执行监控', '异常恢复'],
      requirement: ['需求拆解', '优先级排序', '验收标准', '追踪矩阵'],
      fusion: ['璇玑配置', '维度权重', '融合策略', '治理闸门']
    };
    return insights[expertType] || insights.default || ['系统分析', '方案设计', '实施建议'];
  }

  _findRelatedOperators(text) {
    const lower = text.toLowerCase();
    const operators = [
      { id: 'normalize', name: 'L2归一化', tags: ['归一化', 'normalize', '向量'] },
      { id: 'relu', name: 'ReLU激活', tags: ['激活', 'relu', '非线性'] },
      { id: 'sigmoid', name: 'Sigmoid', tags: ['压缩', 'sigmoid', '概率'] },
      { id: 'softmax', name: 'Softmax', tags: ['概率', 'softmax', '分类'] },
      { id: 'linear', name: '线性变换', tags: ['线性', '缩放', '变换'] },
      { id: 'pagerank', name: 'PageRank', tags: ['排名', 'pagerank', '影响力'] },
      { id: 'label_propagation', name: '标签传播', tags: ['社区', '传播', '聚类'] },
      { id: 'bfs', name: '广度优先搜索', tags: ['路径', '搜索', 'bfs'] },
      { id: 'activate', name: '激活传播', tags: ['传播', '能量', '激活'] },
      { id: 'degree_centrality', name: '度中心性', tags: ['中心性', '度', 'degree'] }
    ];

    return operators.filter(op => 
      op.tags.some(tag => lower.includes(tag.toLowerCase()))
    ).slice(0, 3);
  }

  _composeResponse(userText, expertType, intentAnalysis, insights, relatedOperators) {
    const text = userText.trim();
    const expertLabel = expertType ? {
      algorithm: '算法专家',
      architecture: '架构专家',
      data: '数据专家',
      ai: 'AI专家',
      workflow: '工作流专家',
      operator: '算子系统专家',
      graph: '知识图谱专家',
      security: '安全专家',
      performance: '性能优化专家',
      monitor: '可观测性专家',
      market: '商业智能专家',
      mcp: 'MCP协议专家',
      automation: '自动化专家',
      requirement: '需求工程专家',
      fusion: '融合专家'
    }[expertType] : '智能助手';

    const greetingPatterns = [/你好/, /hello/i, /hi/i, /在吗/, /介绍/];
    if (greetingPatterns.some(p => p.test(text))) {
      return `你好！我是${expertLabel}。\n\n我可以帮你：\n${this._getCapabilitiesList(expertType)}\n\n请告诉我你的具体需求，我会以专业的方式为你解答。`;
    }

    const parts = [];
    
    parts.push(`## ${expertLabel}分析\n\n针对你的问题"${text.slice(0, 50)}${text.length > 50 ? '...' : ''}"，我的分析如下：\n`);

    if (intentAnalysis.primary && intentAnalysis.primary !== 'general') {
      parts.push(`**识别意图**: ${this._translateIntent(intentAnalysis.primary)}（置信度 ${Math.round(intentAnalysis.confidence * 100)}%）\n`);
    }

    parts.push(`### 核心分析\n`);
    parts.push(this._generateExpertAnalysis(text, expertType, intentAnalysis));

    if (insights && insights.length) {
      parts.push(`\n### 专家洞察\n`);
      insights.slice(0, 4).forEach((insight, i) => {
        parts.push(`${i + 1}. **${insight}**: ${this._expandInsight(insight, expertType)}`);
      });
    }

    if (relatedOperators && relatedOperators.length) {
      parts.push(`\n### 推荐算子\n`);
      relatedOperators.forEach(op => {
        parts.push(`- \`${op.id}\` (${op.name}) — 可用于解决此类问题`);
      });
    }

    parts.push(`\n### 下一步建议\n`);
    parts.push(this._getNextSteps(text, expertType));

    return parts.join('\n');
  }

  _getCapabilitiesList(expertType) {
    const caps = {
      algorithm: ['算法复杂度分析', '优化方案设计', '代码实现建议', '边界条件检查'],
      architecture: ['系统架构设计', '微服务拆分', '技术选型建议', '扩展性评估'],
      data: ['数据模型设计', '数据治理方案', 'ETL流程设计', '数据安全策略'],
      ai: ['AI模型选型', '训练策略建议', '推理优化方案', 'AI应用设计'],
      workflow: ['工作流建模', '节点编排建议', '异常处理设计', '性能优化'],
      operator: ['算子抽象设计', '组合算子开发', '守恒律验证', '状态向量管理'],
      graph: ['图谱构建方案', '实体关系抽取', '图算法选型', '应用场景设计'],
      security: ['威胁建模', '安全防护策略', '审计日志设计', '合规检查方案'],
      performance: ['性能瓶颈分析', '优化方案设计', '基准测试建议', '容量规划'],
      monitor: ['监控体系设计', '告警规则配置', '日志采集方案', '链路追踪设计'],
      market: ['市场趋势分析', '竞品对比', '定价策略建议', '增长模型设计'],
      mcp: ['MCP工具设计', '协议兼容方案', '能力描述规范', '错误处理设计'],
      automation: ['自动化策略设计', '触发条件配置', '执行监控方案', '异常恢复设计'],
      requirement: ['需求拆解分析', '优先级排序', '验收标准定义', '追踪矩阵设计'],
      fusion: ['璇玑配置方案', '维度权重设计', '融合策略制定', '治理闸门配置']
    };
    return (caps[expertType] || caps.default || ['系统分析', '方案设计', '实施建议'])
      .map(c => `- ${c}`).join('\n');
  }

  _translateIntent(intent) {
    const map = {
      operator_recommendation: '算子推荐',
      algorithm_analysis: '算法分析',
      graph_analysis: '图谱分析',
      workflow: '工作流',
      automation: '自动化',
      requirement: '需求编译',
      browser: '浏览器自动化',
      market: '算子商城',
      mcp: 'MCP兼容',
      monitor: '系统监控',
      security: '安全审计',
      performance: '性能优化',
      fusion: '全维融合',
      ai_chat: 'AI对话',
      general: '通用咨询'
    };
    return map[intent] || intent;
  }

  _generateExpertAnalysis(text, expertType, intent) {
    const analyses = {
      operator_recommendation: '这是一个算子推荐场景。我会根据你的需求特征（输入类型、输出目标、性能要求），从算子库中筛选最合适的算子组合。关键考量维度包括：算子类型匹配度、参数适配性、守恒律兼容性。',
      algorithm_analysis: '算法分析需要从时间复杂度和空间复杂度两个维度进行评估。我建议采用大O表示法进行标准化度量，并考虑实际运行环境的硬件约束。对于大规模数据场景，还需要评估常数因子和缓存友好性。',
      graph_analysis: '知识图谱分析涉及图结构的多个维度：节点影响力（PageRank）、社区结构（标签传播）、关键路径（BFS最短路径）、节点中心性（度/中介中心性）。我建议根据具体业务场景选择合适的分析方法。',
      workflow: '工作流编排需要考虑：节点依赖关系、并行执行路径、异常处理机制、状态回滚策略。我建议采用有向无环图（DAG）进行流程建模，并为每个节点定义明确的输入输出契约。',
      general: '基于你的问题，我会从以下维度进行分析：问题理解、方案设计、实施路径、风险评估。让我逐步为你展开。'
    };
    return analyses[intent.primary] || analyses.general;
  }

  _expandInsight(insight, expertType) {
    const expansions = {
      '时间复杂度分析': '使用大O表示法，关注最坏情况和平均情况的差异，考虑数据规模增长对性能的影响',
      '空间复杂度评估': '分析内存使用模式，评估缓存友好性，考虑空间换时间的可行性',
      '分层架构设计': '采用关注点分离原则，将系统划分为表现层、业务层、数据层、基础设施层',
      '数据模型设计': '遵循第三范式，同时考虑查询性能进行适度反范式化设计',
      '算子抽象层次': '定义清晰的算子接口，支持组合和复用，确保类型安全',
      '流程建模': '使用BPMN 2.0标准建模，明确网关类型（排他/并行/包容）',
      '图谱构建策略': '采用增量构建方式，定义实体和关系的语义类型',
      '威胁建模': '使用STRIDE模型识别威胁，定义攻击面和防护措施',
      '瓶颈分析': '通过性能剖析定位热点代码，使用火焰图可视化性能分布',
      '监控指标': '定义黄金指标（延迟/流量/错误/饱和度），设计分层监控体系',
      'MCP工具设计': '遵循Model Context Protocol规范，定义工具能力描述',
      '自动化策略': '基于事件驱动设计自动化触发器，定义执行条件和后置动作',
      '需求拆解': '将高层需求逐层分解为可执行的用户故事，定义验收标准',
      '璇玑配置': '根据业务特性配置双十四维权重，定义治理闸门阈值'
    };
    return expansions[insight] || '需要根据具体场景进行深入分析和定制化设计';
  }

  _getNextSteps(text, expertType) {
    const steps = {
      operator: [
        '1. 明确输入数据类型和维度',
        '2. 选择算子基类（FunctionOperator/LinearOperator）',
        '3. 定义算子元数据（输入/输出类型、参数）',
        '4. 实现算子逻辑并注册到算子中心',
        '5. 在工作流编排中测试算子'
      ],
      workflow: [
        '1. 使用流程图编辑器设计节点和连线',
        '2. 为每个节点选择合适的算子',
        '3. 配置节点参数和条件分支',
        '4. 验证流程并执行测试',
        '5. 在璇玑治理中进行全维优化'
      ],
      graph: [
        '1. 定义节点类型和关系类型',
        '2. 添加初始节点和边',
        '3. 运行图谱分析算法（PageRank/社区发现）',
        '4. 查看可视化结果',
        '5. 在图谱管理中进行持续维护'
      ],
      default: [
        '1. 明确需求边界和目标',
        '2. 选择相关模块开始实施',
        '3. 在AI助手对话框中获取更多帮助',
        '4. 查看系统文档了解详细用法',
        '5. 联系专家获得一对一咨询'
      ]
    };
    return (steps[expertType] || steps.default).join('\n');
  }

  _generateLocalGraph(topic, description) {
    const t = (topic || '企业官网需求').toString();
    const d = (description || '').toString();
    
    const templates = {
      '企业官网': {
        nodes: [
          { id: 'concept_website', label: '企业官网', type: '概念', description: '企业官方网站建设项目', attributes: { category: '项目' } },
          { id: 'concept_user', label: '用户', type: '角色', description: '访问官网的终端用户', attributes: { role: '访客' } },
          { id: 'concept_admin', label: '管理员', type: '角色', description: '负责官网内容维护的管理员', attributes: { role: '运营' } },
          { id: 'component_frontend', label: '前端界面', type: '组件', description: '用户可见的Web界面', attributes: { tech: 'Vue.js' } },
          { id: 'component_backend', label: '后端服务', type: '组件', description: '提供API和数据处理能力', attributes: { tech: 'Node.js' } },
          { id: 'component_database', label: '数据库', type: '组件', description: '存储用户、内容和业务数据', attributes: { tech: 'MySQL' } },
          { id: 'component_cms', label: '内容管理系统', type: '组件', description: '支持管理员发布和管理内容', attributes: { feature: '富文本' } },
          { id: 'component_auth', label: '认证授权', type: '组件', description: '用户登录注册和权限管理', attributes: { method: 'JWT' } },
          { id: 'component_search', label: '搜索功能', type: '组件', description: '站内全文搜索能力', attributes: { engine: 'Elasticsearch' } },
          { id: 'process_deploy', label: '部署流程', type: '流程', description: '从开发到上线的完整流程', attributes: { method: 'CI/CD' } },
          { id: 'process_design', label: '设计流程', type: '流程', description: 'UI/UX设计和评审流程', attributes: { tool: 'Figma' } },
          { id: 'process_content', label: '内容运营流程', type: '流程', description: '内容创建、审核、发布流程', attributes: { workflow: '审批制' } },
          { id: 'data_user', label: '用户数据', type: '数据', description: '用户注册信息、行为数据', attributes: { sensitivity: '高' } },
          { id: 'data_content', label: '内容数据', type: '数据', description: '文章、产品、新闻等内容', attributes: { sensitivity: '低' } },
          { id: 'data_config', label: '配置数据', type: '数据', description: '系统配置、权限配置', attributes: { sensitivity: '中' } },
          { id: 'constraint_security', label: '安全约束', type: '约束', description: '数据加密、XSS防护、CSRF防护', attributes: { level: '必须' } },
          { id: 'constraint_performance', label: '性能约束', type: '约束', description: '首屏加载<3s，支持高并发', attributes: { level: '重要' } },
          { id: 'constraint_seo', label: 'SEO约束', type: '约束', description: '搜索引擎优化要求', attributes: { level: '重要' } },
          { id: 'goal_conversion', label: '转化目标', type: '目标', description: '访客到注册/客户的转化', attributes: { kpi: '转化率' } },
          { id: 'goal_brand', label: '品牌目标', type: '目标', description: '提升企业品牌形象和知名度', attributes: { kpi: '品牌指数' } }
        ],
        edges: [
          { source: 'concept_website', target: 'concept_user', label: '服务', weight: 1.0 },
          { source: 'concept_website', target: 'concept_admin', label: '管理', weight: 1.0 },
          { source: 'concept_website', target: 'component_frontend', label: '包含', weight: 1.0 },
          { source: 'concept_website', target: 'component_backend', label: '包含', weight: 1.0 },
          { source: 'concept_website', target: 'component_database', label: '依赖', weight: 1.0 },
          { source: 'component_frontend', target: 'component_backend', label: '使用', weight: 0.9 },
          { source: 'component_backend', target: 'component_database', label: '使用', weight: 1.0 },
          { source: 'component_cms', target: 'component_backend', label: '依赖', weight: 0.8 },
          { source: 'component_auth', target: 'component_backend', label: '集成', weight: 0.9 },
          { source: 'component_search', target: 'component_backend', label: '集成', weight: 0.8 },
          { source: 'component_search', target: 'data_content', label: '搜索', weight: 0.9 },
          { source: 'concept_admin', target: 'component_cms', label: '使用', weight: 0.9 },
          { source: 'concept_user', target: 'component_frontend', label: '访问', weight: 1.0 },
          { source: 'process_design', target: 'component_frontend', label: '产出', weight: 0.8 },
          { source: 'process_deploy', target: 'component_backend', label: '部署', weight: 0.9 },
          { source: 'process_content', target: 'component_cms', label: '管理', weight: 0.9 },
          { source: 'data_user', target: 'component_auth', label: '存储于', weight: 0.9 },
          { source: 'data_content', target: 'component_cms', label: '存储于', weight: 0.9 },
          { source: 'data_config', target: 'component_backend', label: '配置于', weight: 0.8 },
          { source: 'constraint_security', target: 'component_auth', label: '约束', weight: 1.0 },
          { source: 'constraint_security', target: 'component_backend', label: '约束', weight: 1.0 },
          { source: 'constraint_performance', target: 'component_frontend', label: '约束', weight: 0.9 },
          { source: 'constraint_seo', target: 'component_frontend', label: '约束', weight: 0.8 },
          { source: 'goal_conversion', target: 'concept_user', label: '影响', weight: 0.9 },
          { source: 'goal_brand', target: 'concept_website', label: '影响', weight: 1.0 }
        ],
        summary: '企业官网需求知识图谱覆盖了从用户访问、内容管理到后端服务的完整架构，包含20个核心概念节点和25条关系边，体现了各组件间的依赖和约束关系。'
      }
    };
    
    for (const [key, template] of Object.entries(templates)) {
      if (t.includes(key)) {
        return {
          nodes: template.nodes.map(n => ({ ...n, id: n.id.replace(key.toLowerCase().replace(/\s+/g, '_'), 'topic') })),
          edges: template.edges.map(e => ({ ...e })),
          summary: template.summary
        };
      }
    }
    
    const nodes = [
      { id: 'topic_' + Date.now() + '_root', label: t, type: '概念', description: d || t + '核心概念', attributes: { topic: t } },
      { id: 'topic_user', label: '用户', type: '角色', description: t + '的使用者', attributes: {} },
      { id: 'topic_admin', label: '管理员', type: '角色', description: t + '的管理者', attributes: {} },
      { id: 'topic_frontend', label: '前端组件', type: '组件', description: t + '的前端实现', attributes: {} },
      { id: 'topic_backend', label: '后端组件', type: '组件', description: t + '的后端实现', attributes: {} },
      { id: 'topic_data', label: '数据层', type: '组件', description: t + '的数据存储', attributes: {} },
      { id: 'topic_process', label: '核心流程', type: '流程', description: t + '的核心业务流程', attributes: {} },
      { id: 'topic_data_flow', label: '数据流', type: '流程', description: t + '的数据流向', attributes: {} },
      { id: 'topic_constraint', label: '约束条件', type: '约束', description: t + '的关键约束', attributes: {} },
      { id: 'topic_goal', label: '目标', type: '目标', description: t + '的实现目标', attributes: {} },
      { id: 'topic_api', label: 'API接口', type: '组件', description: t + '的对外接口', attributes: {} },
      { id: 'topic_auth', label: '认证授权', type: '组件', description: t + '的安全认证', attributes: {} },
      { id: 'topic_monitor', label: '监控系统', type: '组件', description: t + '的运行监控', attributes: {} },
      { id: 'topic_deploy', label: '部署流程', type: '流程', description: t + '的部署上线', attributes: {} },
      { id: 'topic_data_model', label: '数据模型', type: '数据', description: t + '的数据结构', attributes: {} }
    ];
    
    const edges = [
      { source: 'topic_' + Date.now() + '_root', target: 'topic_user', label: '服务', weight: 1.0 },
      { source: 'topic_' + Date.now() + '_root', target: 'topic_admin', label: '管理', weight: 1.0 },
      { source: 'topic_' + Date.now() + '_root', target: 'topic_frontend', label: '包含', weight: 1.0 },
      { source: 'topic_' + Date.now() + '_root', target: 'topic_backend', label: '包含', weight: 1.0 },
      { source: 'topic_' + Date.now() + '_root', target: 'topic_data', label: '依赖', weight: 1.0 },
      { source: 'topic_frontend', target: 'topic_backend', label: '使用', weight: 0.9 },
      { source: 'topic_backend', target: 'topic_data', label: '使用', weight: 1.0 },
      { source: 'topic_backend', target: 'topic_api', label: '提供', weight: 0.9 },
      { source: 'topic_backend', target: 'topic_auth', label: '集成', weight: 0.9 },
      { source: 'topic_backend', target: 'topic_monitor', label: '集成', weight: 0.8 },
      { source: 'topic_user', target: 'topic_frontend', label: '访问', weight: 1.0 },
      { source: 'topic_admin', target: 'topic_backend', label: '管理', weight: 0.9 },
      { source: 'topic_process', target: 'topic_backend', label: '实现于', weight: 0.8 },
      { source: 'topic_data_flow', target: 'topic_process', label: '贯穿', weight: 0.9 },
      { source: 'topic_data_flow', target: 'topic_data', label: '存储于', weight: 0.9 },
      { source: 'topic_constraint', target: 'topic_backend', label: '约束', weight: 1.0 },
      { source: 'topic_constraint', target: 'topic_frontend', label: '约束', weight: 0.9 },
      { source: 'topic_goal', target: 'topic_user', label: '服务于', weight: 0.9 },
      { source: 'topic_goal', target: 'topic_' + Date.now() + '_root', label: '达成', weight: 1.0 },
      { source: 'topic_deploy', target: 'topic_backend', label: '部署', weight: 0.9 },
      { source: 'topic_deploy', target: 'topic_frontend', label: '部署', weight: 0.9 },
      { source: 'topic_data_model', target: 'topic_data', label: '定义于', weight: 1.0 },
      { source: 'topic_api', target: 'topic_frontend', label: '被调用', weight: 0.8 },
      { source: 'topic_auth', target: 'topic_user', label: '验证', weight: 0.9 },
      { source: 'topic_monitor', target: 'topic_deploy', label: '监控', weight: 0.8 }
    ];
    
    const rootId = nodes[0].id;
    const rootEdges = edges.map(e => ({ ...e, source: e.source === 'topic_' + Date.now() + '_root' ? rootId : e.source }));
    
    return {
      nodes,
      edges: rootEdges,
      summary: `${t}知识图谱包含${nodes.length}个核心概念和${edges.length}条关系，覆盖了从用户、组件、流程到约束和目标的完整维度。`
    };
  }
}

let gatewayInstance = null;

function getGateway() {
  if (!gatewayInstance) {
    gatewayInstance = new LLMGateway();
  }
  return gatewayInstance;
}

module.exports = { LLMGateway, getGateway, PROVIDER_PRESETS, LatencyWarmRouter, ROUTING_STRATEGIES };