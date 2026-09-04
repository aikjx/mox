Read "D:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\llm-gateway.js":

     1	'use strict';
     2	
     3	const fs = require('fs');
     4	const path = require('path');
     5	const crypto = require('crypto');
     6	
     7	const DATA_DIR = path.join(__dirname, '..', 'data');
     8	const CIPHER_KEY = process.env.LLM_CIPHER_KEY || 'ous-llm-gateway-enterprise-key-2024';
     9	
    10	const PROVIDER_PRESETS = {
    11	  deepseek: {
    12	    name: 'DeepSeek',
    13	    base_url: 'https://api.deepseek.com/v1',
    14	    models: ['deepseek-chat', 'deepseek-reasoner'],
    15	    description: 'DeepSeek 大模型，支持中文对话和代码生成'
    16	  },
    17	  volcengine: {
    18	    name: '火山引擎',
    19	    base_url: 'https://ark.cn-beijing.volces.com/api/v3',
    20	    models: ['doubao-pro-32k', 'doubao-pro-128k', 'doubao-lite-32k'],
    21	    description: '字节跳动豆包大模型，支持中文业务场景'
    22	  },
    23	  qwen: {
    24	    name: '阿里云千问',
    25	    base_url: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    26	    models: ['qwen-max', 'qwen-plus', 'qwen-turbo', 'qwen-long'],
    27	    description: '阿里云千问大模型，支持多模态和长上下文'
    28	  },
    29	  kimi: {
    30	    name: 'Kimi (月之暗面)',
    31	    base_url: 'https://api.moonshot.cn/v1',
    32	    models: ['moonshot-v1-8k', 'moonshot-v1-32k', 'moonshot-v1-128k', 'kimi-latest'],
    33	    description: '月之暗面 Kimi 大模型，长上下文与中文理解能力强'
    34	  },
    35	  zhipu: {
    36	    name: '智谱AI',
    37	    base_url: 'https://open.bigmodel.cn/api/paas/v4',
    38	    models: ['glm-4', 'glm-4-flash', 'glm-3-turbo'],
    39	    description: '智谱AI大模型，支持长文本和复杂推理'
    40	  },
    41	  openai: {
    42	    name: 'OpenAI',
    43	    base_url: 'https://api.openai.com/v1',
    44	    models: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-3.5-turbo'],
    45	    description: 'OpenAI 最新模型，支持多模态和高级推理'
    46	  },
    47	  anthropic: {
    48	    name: 'Anthropic',
    49	    base_url: 'https://api.anthropic.com/v1',
    50	    models: ['claude-3.5-sonnet', 'claude-3-opus', 'claude-3-sonnet'],
    51	    description: 'Anthropic Claude模型，长上下文推理能力强'
    52	  },
    53	  google: {
    54	    name: 'Google Gemini',
    55	    base_url: 'https://generativelanguage.googleapis.com/v1beta',
    56	    models: ['gemini-2.0-flash', 'gemini-1.5-pro', 'gemini-1.5-flash'],
    57	    description: 'Google Gemini 模型，多模态能力突出'
    58	  },
    59	  ollama: {
    60	    name: 'Ollama (本地)',
    61	    base_url: 'http://localhost:11434/api',
    62	    models: ['llama3', 'qwen2.5', 'deepseek-r1', 'mistral'],
    63	    description: '本地 Ollama 部署，数据完全离线'
    64	  },
    65	  custom: {
    66	    name: '自定义',
    67	    base_url: '',
    68	    models: [],
    69	    description: '自定义 Provider，填入 Base URL 和模型名即可'
    70	  }
    71	};
    72	
    73	function readJSON(file, fallback) {
    74	  try {
    75	    const fp = path.join(DATA_DIR, file);
    76	    if (!fs.existsSync(fp)) return fallback;
    77	    const raw = fs.readFileSync(fp, 'utf8');
    78	    return raw ? JSON.parse(raw) : fallback;
    79	  } catch (e) {
    80	    return fallback;
    81	  }
    82	}
    83	
    84	function writeJSON(file, data) {
    85	  try {
    86	    fs.writeFileSync(path.join(DATA_DIR, file), JSON.stringify(data, null, 2), 'utf8');
    87	    return true;
    88	  } catch (e) {
    89	    console.error('[llm-gateway] writeJSON', file, e.message);
    90	    return false;
    91	  }
    92	}
    93	
    94	function encryptApiKey(apiKey) {
    95	  if (!apiKey) return '';
    96	  const iv = crypto.randomBytes(16);
    97	  const cipher = crypto.createCipheriv('aes-256-gcm', Buffer.from(CIPHER_KEY.padEnd(32).slice(0, 32)), iv);
    98	  let encrypted = cipher.update(apiKey, 'utf8', 'hex');
    99	  encrypted += cipher.final('hex');
   100	  const tag = cipher.getAuthTag().toString('hex');
   101	  return JSON.stringify({ iv: iv.toString('hex'), encrypted, tag });
   102	}
   103	
   104	function decryptApiKey(encryptedStr) {
   105	  if (!encryptedStr) return '';
   106	  try {
   107	    const obj = JSON.parse(encryptedStr);
   108	    const iv = Buffer.from(obj.iv, 'hex');
   109	    const tag = Buffer.from(obj.tag, 'hex');
   110	    const decipher = crypto.createDecipheriv('aes-256-gcm', Buffer.from(CIPHER_KEY.padEnd(32).slice(0, 32)), iv);
   111	    decipher.setAuthTag(tag);
   112	    let decrypted = decipher.update(obj.encrypted, 'hex', 'utf8');
   113	    decrypted += decipher.final('utf8');
   114	    return decrypted;
   115	  } catch (e) {
   116	    return encryptedStr;
   117	  }
   118	}
   119	
   120	function maskApiKey(key) {
   121	  if (!key) return '';
   122	  const unmasked = decryptApiKey(key);
   123	  if (unmasked.length <= 8) return '****';
   124	  return unmasked.slice(0, 4) + '****' + unmasked.slice(-4);
   125	}
   126	
   127	// 各 Provider 的能力评分（用于"自动优选最好的 AI 引擎"）
   128	// 综合中文理解、代码、推理、长上下文等维度，分数越高越优先被自动选中。
   129	const PROVIDER_CAPABILITY_SCORE = {
   130	  deepseek: 95, // 中文 + 代码 + 推理强
   131	  volcengine: 90, // 豆包，长上下文友好
   132	  openai: 92, // 综合最强推理/多模态
   133	  qwen: 85, // 阿里云千问，多模态 + 长上下文
   134	  kimi: 86, // Kimi，长上下文 + 中文理解
   135	  zhipu: 82, // 智谱 GLM，长文本 + 复杂推理
   136	  anthropic: 88, // Claude，长上下文推理
   137	  google: 87, // Gemini，多模态
   138	  local: 0, // 内置假引擎，不算真实 AI
   139	  _default: 60 // 未知 provider 给中性分
   140	};
   141	
   142	// ---- O1 · LatencyWarmRouter（对比 Dify/LangGraph/Flo/AutoGen，企业级多活路由） ----
   143	// 核心算法：
   144	//   score = 0.6 * norm(1 - ewma_latency) + 0.3 * success_rate_ewma + 0.1 * priority_norm
   145	//   EWMA α=0.2；每 50 次请求 Top-2 providers 主动 "预热 ping"（短 dummy call 或 /v1/models 等价），以捕获真实抖动。
   146	//   失败：立即按 fallback=true 降级，更新 success_rate ewma 并扣减分数；下一次排序自动将其降级。
   147	class LatencyWarmRouter {
   148	  constructor(providers, options = {}) {
   149	    this.options = Object.assign({ alpha: 0.2, warmEveryN: 50, warmTopK: 2, pingTimeoutMs: 400 }, options);
   150	    this._warmRequestCount = 0;
   151	    this.providers = providers; // 引用：每次 score 前读取最新 provider 对象
   152	    // 初始化 EWMA 指标
   153	    this.ewmaLatencyMs = Object.create(null);
   154	    this.ewmaSuccessRate = Object.create(null);
   155	    for (const p of Object.values(providers)) {
   156	      const id = p.id || p.provider;
   157	      const baseLat = p.estimated_latency_ms || 400;
   158	      this.ewmaLatencyMs[id] = typeof baseLat === 'number' ? baseLat : 400;
   159	      this.ewmaSuccessRate[id] = (typeof p.error_rate === 'number') ? Math.max(0, 1 - p.error_rate) : 0.95;
   160	    }
   161	  }
   162	  _priorityOf(p) {
   163	    if (typeof p.priority === 'number') return p.priority;
   164	    if (typeof p.provider === 'string') {
   165	      return (PROVIDER_CAPABILITY_SCORE[p.provider] != null) ? PROVIDER_CAPABILITY_SCORE[p.provider] : PROVIDER_CAPABILITY_SCORE._default;
   166	    }
   167	    return PROVIDER_CAPABILITY_SCORE._default;
   168	  }
   169	  _scoreProvider(id, p) {
   170	    const lat = this.ewmaLatencyMs[id];
   171	    const maxLat = Math.max(1, ...Object.values(this.ewmaLatencyMs));
   172	    const normLat = 1 - Math.min(1, lat / maxLat);
   173	    const success = this.ewmaSuccessRate[id];
   174	    const priMax = Math.max(1, ...Object.values(this.providers).map(x => this._priorityOf(x)));
   175	    const priNorm = this._priorityOf(p) / priMax;
   176	    return 0.6 * normLat + 0.3 * success + 0.1 * priNorm;
   177	  }
   178	  // 返回启用 provider 按得分降序排序的 id 数组
   179	  rankedEnabledIds() {
   180	    const entries = Object.entries(this.providers)
   181	      .filter(([id, p]) => p && p.enabled !== false);
   182	    return entries
   183	      .map(([id, p]) => [id, this._scoreProvider(id, p)])
   184	      .sort((a,b) => b[1] - a[1])
   185	      .map(([id]) => id);
   186	  }
   187	  // 记录一次真实请求结果（更新 EWMA）
   188	  recordResult(id, latencyMs, ok) {
   189	    if (this.ewmaLatencyMs[id] == null) this.ewmaLatencyMs[id] = Math.max(1, latencyMs || 1);
   190	    if (this.ewmaSuccessRate[id] == null) this.ewmaSuccessRate[id] = ok ? 1 : 0.5;
   191	    const alpha = this.options.alpha;
   192	    this.ewmaLatencyMs[id] = (1 - alpha) * this.ewmaLatencyMs[id] + alpha * Math.max(1, latencyMs || 1);
   193	    const sampleErr = ok ? 0 : 1;
   194	    this.ewmaSuccessRate[id] = (1 - alpha) * this.ewmaSuccessRate[id] + alpha * (1 - sampleErr);
   195	  }
   196	  // 预热 Top-2 providers：调用方通过 warmCb(p) 做一次轻量 ping；完成后 recordResult。
   197	  async maybeWarmTop(warmCb) {
   198	    this._warmRequestCount++;
   199	    if (this._warmRequestCount % this.options.warmEveryN !== 1) return;
   200	    const ids = this.rankedEnabledIds().slice(0, this.options.warmTopK);
   201	    for (const id of ids) {
   202	      try {
   203	        const start = Date.now();
   204	        const ok = await Promise.race([
   205	          Promise.resolve(warmCb && warmCb(this.providers[id])).then(() => true),
   206	          new Promise(r => setTimeout(() => r(false), this.options.pingTimeoutMs))
   207	        ]);
   208	        this.recordResult(id, Date.now() - start, !!ok);
   209	      } catch (_) {
   210	        this.recordResult(id, this.options.pingTimeoutMs, false);
   211	      }
   212	    }
   213	  }
   214	  // 暴露给 unit-test / summary
   215	  snapshot() {
   216	    return Object.fromEntries(
   217	      Object.entries(this.providers).map(([id, p]) => [id, {
   218	        lat_ewma: this.ewmaLatencyMs[id] ?? null,
   219	        sr_ewma: this.ewmaSuccessRate[id] ?? null,
   220	        score: this._scoreProvider(id, p)
   221	      }])
   222	    );
   223	  }
   224	}
   225	
   226	// O1 策略常量（和 getRoutingConfig() / T4 H2 保持一致）
   227	const ROUTING_STRATEGIES = ['priority', 'fallback', 'latency-warm'];
   228	
   229	class LLMGateway {
   230	  constructor() {
   231	    this.providers = {};
   232	    this.activeProvider = null;
   233	    this.conversations = new Map();
   234	    this.usage = {};
   235	    this.requestLog = [];
   236	    this.maxRetries = 3;
   237	    this.requestTimeout = 30000;
   238	    /** @type {LatencyWarmRouter|null} O1 补丁实例 */
   239	    this._warmRouter = null;
   240	    this._warmRequestCount = 0;
   241	    this._init();
   242	  }
   243	
   244	  _init() {
   245	    const config = readJSON('llm_config.json', []);
   246	    if (Array.isArray(config) && config.length) {
   247	      config.forEach((p) => {
   248	        const provider = { ...p };
   249	        if (provider.api_key && !provider.api_key.startsWith('{')) {
   250	          provider.api_key = encryptApiKey(provider.api_key);
   251	        }
   252	        this.providers[p.id || p.provider] = provider;
   253	      });
   254	    }
   255	
   256	    // 环境变量自动注入：若系统环境变量中存在 DeepSeek API Key，
   257	    // 则自动补全并启用 DeepSeek Provider，无需在前端手动填写。
   258	    const envDeepSeekKey = process.env.DEEPSEEK_API_KEY || process.env.DEEPSEEK_API_KEY_ENV;
   259	    if (envDeepSeekKey && String(envDeepSeekKey).trim().length > 0) {
   260	      const dsId = 'llm_deepseek';
   261	      const ds = this.providers[dsId] || { id: dsId, name: 'DeepSeek', provider: 'deepseek', base_url: 'https://api.deepseek.com/v1', model: 'deepseek-chat', description: 'DeepSeek 大模型（环境变量自动注入）' };
   262	      ds.api_key = encryptApiKey(String(envDeepSeekKey).trim());
   263	      ds.enabled = true;
   264	      ds.provider = 'deepseek';
   265	      this.providers[dsId] = ds;
   266	      console.log('[LLM] 已从环境变量 DEEPSEEK_API_KEY 自动启用 DeepSeek 引擎');
   267	    }
   268	    // 选择激活 Provider 的规则：
   269	    //  - 默认不应选 local（内置假引擎），否则对话永远走不到真实 LLM
   270	    //  - 仅在"真实 Provider 已启用且配置了 api_key"时自动激活（按能力评分优选最好的引擎）
   271	    //  - 没有任何真实 AI 时，才退回 local 作为兜底（此时标记为无 AI）
   272	    const realCandidates = Object.values(this.providers).filter(
   273	      (p) => p.provider && p.provider !== 'local' && p.enabled && p.api_key && String(p.api_key).trim().length > 0
   274	    );
   275	    realCandidates.sort((a, b) => (PROVIDER_CAPABILITY_SCORE[b.provider] || PROVIDER_CAPABILITY_SCORE._default) - (PROVIDER_CAPABILITY_SCORE[a.provider] || PROVIDER_CAPABILITY_SCORE._default));
   276	    if (realCandidates.length) {
   277	      this.activeProvider = realCandidates[0].id || realCandidates[0].provider;
   278	    } else {
   279	      // 无真实 AI，退回 local 兜底（保持向后兼容）
   280	      const local = Object.values(this.providers).find((p) => p.provider === 'local');
   281	      this.activeProvider = local ? (local.id || local.provider) : (Object.keys(this.providers)[0] || null);
   282	    }
   283	    this.usage = readJSON('llm_usage.json', {});
   284	  }
   285	  // 当前是否配置了「真实 AI 引擎」（已启用 + 非 local + 已填 api_key）
   286	  isRealAI() {
   287	    const provider = this.activeProvider ? this.providers[this.activeProvider] : null;
   288	    return !!(provider && provider.provider && provider.provider !== 'local' && provider.api_key && String(provider.api_key).trim().length > 0);
   289	  }
   290	
   291	  // 当前可用（已启用 + 已配置 Key + 非 local）的 Provider 列表，供优化引擎枚举
   292	  listAvailableProviders() {
   293	    return Object.values(this.providers)
   294	      .filter((p) => p.provider && p.provider !== 'local' && p.enabled && p.api_key && String(p.api_key).trim().length > 0)
   295	      .map((p) => ({ id: p.id, name: p.name || p.id, provider: p.provider, model: p.model }));
   296	  }
   297	
   298	  // 评测专用：指定 Provider 的严格单次调用（不重试、不本地降级），
   299	  // 失败即抛错，保证优化评分不被假回复污染。
   300	  async chatWithProvider(providerId, params) {
   301	    const provider = this.providers[providerId];
   302	    if (!provider || !provider.enabled || provider.provider === 'local' || !provider.api_key) {
   303	      throw new Error(`Provider 不可用或未配置: ${providerId}`);
   304	    }
   305	    const { messages, temperature = 0.3, maxTokens = 512, systemPrompt } = params;
   306	    const all = systemPrompt
   307	      ? [{ role: 'system', content: this._buildTimeContext() }, { role: 'system', content: systemPrompt }, ...messages]
   308	      : [{ role: 'system', content: this._buildTimeContext() }, ...messages];
   309	
   310	    const url = provider.base_url || 'https://api.openai.com/v1';
   311	    const model = provider.model || 'gpt-4';
   312	    const apiKey = decryptApiKey(provider.api_key);
   313	
   314	    const controller = new AbortController();
   315	    const timeoutId = setTimeout(() => controller.abort(), this.requestTimeout);
   316	    const start = Date.now();
   317	    try {
   318	      const response = await fetch(`${url}/chat/completions`, {
   319	        method: 'POST',
   320	        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${apiKey}` },
   321	        body: JSON.stringify({ model, messages: all, temperature, max_tokens: maxTokens }),
   322	        signal: controller.signal
   323	      });
   324	      if (!response.ok) throw new Error(`LLM API error: ${response.status}`);
   325	      const data = await response.json();
   326	      this._recordUsage(provider.id || provider.provider, data.usage || {});
   327	      return {
   328	        content: data.choices[0].message.content,
   329	        usage: data.usage || { total_tokens: 0 },
   330	        model: data.model,
   331	        provider: provider.id,
   332	        latency_ms: Date.now() - start
   333	      };
   334	    } finally {
   335	      clearTimeout(timeoutId);
   336	    }
   337	  }
   338	
   339	  // 构建实时时间上下文：LLM 训练数据存在截止时间，不注入当前时间会导致
   340	  // "今天是？"这类问题被模型凭训练记忆编造日期（幻觉）。
   341	  _buildTimeContext() {
   342	    const now = new Date();
   343	    const days = ['日', '一', '二', '三', '四', '五', '六'];
   344	    const pad = (n) => String(n).padStart(2, '0');
   345	    const tzOffsetMin = -now.getTimezoneOffset();
   346	    const tzSign = tzOffsetMin >= 0 ? '+' : '-';
   347	    const tzHours = Math.floor(Math.abs(tzOffsetMin) / 60);
   348	    const tzMins = Math.abs(tzOffsetMin) % 60;
   349	    const tz = `UTC${tzSign}${tzHours}${tzMins ? ':' + pad(tzMins) : ''}`;
   350	    return [
   351	      '【实时环境】',
   352	      `当前真实日期时间：${now.getFullYear()}年${now.getMonth() + 1}月${now.getDate()}日（星期${days[now.getDay()]}）${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}，时区 ${tz}。`,
   353	      '规则：凡涉及"今天/现在/当前日期"等时间问题，必须以上述时间为唯一事实依据，严禁凭训练记忆猜测或编造日期。'
   354	    ].join('\n');
   355	  }
   356	
   357	  // 构建增强消息（时间上下文 + 联网上下文 + 专家系统提示 + 会话历史）：chat 与 chatStream 共用
   358	  _buildEnhancedMessages(params) {
   359	    const { messages, sessionId, expertType, systemPrompt, webSearchContext } = params;
   360	    const enhancedMessages = [];
   361	
   362	    // 始终注入实时时间上下文（防止日期幻觉）
   363	    enhancedMessages.push({ role: 'system', content: this._buildTimeContext() });
   364	
   365	    // 联网搜索上下文：由调用方（/ai/chat）在开启联网时注入
   366	    if (webSearchContext) {
   367	      enhancedMessages.push({ role: 'system', content: webSearchContext });
   368	    }
   369	
   370	    if (systemPrompt) {
   371	      enhancedMessages.push({ role: 'system', content: systemPrompt });
   372	    } else if (expertType) {
   373	      const expertPrompt = this._getExpertSystemPrompt(expertType);
   374	      enhancedMessages.push({ role: 'system', content: expertPrompt });
   375	    }
   376	
   377	    const convHistory = sessionId ? this.conversations.get(sessionId) || [] : [];
   378	    return { allMessages: [...enhancedMessages, ...convHistory, ...messages], convHistory };
   379	  }
   380	
   381	  // 会话记忆更新（chat 与 chatStream 共用）：LRU 1000 会话
   382	  _updateConversation(sessionId, convHistory, messages, content) {
   383	    if (!sessionId) return;
   384	    const updatedHistory = [...convHistory, ...messages];
   385	    if (content) {
   386	      updatedHistory.push({ role: 'assistant', content });
   387	    }
   388	    this.conversations.set(sessionId, updatedHistory);
   389	    if (this.conversations.size > 1000) {
   390	      const oldestKey = this.conversations.keys().next().value;
   391	      this.conversations.delete(oldestKey);
   392	    }
   393	  }
   394	
   395	  async chat(params) {
   396	    const { messages, sessionId, expertType, systemPrompt, temperature = 0.7, maxTokens = 2048 } = params;
   397	
   398	    const provider = this.activeProvider ? this.providers[this.activeProvider] : null;
   399	
   400	    const { allMessages, convHistory } = this._buildEnhancedMessages(params);
   401	
   402	    // O1：预热 Top-K（latency-warm 策略每次 chat 触发；对 priority/fallback 策略此操作为空，开销可忽略）
   403	    const routingCfg = this.getRoutingConfig();
   404	    const strategy = ROUTING_STRATEGIES.includes(routingCfg.strategy) ? routingCfg.strategy : 'fallback';
   405	    const enableFallback = routingCfg.fallback !== false;
   406	    if (strategy === 'latency-warm') {
   407	      const r = this._ensureWarmRouter();
   408	      // 预热不阻塞主请求（fire-and-forget 但记录结果）—— 避免预热慢拖慢首次响应
   409	      r.maybeWarmTop(async (p) => {
   410	        // 轻量 ping：GET {base_url}/models（若未配置则模拟）；无有效 key 不会抛错但返回 false。
   411	        if (!p || !p.base_url) return false;
   412	        try {
   413	          const r0 = await fetch(`${p.base_url}/models`, {
   414	            method: 'GET',
   415	            signal: AbortSignal.timeout ? AbortSignal.timeout(500) : void 0,
   416	            headers: p.api_key ? { 'Authorization': `Bearer ${decryptApiKey(p.api_key)}` } : {}
   417	          });
   418	          return r0 && r0.ok;
   419	        } catch (_) { return false; }
   420	      }).catch(() => {});
   421	    }
   422	
   423	    let result;
   424	
   425	    const singleProviderMode = (provider && provider.enabled && provider.provider !== 'local' && strategy === 'priority' && this.activeProvider);
   426	    if (singleProviderMode) {
   427	      // O1 兼容：单 activeProvider 指定模式（priority 下仅走该 provider，与旧行为一致）
   428	      result = await this._callExternalProvider(provider, allMessages, temperature, maxTokens);
   429	      // O1 EWMA 更新（若 router 已存在）：
   430	      if (this._warmRouter) this._warmRouter.recordResult(provider.id || provider.provider, result && result.latency_ms || 0, !(result && String(result.provider || '').startsWith('local')));
   431	    } else if (strategy === 'fallback' || strategy === 'latency-warm') {
   432	      // O1 fallback / latency-warm：多候选依次尝试
   433	      const candidates = this._candidateProviders(strategy);
   434	      const local = this._generateIntelligentResponse(messages, expertType, convHistory);
   435	      result = local;
   436	      for (const id of candidates) {
   437	        const p = this.providers[id];
   438	        if (!p || p.enabled === false || p.provider === 'local' || !p.api_key) continue;
   439	        const startTs = Date.now();
   440	        try {
   441	          const r = await this._callExternalProvider(p, allMessages, temperature, maxTokens);
   442	          const latency = Date.now() - startTs;
   443	          const isRealAI = r && !(String(r.provider || '').startsWith('local'));
   444	          if (this._warmRouter) this._warmRouter.recordResult(id, latency, isRealAI);
   445	          if (isRealAI) { result = Object.assign({}, r, { latency_ms: latency, used_fallback: candidates.indexOf(id) > 0, routing_strategy: strategy }); break; }
   446	        } catch (e) {
   447	          if (this._warmRouter) this._warmRouter.recordResult(id, Date.now() - startTs, false);
   448	          if (!enableFallback) break; // 无 fallback 直接用 local 默认
   449	        }
   450	      }
   451	    } else if (provider && provider.enabled && provider.provider !== 'local') {
   452	      result = await this._callExternalProvider(provider, allMessages, temperature, maxTokens);
   453	    } else if (expertType === 'graph' && systemPrompt && systemPrompt.includes('nodes') && systemPrompt.includes('edges')) {
   454	      console.log('[gateway] Graph generation detected, expertType:', expertType, 'systemPrompt length:', systemPrompt.length);
   455	      const userText = messages.filter(m => m.role === 'user').map(m => m.content).join(' ');
   456	      const topicMatch = userText.match(/主题[：:]\s*(.+)/);
   457	      const descMatch = userText.match(/详细描述[：:]\s*(.+)/);
   458	      const topic = topicMatch ? topicMatch[1].trim() : userText.split('\n')[0].trim();
   459	      const description = descMatch ? descMatch[1].trim() : '';
   460	      console.log('[gateway] Extracted topic:', topic, 'description:', description);
   461	      const graphData = this._generateLocalGraph(topic, description);
   462	      console.log('[gateway] Generated graph:', graphData.nodes.length, 'nodes,', graphData.edges.length, 'edges');
   463	      result = {
   464	        content: JSON.stringify(graphData),
   465	        usage: { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
   466	        model: 'ous-local-graph-v1',
   467	        provider: 'local-graph-generator',
   468	        metadata: { type: 'graph-generation', nodeCount: graphData.nodes.length, edgeCount: graphData.edges.length }
   469	      };
   470	    } else {
   471	      console.log('[gateway] Falling through to _generateIntelligentResponse, expertType:', expertType, 'hasSystemPrompt:', !!systemPrompt);
   472	      result = this._generateIntelligentResponse(messages, expertType, convHistory);
   473	    }
   474	
   475	    this._updateConversation(sessionId, convHistory, messages, result && result.content);
   476	
   477	    return result;
   478	  }
   479	
   480	  /**
   481	   * 流式对话（SSE）：真实 Provider 逐 token 推送，onChunk(delta, fullContent) 回调。
   482	   * 无真实 AI 时一次性降级推送本地结果（不伪装流式，ai_powered 标记为 false）。
   483	   * 协议：OpenAI 兼容 stream:true + stream_options.include_usage（DeepSeek/vLLM 等均支持）。
   484	   */
   485	  async chatStream(params, onChunk) {
   486	    const { messages, sessionId, temperature = 0.7, maxTokens = 2048 } = params;
   487	    const provider = this.activeProvider ? this.providers[this.activeProvider] : null;
   488	    const { allMessages, convHistory } = this._buildEnhancedMessages(params);
   489	
   490	    // 无真实 AI：降级一次性返回（显式标记非 AI，不伪装）
   491	    if (!provider || !provider.enabled || provider.provider === 'local') {
   492	      const local = this._generateIntelligentResponse(messages, null, convHistory);
   493	      if (onChunk && local.content) onChunk(local.content, local.content);
   494	      this._updateConversation(sessionId, convHistory, messages, local.content);
   495	      return { ...local, ai_powered: false };
   496	    }
   497	
   498	    const url = provider.base_url || 'https://api.openai.com/v1';
   499	    const model = provider.model || 'gpt-4';
   500	    const apiKey = decryptApiKey(provider.api_key);
   501	    const start = Date.now();
   502	
   503	    const payload = {
   504	      model,
   505	      messages: allMessages,
   506	      temperature,
   507	      max_tokens: maxTokens,
   508	      stream: true,
   509	      stream_options: { include_usage: true }
   510	    };
   511	
   512	    const controller = new AbortController();
   513	    const timeoutId = setTimeout(() => controller.abort(), this.requestTimeout);
   514	    // 客户端断开（SSE 连接关闭）→ 中止上游流，避免无谓 token 消耗
   515	    if (params.signal) {
   516	      params.signal.addEventListener('abort', () => controller.abort(), { once: true });
   517	    }
   518	    try {
   519	      const response = await fetch(`${url}/chat/completions`, {
   520	        method: 'POST',
   521	        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${apiKey}` },
   522	        body: JSON.stringify(payload),
   523	        signal: controller.signal
   524	      });
   525	      if (!response.ok) throw new Error(`LLM API error: ${response.status}`);
   526	
   527	      const reader = response.body.getReader();
   528	      const decoder = new TextDecoder('utf-8');
   529	      let buffer = '';
   530	      let content = '';
   531	      let usage = null;
   532	      let respModel = model;
   533	
   534	      while (true) {
   535	        const { done, value } = await reader.read();
   536	        if (done) break;
   537	        buffer += decoder.decode(value, { stream: true });
   538	        const lines = buffer.split('\n');
   539	        buffer = lines.pop() || ''; // 尾行可能不完整，留待下个 chunk
   540	        for (const line of lines) {
   541	          const trimmed = line.trim();
   542	          if (!trimmed.startsWith('data:')) continue;
   543	          const data = trimmed.slice(5).trim();
   544	          if (data === '[DONE]') continue;
   545	          try {
   546	            const json = JSON.parse(data);
   547	            if (json.usage) usage = json.usage;
   548	            if (json.model) respModel = json.model;
   549	            const delta = json.choices && json.choices[0] && json.choices[0].delta && json.choices[0].delta.content;
   550	            if (delta) {
   551	              content += delta;
   552	              if (onChunk) onChunk(delta, content);
   553	            }
   554	          } catch (_e) { /* 不完整 JSON 行，忽略 */ }
   555	        }
   556	      }
   557	
   558	      if (usage) this._recordUsage(provider.id || provider.provider, usage);
   559	      this._logRequest(provider.id || provider.provider, 'success', Date.now() - start);
   560	
   561	      this._updateConversation(sessionId, convHistory, messages, content);
   562	
   563	      return {
   564	        content,
   565	        usage: usage || { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
   566	        model: respModel,
   567	        provider: provider.id,
   568	        latency_ms: Date.now() - start,
   569	        ai_powered: true
   570	      };
   571	    } finally {
   572	      clearTimeout(timeoutId);
   573	    }
   574	  }
   575	
   576	  _getExpertSystemPrompt(expertType) {
   577	    const prompts = {
   578	      algorithm: '你是一位资深算法专家，擅长分析算法复杂度、优化方案和代码实现。请以专业、精准的方式回答。',
   579	      architecture: '你是一位系统架构专家，精通企业级系统设计、微服务架构和分布式系统。请提供清晰、可落地的架构建议。',
   580	      data: '你是一位数据专家，精通数据建模、数据治理、ETL流程和数据可视化。请给出专业的数据方案。',
   581	      ai: '你是一位AI专家，精通机器学习、深度学习、大模型应用和AI工程化。请提供前沿且实用的AI建议。',
   582	      workflow: '你是一位工作流专家，精通BPMN、流程编排、自动化引擎和业务流程优化。请设计高效的工作流方案。',
   583	      operator: '你是一位算子系统专家，精通算子抽象、算子组合、状态向量空间和守恒律。请提供符合算子系统数学公理的方案。',
   584	      graph: '你是一位知识图谱专家，精通图算法、实体关系抽取、图谱构建和图神经网络。请提供专业的图谱方案。',
   585	      security: '你是一位安全专家，精通应用安全、数据安全、网络安全和合规审计。请给出全面的安全建议。',
   586	      performance: '你是一位性能优化专家，精通性能分析、瓶颈定位、优化策略和容量规划。请提供量化的性能方案。',
   587	      monitor: '你是一位可观测性专家，精通监控体系、告警策略、日志分析和链路追踪。请设计完善的监控方案。',
   588	      market: '你是一位商业智能专家，精通市场分析、用户画像、推荐系统和商业化策略。请提供商业洞察。',
   589	      mcp: '你是一位MCP协议专家，精通Model Context Protocol设计、工具集成和跨平台兼容。请提供标准的MCP方案。',
   590	      automation: '你是一位自动化专家，精通RPA、流程自动化、智能体工作流和低代码平台。请提供端到端自动化方案。',
   591	      requirement: '你是一位需求工程专家，精通需求分析、需求建模、需求追踪和需求编译。请提供结构化的需求方案。',
   592	      fusion: '你是一位融合专家，精通璇玑体系、双十四维治理、mox 模块化系统架构融合和跨系统集成。请提供mox 模块化系统架构融合方案。',
   593	      default: '你是一位智能助手，可以帮助进行系统分析、代码实现、架构设计和问题解决。请以专业、精准的方式回答。'
   594	    };
   595	    return prompts[expertType] || prompts.default;
   596	  }
   597	
   598	  async _callExternalProvider(provider, messages, temperature, maxTokens) {
   599	    const url = provider.base_url || 'https://api.openai.com/v1';
   600	    const model = provider.model || 'gpt-4';
   601	    const apiKey = decryptApiKey(provider.api_key);
   602	
   603	    const payload = {
   604	      model,
   605	      messages,
   606	      temperature,
   607	      max_tokens: maxTokens
   608	    };
   609	
   610	    let lastError = null;
   611	    for (let attempt = 1; attempt <= this.maxRetries; attempt++) {
   612	      try {
   613	        const controller = new AbortController();
   614	        const timeoutId = setTimeout(() => controller.abort(), this.requestTimeout);
   615	
   616	        const response = await fetch(`${url}/chat/completions`, {
   617	          method: 'POST',
   618	          headers: {
   619	            'Content-Type': 'application/json',
   620	            'Authorization': `Bearer ${apiKey}`
   621	          },
   622	          body: JSON.stringify(payload),
   623	          signal: controller.signal
   624	        });
   625	
   626	        clearTimeout(timeoutId);
   627	
   628	        if (!response.ok) {
   629	          throw new Error(`LLM API error: ${response.status}`);
   630	        }
   631	
   632	        const data = await response.json();
   633	        
   634	        this._recordUsage(provider.id || provider.provider, data.usage || {});
   635	        this._logRequest(provider.id || provider.provider, 'success', Date.now() - (this._requestStart || Date.now()));
   636	
   637	        return {
   638	          content: data.choices[0].message.content,
   639	          usage: data.usage,
   640	          model: data.model,
   641	          provider: provider.id
   642	        };
   643	      } catch (error) {
   644	        lastError = error;
   645	        if (attempt < this.maxRetries) {
   646	          const delay = Math.pow(2, attempt) * 100;
   647	          await new Promise(resolve => setTimeout(resolve, delay));
   648	        }
   649	      }
   650	    }
   651	
   652	    this._logRequest(provider.id || provider.provider, 'failed', 0, lastError?.message);
   653	    console.warn('[llm-gateway] External provider failed after retries, falling back to local:', lastError?.message);
   654	    return this._generateIntelligentResponse(messages, null, []);
   655	  }
   656	
   657	  _recordUsage(providerId, usage) {
   658	    if (!this.usage[providerId]) {
   659	      this.usage[providerId] = { total_tokens: 0, prompt_tokens: 0, completion_tokens: 0, requests: 0, last_updated: null };
   660	    }
   661	    const u = this.usage[providerId];
   662	    u.total_tokens += usage.total_tokens || 0;
   663	    u.prompt_tokens += usage.prompt_tokens || 0;
   664	    u.completion_tokens += usage.completion_tokens || 0;
   665	    u.requests += 1;
   666	    u.last_updated = new Date().toISOString();
   667	    writeJSON('llm_usage.json', this.usage);
   668	  }
   669	
   670	  _logRequest(providerId, status, latency, error) {
   671	    const log = {
   672	      provider: providerId,
   673	      status,
   674	      latency_ms: latency,
   675	      error: error || null,
   676	      timestamp: new Date().toISOString()
   677	    };
   678	    this.requestLog.push(log);
   679	    if (this.requestLog.length > 1000) {
   680	      this.requestLog = this.requestLog.slice(-500);
   681	    }
   682	  }
   683	
   684	  getUsage() {
   685	    return this.usage;
   686	  }
   687	
   688	  getRequestLog(limit = 50) {
   689	    return this.requestLog.slice(-limit).reverse();
   690	  }
   691	
   692	  async testConnection(providerId) {
   693	    const provider = this.providers[providerId];
   694	    if (!provider) {
   695	      return { success: false, message: 'Provider not found' };
   696	    }
   697	
   698	    if (provider.provider === 'local' || provider.type === 'local') {
   699	      return { success: true, message: '本地引擎正常', latencyMs: 0, provider: providerId };
   700	    }
   701	
   702	    const startTime = Date.now();
   703	    try {
   704	      const url = provider.base_url;
   705	      const apiKey = decryptApiKey(provider.api_key);
   706	      const response = await fetch(`${url}/models`, {
   707	        method: 'GET',
   708	        headers: {
   709	          'Authorization': `Bearer ${apiKey}`
   710	        }
   711	      });
   712	
   713	      const latencyMs = Date.now() - startTime;
   714	
   715	      if (!response.ok) {
   716	        let errorMsg = `HTTP ${response.status}`;
   717	        if (response.status === 401) errorMsg = 'API Key 无效或未授权';
   718	        else if (response.status === 429) errorMsg = '请求频率超限，请稍后重试';
   719	        else if (response.status === 404) errorMsg = 'API 端点不存在，请检查 Base URL';
   720	        
   721	        return {
   722	          success: false,
   723	          message: errorMsg,
   724	          latencyMs,
   725	          provider: providerId,
   726	          statusCode: response.status
   727	        };
   728	      }
   729	
   730	      const data = await response.json().catch(() => ({}));
   731	      const models = data.data ? data.data.map(m => m.id || m.id) : [];
   732	
   733	      return {
   734	        success: true,
   735	        message: `连接成功，检测到 ${models.length} 个可用模型`,
   736	        latencyMs,
   737	        provider: providerId,
   738	        models: models.slice(0, 20)
   739	      };
   740	    } catch (error) {
   741	      const latencyMs = Date.now() - startTime;
   742	      return {
   743	        success: false,
   744	        message: `连接失败: ${error.message}`,
   745	        latencyMs,
   746	        provider: providerId
   747	      };
   748	    }
   749	  }
   750	
   751	  async discoverModels(providerId) {
   752	    const provider = this.providers[providerId];
   753	    if (!provider || !provider.base_url) {
   754	      return { success: false, models: [] };
   755	    }
   756	
   757	    if (provider.provider === 'local' || provider.type === 'local') {
   758	      return { success: true, models: ['ous-internal-v3'] };
   759	    }
   760	
   761	    try {
   762	      const url = provider.base_url;
   763	      const apiKey = decryptApiKey(provider.api_key);
   764	      const response = await fetch(`${url}/models`, {
   765	        method: 'GET',
   766	        headers: {
   767	          'Authorization': `Bearer ${apiKey}`
   768	        }
   769	      });
   770	
   771	      if (!response.ok) {
   772	        return { success: false, models: [], message: `HTTP ${response.status}` };
   773	      }
   774	
   775	      const data = await response.json().catch(() => ({}));
   776	      const models = data.data ? data.data.map(m => ({
   777	        id: m.id,
   778	        name: m.id,
   779	        owned_by: m.owned_by,
   780	        context_window: m.context_window || m.max_context_window || 0
   781	      })) : [];
   782	
   783	      return { success: true, models };
   784	    } catch (error) {
   785	      return { success: false, models: [], message: error.message };
   786	    }
   787	  }
   788	
   789	  getHealth() {
   790	    const providers = Object.values(this.providers);
   791	    const enabledCount = providers.filter(p => p.enabled).length;
   792	    const externalCount = providers.filter(p => p.provider !== 'local').length;
   793	    
   794	    return {
   795	      total_providers: providers.length,
   796	      enabled_providers: enabledCount,
   797	      external_providers: externalCount,
   798	      active_provider: this.activeProvider,
   799	      active_provider_name: this.providers[this.activeProvider]?.name || '无',
   800	      status: enabledCount > 0 ? 'ready' : 'no_provider',
   801	      local_available: !!this.providers[Object.keys(this.providers).find(k => this.providers[k].provider === 'local')]
   802	    };
   803	  }
   804	
   805	  getPresetProviders() {
   806	    return Object.entries(PROVIDER_PRESETS).map(([key, preset]) => ({
   807	      id: key,
   808	      name: preset.name,
   809	      base_url: preset.base_url,
   810	      models: preset.models,
   811	      description: preset.description
   812	    }));
   813	  }
   814	
   815	  listProviders() {
   816	    return Object.entries(this.providers).map(([id, p]) => ({
   817	      id,
   818	      name: p.name || id,
   819	      type: p.provider,
   820	      enabled: p.enabled,
   821	      active: id === this.activeProvider,
   822	      model: p.model,
   823	      base_url: p.base_url,
   824	      has_key: !!(p.api_key && p.api_key.trim()),
   825	      api_key_masked: maskApiKey(p.api_key),
   826	      description: p.description,
   827	      updated_at: p.updated_at,
   828	      created_at: p.created_at
   829	    }));
   830	  }
   831	
   832	  getProvider(providerId) {
   833	    const provider = this.providers[providerId];
   834	    if (provider) {
   835	      return {
   836	        ...provider,
   837	        api_key_masked: maskApiKey(provider.api_key)
   838	      };
   839	    }
   840	    return null;
   841	  }
   842	
   843	  setActiveProvider(providerId) {
   844	    if (this.providers[providerId]) {
   845	      this.activeProvider = providerId;
   846	      const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
   847	      writeJSON('llm_config.json', config);
   848	      return true;
   849	    }
   850	    return false;
   851	  }
   852	
   853	  addProvider(provider) {
   854	    const id = provider.id || `llm_${Date.now()}`;
   855	    const now = new Date().toISOString();
   856	    
   857	    const preset = provider.provider && PROVIDER_PRESETS[provider.provider];
   858	    
   859	    const encryptedKey = provider.api_key ? encryptApiKey(provider.api_key) : '';
   860	    
   861	    this.providers[id] = {
   862	      id,
   863	      provider: provider.provider || 'custom',
   864	      base_url: provider.base_url || (preset ? preset.base_url : ''),
   865	      model: provider.model || (preset && preset.models ? preset.models[0] : 'default'),
   866	      api_key: encryptedKey,
   867	      enabled: provider.enabled || false,
   868	      name: provider.name || (preset ? preset.name : id),
   869	      description: provider.description || (preset ? preset.description : ''),
   870	      temperature: provider.temperature || 0.7,
   871	      max_tokens: provider.max_tokens || 2048,
   872	      updated_at: now,
   873	      created_at: now
   874	    };
   875	    
   876	    const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
   877	    writeJSON('llm_config.json', config);
   878	    
   879	    if (provider.enabled && !this.activeProvider) {
   880	      this.activeProvider = id;
   881	    }
   882	    return id;
   883	  }
   884	
   885	  updateProvider(providerId, updates) {
   886	    if (!this.providers[providerId]) return false;
   887	    
   888	    const allowedFields = ['name', 'base_url', 'model', 'enabled', 'description', 'temperature', 'max_tokens'];
   889	    for (const field of allowedFields) {
   890	      if (updates[field] !== undefined) {
   891	        this.providers[providerId][field] = updates[field];
   892	      }
   893	    }
   894	    
   895	    if (updates.api_key !== undefined) {
   896	      if (updates.api_key === '') {
   897	        this.providers[providerId].api_key = '';
   898	      } else {
   899	        this.providers[providerId].api_key = encryptApiKey(updates.api_key);
   900	      }
   901	    }
   902	    
   903	    this.providers[providerId].updated_at = new Date().toISOString();
   904	    
   905	    const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
   906	    writeJSON('llm_config.json', config);
   907	    return true;
   908	  }
   909	
   910	  removeProvider(providerId) {
   911	    if (this.providers[providerId]) {
   912	      delete this.providers[providerId];
   913	      const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
   914	      writeJSON('llm_config.json', config);
   915	      if (this.activeProvider === providerId) {
   916	        const firstKey = Object.keys(this.providers)[0];
   917	        this.activeProvider = firstKey || null;
   918	        if (this.activeProvider && !this.providers[this.activeProvider].enabled) {
   919	          this.providers[this.activeProvider].enabled = true;
   920	        }
   921	      }
   922	      return true;
   923	    }
   924	    return false;
   925	  }
   926	
   927	  enableProvider(providerId) {
   928	    if (!this.providers[providerId]) return false;
   929	    this.providers[providerId].enabled = true;
   930	    if (!this.activeProvider) {
   931	      this.activeProvider = providerId;
   932	    }
   933	    const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
   934	    writeJSON('llm_config.json', config);
   935	    return true;
   936	  }
   937	
   938	  disableProvider(providerId) {
   939	    if (!this.providers[providerId]) return false;
   940	    this.providers[providerId].enabled = false;
   941	    if (this.activeProvider === providerId) {
   942	      const nextProvider = Object.keys(this.providers).find(k => this.providers[k].enabled);
   943	      this.activeProvider = nextProvider || null;
   944	    }
   945	    const config = Object.values(this.providers).map(({ api_key, ...rest }) => rest);
   946	    writeJSON('llm_config.json', config);
   947	    return true;
   948	  }
   949	
   950	  getRoutingConfig() {
   951	    return readJSON('llm_routing.json', {
   952	      strategy: 'latency-warm', // O1：默认启用 latency-warm（优于 legacy priority）
   953	      providers: Object.keys(this.providers).filter(k => this.providers[k].enabled),
   954	      fallback: true,
   955	      load_balance: false,
   956	      weights: {},
   957	      // O1 可调参数（与 LatencyWarmRouter options 对齐）
   958	      warm: {
   959	        alpha: 0.2,
   960	        warmEveryN: 50,
   961	        warmTopK: 2,
   962	        pingTimeoutMs: 400,
   963	      },
   964	      loadBalanceStrategy: 'random', // random / round_robin / least_latency_ewma
   965	    });
   966	  }
   967	
   968	  updateRoutingConfig(config) {
   969	    // O1：更新后重置 LatencyWarmRouter，以便下一次 chat() 重新初始化
   970	    this._warmRouter = null;
   971	    return writeJSON('llm_routing.json', config);
   972	  }
   973	
   974	  // ---- O1 路由选择：按 getRoutingConfig().strategy 返回候选 provider ID 数组 ----
   975	  _ensureWarmRouter() {
   976	    if (!this._warmRouter) {
   977	      const cfg = this.getRoutingConfig();
   978	      this._warmRouter = new LatencyWarmRouter(this.providers, Object.assign({
   979	        alpha: 0.2, warmEveryN: 50, warmTopK: 2, pingTimeoutMs: 400
   980	      }, (cfg && cfg.warm) || {}));
   981	    }
   982	    return this._warmRouter;
   983	  }
   984	
   985	  /**
   986	   * O1：返回排序后的 provider 候选列表（按当前策略）。
   987	   *   - priority：按 PROVIDER_CAPABILITY_SCORE + p.priority 数值排序，降序。
   988	   *   - fallback：按 priority 排序（fallback=true 语义保留，失败降级由 chat/_callExternalProvider 执行）
   989	   *   - latency-warm：使用 LatencyWarmRouter.rankedEnabledIds
   990	   */
   991	  _candidateProviders(strategy) {
   992	    const enabledAll = Object.entries(this.providers)
   993	      .filter(([id, p]) => p && p.enabled !== false && p.provider !== 'local')
   994	      .map(([id, p]) => id);
   995	    const priScore = (id) => {
   996	      const p = this.providers[id];
   997	      if (typeof p.priority === 'number') return p.priority;
   998	      return PROVIDER_CAPABILITY_SCORE[p.provider] ?? PROVIDER_CAPABILITY_SCORE._default;
   999	    };
  1000	    switch (strategy) {
  1001	      case 'latency-warm': {
  1002	        const r = this._ensureWarmRouter();
  1003	        const ranked = r.rankedEnabledIds().filter(id => enabledAll.includes(id));
  1004	        return ranked.length ? ranked : enabledAll.sort((a,b) => priScore(b)-priScore(a));
  1005	      }
  1006	      case 'priority':
  1007	      case 'fallback':
  1008	      default:
  1009	        return enabledAll.sort((a,b) => priScore(b)-priScore(a));
  1010	    }
  1011	  }
  1012	
  1013	  _generateIntelligentResponse(messages, expertType, history) {
  1014	    const lastMsg = messages.filter(m => m.role === 'user').pop()?.content || '';
  1015	    const context = [...history, ...messages];
  1016	    
  1017	    const expertInsights = this._getExpertInsights(expertType);
  1018	    const intentAnalysis = this._analyzeIntent(lastMsg);
  1019	    const relatedOperators = this._findRelatedOperators(lastMsg);
  1020	    
  1021	    const response = {
  1022	      content: this._composeResponse(lastMsg, expertType, intentAnalysis, expertInsights, relatedOperators),
  1023	      usage: { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
  1024	      model: 'ous-internal-v3',
  1025	      provider: 'local-intelligent',
  1026	      metadata: {
  1027	        intent: intentAnalysis.primary,
  1028	        confidence: intentAnalysis.confidence,
  1029	        expert_type: expertType || 'general',
  1030	        related_operators: relatedOperators
  1031	      }
  1032	    };
  1033	
  1034	    return response;
  1035	  }
  1036	
  1037	  _analyzeIntent(text) {
  1038	    const lower = text.toLowerCase();
  1039	    const intents = {
  1040	      operator_recommendation: /推荐|算子|operator|建议|用什么/.test(lower),
  1041	      algorithm_analysis: /算法|复杂度|时间|空间|algorithm|complexity/.test(lower),
  1042	      graph_analysis: /图谱|节点|边|中心性|社区|pagerank|graph|node/.test(lower),
  1043	      workflow: /工作流|编排|流程|pipeline|workflow|flow/.test(lower),
  1044	      automation: /自动化|automation|自动|机器人/.test(lower),
  1045	      requirement: /需求|编译|草莓|caomei|requirement/.test(lower),
  1046	      browser: /浏览器|页面|爬取|抓取|browser|web/.test(lower),
  1047	      market: /商城|市场|下载|购买|market/.test(lower),
  1048	      mcp: /mcp|兼容|协议|protocol/.test(lower),
  1049	      monitor: /监控|日志|指标|报警|monitor|metric/.test(lower),
  1050	      security: /安全|权限|审计|security|permission/.test(lower),
  1051	      performance: /性能|优化|加速|快|performance|optimize/.test(lower),
  1052	      fusion: /融合|璇玑|mox 模块化系统架构|治理|fusion|mox/.test(lower),
  1053	      ai_chat: /你好|hello|hi|介绍|说明/.test(lower)
  1054	    };
  1055	
  1056	    const matched = Object.entries(intents)
  1057	      .filter(([, v]) => v)
  1058	      .map(([k]) => k);
  1059	
  1060	    if (matched.length === 0) {
  1061	      return { primary: 'general', confidence: 0.6 };
  1062	    }
  1063	
  1064	    return { primary: matched[0], secondary: matched.slice(1), confidence: 0.85 };
  1065	  }
  1066	
  1067	  _getExpertInsights(expertType) {
  1068	    const insights = {
  1069	      algorithm: ['时间复杂度分析', '空间复杂度评估', '边界条件处理', '优化建议'],
  1070	      architecture: ['分层架构设计', '服务拆分策略', '数据流向分析', '扩展性评估'],
  1071	      data: ['数据模型设计', '数据质量规则', '数据血缘追踪', '数据安全策略'],
  1072	      ai: ['模型选型建议', '训练策略', '推理优化', 'AI治理框架'],
  1073	      workflow: ['流程建模', '节点编排', '异常处理', '性能调优'],
  1074	      operator: ['算子抽象层次', '组合算子设计', '守恒律验证', '状态向量管理'],
  1075	      graph: ['图构建策略', '实体关系抽取', '图算法选型', '图谱应用场景'],
  1076	      security: ['威胁建模', '防护策略', '审计日志', '合规检查'],
  1077	      performance: ['瓶颈分析', '优化方案', '基准测试', '容量规划'],
  1078	      monitor: ['监控指标', '告警规则', '日志采集', '链路追踪'],
  1079	      market: ['市场分析', '竞品对比', '定价策略', '增长模型'],
  1080	      mcp: ['工具协议', '能力描述', '会话管理', '错误处理'],
  1081	      automation: ['自动化策略', '触发条件', '执行监控', '异常恢复'],
  1082	      requirement: ['需求拆解', '优先级排序', '验收标准', '追踪矩阵'],
  1083	      fusion: ['璇玑配置', '维度权重', '融合策略', '治理闸门']
  1084	    };
  1085	    return insights[expertType] || insights.default || ['系统分析', '方案设计', '实施建议'];
  1086	  }
  1087	
  1088	  _findRelatedOperators(text) {
  1089	    const lower = text.toLowerCase();
  1090	    const operators = [
  1091	      { id: 'normalize', name: 'L2归一化', tags: ['归一化', 'normalize', '向量'] },
  1092	      { id: 'relu', name: 'ReLU激活', tags: ['激活', 'relu', '非线性'] },
  1093	      { id: 'sigmoid', name: 'Sigmoid', tags: ['压缩', 'sigmoid', '概率'] },
  1094	      { id: 'softmax', name: 'Softmax', tags: ['概率', 'softmax', '分类'] },
  1095	      { id: 'linear', name: '线性变换', tags: ['线性', '缩放', '变换'] },
  1096	      { id: 'pagerank', name: 'PageRank', tags: ['排名', 'pagerank', '影响力'] },
  1097	      { id: 'label_propagation', name: '标签传播', tags: ['社区', '传播', '聚类'] },
  1098	      { id: 'bfs', name: '广度优先搜索', tags: ['路径', '搜索', 'bfs'] },
  1099	      { id: 'activate', name: '激活传播', tags: ['传播', '能量', '激活'] },
  1100	      { id: 'degree_centrality', name: '度中心性', tags: ['中心性', '度', 'degree'] }
  1101	    ];
  1102	
  1103	    return operators.filter(op => 
  1104	      op.tags.some(tag => lower.includes(tag.toLowerCase()))
  1105	    ).slice(0, 3);
  1106	  }
  1107	
  1108	  _composeResponse(userText, expertType, intentAnalysis, insights, relatedOperators) {
  1109	    const text = userText.trim();
  1110	    const expertLabel = expertType ? {
  1111	      algorithm: '算法专家',
  1112	      architecture: '架构专家',
  1113	      data: '数据专家',
  1114	      ai: 'AI专家',
  1115	      workflow: '工作流专家',
  1116	      operator: '算子系统专家',
  1117	      graph: '知识图谱专家',
  1118	      security: '安全专家',
  1119	      performance: '性能优化专家',
  1120	      monitor: '可观测性专家',
  1121	      market: '商业智能专家',
  1122	      mcp: 'MCP协议专家',
  1123	      automation: '自动化专家',
  1124	      requirement: '需求工程专家',
  1125	      fusion: '融合专家'
  1126	    }[expertType] : '智能助手';
  1127	
  1128	    const greetingPatterns = [/你好/, /hello/i, /hi/i, /在吗/, /介绍/];
  1129	    if (greetingPatterns.some(p => p.test(text))) {
  1130	      return `你好！我是${expertLabel}。\n\n我可以帮你：\n${this._getCapabilitiesList(expertType)}\n\n请告诉我你的具体需求，我会以专业的方式为你解答。`;
  1131	    }
  1132	
  1133	    const parts = [];
  1134	    
  1135	    parts.push(`## ${expertLabel}分析\n\n针对你的问题"${text.slice(0, 50)}${text.length > 50 ? '...' : ''}"，我的分析如下：\n`);
  1136	
  1137	    if (intentAnalysis.primary && intentAnalysis.primary !== 'general') {
  1138	      parts.push(`**识别意图**: ${this._translateIntent(intentAnalysis.primary)}（置信度 ${Math.round(intentAnalysis.confidence * 100)}%）\n`);
  1139	    }
  1140	
  1141	    parts.push(`### 核心分析\n`);
  1142	    parts.push(this._generateExpertAnalysis(text, expertType, intentAnalysis));
  1143	
  1144	    if (insights && insights.length) {
  1145	      parts.push(`\n### 专家洞察\n`);
  1146	      insights.slice(0, 4).forEach((insight, i) => {
  1147	        parts.push(`${i + 1}. **${insight}**: ${this._expandInsight(insight, expertType)}`);
  1148	      });
  1149	    }
  1150	
  1151	    if (relatedOperators && relatedOperators.length) {
  1152	      parts.push(`\n### 推荐算子\n`);
  1153	      relatedOperators.forEach(op => {
  1154	        parts.push(`- \`${op.id}\` (${op.name}) — 可用于解决此类问题`);
  1155	      });
  1156	    }
  1157	
  1158	    parts.push(`\n### 下一步建议\n`);
  1159	    parts.push(this._getNextSteps(text, expertType));
  1160	
  1161	    return parts.join('\n');
  1162	  }
  1163	
  1164	  _getCapabilitiesList(expertType) {
  1165	    const caps = {
  1166	      algorithm: ['算法复杂度分析', '优化方案设计', '代码实现建议', '边界条件检查'],
  1167	      architecture: ['系统架构设计', '微服务拆分', '技术选型建议', '扩展性评估'],
  1168	      data: ['数据模型设计', '数据治理方案', 'ETL流程设计', '数据安全策略'],
  1169	      ai: ['AI模型选型', '训练策略建议', '推理优化方案', 'AI应用设计'],
  1170	      workflow: ['工作流建模', '节点编排建议', '异常处理设计', '性能优化'],
  1171	      operator: ['算子抽象设计', '组合算子开发', '守恒律验证', '状态向量管理'],
  1172	      graph: ['图谱构建方案', '实体关系抽取', '图算法选型', '应用场景设计'],
  1173	      security: ['威胁建模', '安全防护策略', '审计日志设计', '合规检查方案'],
  1174	      performance: ['性能瓶颈分析', '优化方案设计', '基准测试建议', '容量规划'],
  1175	      monitor: ['监控体系设计', '告警规则配置', '日志采集方案', '链路追踪设计'],
  1176	      market: ['市场趋势分析', '竞品对比', '定价策略建议', '增长模型设计'],
  1177	      mcp: ['MCP工具设计', '协议兼容方案', '能力描述规范', '错误处理设计'],
  1178	      automation: ['自动化策略设计', '触发条件配置', '执行监控方案', '异常恢复设计'],
  1179	      requirement: ['需求拆解分析', '优先级排序', '验收标准定义', '追踪矩阵设计'],
  1180	      fusion: ['璇玑配置方案', '维度权重设计', '融合策略制定', '治理闸门配置']
  1181	    };
  1182	    return (caps[expertType] || caps.default || ['系统分析', '方案设计', '实施建议'])
  1183	      .map(c => `- ${c}`).join('\n');
  1184	  }
  1185	
  1186	  _translateIntent(intent) {
  1187	    const map = {
  1188	      operator_recommendation: '算子推荐',
  1189	      algorithm_analysis: '算法分析',
  1190	      graph_analysis: '图谱分析',
  1191	      workflow: '工作流',
  1192	      automation: '自动化',
  1193	      requirement: '需求编译',
  1194	      browser: '浏览器自动化',
  1195	      market: '算子商城',
  1196	      mcp: 'MCP兼容',
  1197	      monitor: '系统监控',
  1198	      security: '安全审计',
  1199	      performance: '性能优化',
  1200	      fusion: 'mox 模块化系统架构融合',
  1201	      ai_chat: 'AI对话',
  1202	      general: '通用咨询'
  1203	    };
  1204	    return map[intent] || intent;
  1205	  }
  1206	
  1207	  _generateExpertAnalysis(text, expertType, intent) {
  1208	    const analyses = {
  1209	      operator_recommendation: '这是一个算子推荐场景。我会根据你的需求特征（输入类型、输出目标、性能要求），从算子库中筛选最合适的算子组合。关键考量维度包括：算子类型匹配度、参数适配性、守恒律兼容性。',
  1210	      algorithm_analysis: '算法分析需要从时间复杂度和空间复杂度两个维度进行评估。我建议采用大O表示法进行标准化度量，并考虑实际运行环境的硬件约束。对于大规模数据场景，还需要评估常数因子和缓存友好性。',
  1211	      graph_analysis: '知识图谱分析涉及图结构的多个维度：节点影响力（PageRank）、社区结构（标签传播）、关键路径（BFS最短路径）、节点中心性（度/中介中心性）。我建议根据具体业务场景选择合适的分析方法。',
  1212	      workflow: '工作流编排需要考虑：节点依赖关系、并行执行路径、异常处理机制、状态回滚策略。我建议采用有向无环图（DAG）进行流程建模，并为每个节点定义明确的输入输出契约。',
  1213	      general: '基于你的问题，我会从以下维度进行分析：问题理解、方案设计、实施路径、风险评估。让我逐步为你展开。'
  1214	    };
  1215	    return analyses[intent.primary] || analyses.general;
  1216	  }
  1217	
  1218	  _expandInsight(insight, expertType) {
  1219	    const expansions = {
  1220	      '时间复杂度分析': '使用大O表示法，关注最坏情况和平均情况的差异，考虑数据规模增长对性能的影响',
  1221	      '空间复杂度评估': '分析内存使用模式，评估缓存友好性，考虑空间换时间的可行性',
  1222	      '分层架构设计': '采用关注点分离原则，将系统划分为表现层、业务层、数据层、基础设施层',
  1223	      '数据模型设计': '遵循第三范式，同时考虑查询性能进行适度反范式化设计',
  1224	      '算子抽象层次': '定义清晰的算子接口，支持组合和复用，确保类型安全',
  1225	      '流程建模': '使用BPMN 2.0标准建模，明确网关类型（排他/并行/包容）',
  1226	      '图谱构建策略': '采用增量构建方式，定义实体和关系的语义类型',
  1227	      '威胁建模': '使用STRIDE模型识别威胁，定义攻击面和防护措施',
  1228	      '瓶颈分析': '通过性能剖析定位热点代码，使用火焰图可视化性能分布',
  1229	      '监控指标': '定义黄金指标（延迟/流量/错误/饱和度），设计分层监控体系',
  1230	      'MCP工具设计': '遵循Model Context Protocol规范，定义工具能力描述',
  1231	      '自动化策略': '基于事件驱动设计自动化触发器，定义执行条件和后置动作',
  1232	      '需求拆解': '将高层需求逐层分解为可执行的用户故事，定义验收标准',
  1233	      '璇玑配置': '根据业务特性配置双十四维权重，定义治理闸门阈值'
  1234	    };
  1235	    return expansions[insight] || '需要根据具体场景进行深入分析和定制化设计';
  1236	  }
  1237	
  1238	  _getNextSteps(text, expertType) {
  1239	    const steps = {
  1240	      operator: [
  1241	        '1. 明确输入数据类型和维度',
  1242	        '2. 选择算子基类（FunctionOperator/LinearOperator）',
  1243	        '3. 定义算子元数据（输入/输出类型、参数）',
  1244	        '4. 实现算子逻辑并注册到算子中心',
  1245	        '5. 在工作流编排中测试算子'
  1246	      ],
  1247	      workflow: [
  1248	        '1. 使用流程图编辑器设计节点和连线',
  1249	        '2. 为每个节点选择合适的算子',
  1250	        '3. 配置节点参数和条件分支',
  1251	        '4. 验证流程并执行测试',
  1252	        '5. 在璇玑治理中进行mox 模块化系统架构优化'
  1253	      ],
  1254	      graph: [
  1255	        '1. 定义节点类型和关系类型',
  1256	        '2. 添加初始节点和边',
  1257	        '3. 运行图谱分析算法（PageRank/社区发现）',
  1258	        '4. 查看可视化结果',
  1259	        '5. 在图谱管理中进行持续维护'
  1260	      ],
  1261	      default: [
  1262	        '1. 明确需求边界和目标',
  1263	        '2. 选择相关模块开始实施',
  1264	        '3. 在AI助手对话框中获取更多帮助',
  1265	        '4. 查看系统文档了解详细用法',
  1266	        '5. 联系专家获得一对一咨询'
  1267	      ]
  1268	    };
  1269	    return (steps[expertType] || steps.default).join('\n');
  1270	  }
  1271	
  1272	  _generateLocalGraph(topic, description) {
  1273	    const t = (topic || '企业官网需求').toString();
  1274	    const d = (description || '').toString();
  1275	    
  1276	    const templates = {
  1277	      '企业官网': {
  1278	        nodes: [
  1279	          { id: 'concept_website', label: '企业官网', type: '概念', description: '企业官方网站建设项目', attributes: { category: '项目' } },
  1280	          { id: 'concept_user', label: '用户', type: '角色', description: '访问官网的终端用户', attributes: { role: '访客' } },
  1281	          { id: 'concept_admin', label: '管理员', type: '角色', description: '负责官网内容维护的管理员', attributes: { role: '运营' } },
  1282	          { id: 'component_frontend', label: '前端界面', type: '组件', description: '用户可见的Web界面', attributes: { tech: 'Vue.js' } },
  1283	          { id: 'component_backend', label: '后端服务', type: '组件', description: '提供API和数据处理能力', attributes: { tech: 'Node.js' } },
  1284	          { id: 'component_database', label: '数据库', type: '组件', description: '存储用户、内容和业务数据', attributes: { tech: 'MySQL' } },
  1285	          { id: 'component_cms', label: '内容管理系统', type: '组件', description: '支持管理员发布和管理内容', attributes: { feature: '富文本' } },
  1286	          { id: 'component_auth', label: '认证授权', type: '组件', description: '用户登录注册和权限管理', attributes: { method: 'JWT' } },
  1287	          { id: 'component_search', label: '搜索功能', type: '组件', description: '站内全文搜索能力', attributes: { engine: 'Elasticsearch' } },
  1288	          { id: 'process_deploy', label: '部署流程', type: '流程', description: '从开发到上线的完整流程', attributes: { method: 'CI/CD' } },
  1289	          { id: 'process_design', label: '设计流程', type: '流程', description: 'UI/UX设计和评审流程', attributes: { tool: 'Figma' } },
  1290	          { id: 'process_content', label: '内容运营流程', type: '流程', description: '内容创建、审核、发布流程', attributes: { workflow: '审批制' } },
  1291	          { id: 'data_user', label: '用户数据', type: '数据', description: '用户注册信息、行为数据', attributes: { sensitivity: '高' } },
  1292	          { id: 'data_content', label: '内容数据', type: '数据', description: '文章、产品、新闻等内容', attributes: { sensitivity: '低' } },
  1293	          { id: 'data_config', label: '配置数据', type: '数据', description: '系统配置、权限配置', attributes: { sensitivity: '中' } },
  1294	          { id: 'constraint_security', label: '安全约束', type: '约束', description: '数据加密、XSS防护、CSRF防护', attributes: { level: '必须' } },
  1295	          { id: 'constraint_performance', label: '性能约束', type: '约束', description: '首屏加载<3s，支持高并发', attributes: { level: '重要' } },
  1296	          { id: 'constraint_seo', label: 'SEO约束', type: '约束', description: '搜索引擎优化要求', attributes: { level: '重要' } },
  1297	          { id: 'goal_conversion', label: '转化目标', type: '目标', description: '访客到注册/客户的转化', attributes: { kpi: '转化率' } },
  1298	          { id: 'goal_brand', label: '品牌目标', type: '目标', description: '提升企业品牌形象和知名度', attributes: { kpi: '品牌指数' } }
  1299	        ],
  1300	        edges: [
  1301	          { source: 'concept_website', target: 'concept_user', label: '服务', weight: 1.0 },
  1302	          { source: 'concept_website', target: 'concept_admin', label: '管理', weight: 1.0 },
  1303	          { source: 'concept_website', target: 'component_frontend', label: '包含', weight: 1.0 },
  1304	          { source: 'concept_website', target: 'component_backend', label: '包含', weight: 1.0 },
  1305	          { source: 'concept_website', target: 'component_database', label: '依赖', weight: 1.0 },
  1306	          { source: 'component_frontend', target: 'component_backend', label: '使用', weight: 0.9 },
  1307	          { source: 'component_backend', target: 'component_database', label: '使用', weight: 1.0 },
  1308	          { source: 'component_cms', target: 'component_backend', label: '依赖', weight: 0.8 },
  1309	          { source: 'component_auth', target: 'component_backend', label: '集成', weight: 0.9 },
  1310	          { source: 'component_search', target: 'component_backend', label: '集成', weight: 0.8 },
  1311	          { source: 'component_search', target: 'data_content', label: '搜索', weight: 0.9 },
  1312	          { source: 'concept_admin', target: 'component_cms', label: '使用', weight: 0.9 },
  1313	          { source: 'concept_user', target: 'component_frontend', label: '访问', weight: 1.0 },
  1314	          { source: 'process_design', target: 'component_frontend', label: '产出', weight: 0.8 },
  1315	          { source: 'process_deploy', target: 'component_backend', label: '部署', weight: 0.9 },
  1316	          { source: 'process_content', target: 'component_cms', label: '管理', weight: 0.9 },
  1317	          { source: 'data_user', target: 'component_auth', label: '存储于', weight: 0.9 },
  1318	          { source: 'data_content', target: 'component_cms', label: '存储于', weight: 0.9 },
  1319	          { source: 'data_config', target: 'component_backend', label: '配置于', weight: 0.8 },
  1320	          { source: 'constraint_security', target: 'component_auth', label: '约束', weight: 1.0 },
  1321	          { source: 'constraint_security', target: 'component_backend', label: '约束', weight: 1.0 },
  1322	          { source: 'constraint_performance', target: 'component_frontend', label: '约束', weight: 0.9 },
  1323	          { source: 'constraint_seo', target: 'component_frontend', label: '约束', weight: 0.8 },
  1324	          { source: 'goal_conversion', target: 'concept_user', label: '影响', weight: 0.9 },
  1325	          { source: 'goal_brand', target: 'concept_website', label: '影响', weight: 1.0 }
  1326	        ],
  1327	        summary: '企业官网需求知识图谱覆盖了从用户访问、内容管理到后端服务的完整架构，包含20个核心概念节点和25条关系边，体现了各组件间的依赖和约束关系。'
  1328	      }
  1329	    };
  1330	    
  1331	    for (const [key, template] of Object.entries(templates)) {
  1332	      if (t.includes(key)) {
  1333	        return {
  1334	          nodes: template.nodes.map(n => ({ ...n, id: n.id.replace(key.toLowerCase().replace(/\s+/g, '_'), 'topic') })),
  1335	          edges: template.edges.map(e => ({ ...e })),
  1336	          summary: template.summary
  1337	        };
  1338	      }
  1339	    }
  1340	    
  1341	    const nodes = [
  1342	      { id: 'topic_' + Date.now() + '_root', label: t, type: '概念', description: d || t + '核心概念', attributes: { topic: t } },
  1343	      { id: 'topic_user', label: '用户', type: '角色', description: t + '的使用者', attributes: {} },
  1344	      { id: 'topic_admin', label: '管理员', type: '角色', description: t + '的管理者', attributes: {} },
  1345	      { id: 'topic_frontend', label: '前端组件', type: '组件', description: t + '的前端实现', attributes: {} },
  1346	      { id: 'topic_backend', label: '后端组件', type: '组件', description: t + '的后端实现', attributes: {} },
  1347	      { id: 'topic_data', label: '数据层', type: '组件', description: t + '的数据存储', attributes: {} },
  1348	      { id: 'topic_process', label: '核心流程', type: '流程', description: t + '的核心业务流程', attributes: {} },
  1349	      { id: 'topic_data_flow', label: '数据流', type: '流程', description: t + '的数据流向', attributes: {} },
  1350	      { id: 'topic_constraint', label: '约束条件', type: '约束', description: t + '的关键约束', attributes: {} },
  1351	      { id: 'topic_goal', label: '目标', type: '目标', description: t + '的实现目标', attributes: {} },
  1352	      { id: 'topic_api', label: 'API接口', type: '组件', description: t + '的对外接口', attributes: {} },
  1353	      { id: 'topic_auth', label: '认证授权', type: '组件', description: t + '的安全认证', attributes: {} },
  1354	      { id: 'topic_monitor', label: '监控系统', type: '组件', description: t + '的运行监控', attributes: {} },
  1355	      { id: 'topic_deploy', label: '部署流程', type: '流程', description: t + '的部署上线', attributes: {} },
  1356	      { id: 'topic_data_model', label: '数据模型', type: '数据', description: t + '的数据结构', attributes: {} }
  1357	    ];
  1358	    
  1359	    const edges = [
  1360	      { source: 'topic_' + Date.now() + '_root', target: 'topic_user', label: '服务', weight: 1.0 },
  1361	      { source: 'topic_' + Date.now() + '_root', target: 'topic_admin', label: '管理', weight: 1.0 },
  1362	      { source: 'topic_' + Date.now() + '_root', target: 'topic_frontend', label: '包含', weight: 1.0 },
  1363	      { source: 'topic_' + Date.now() + '_root', target: 'topic_backend', label: '包含', weight: 1.0 },
  1364	      { source: 'topic_' + Date.now() + '_root', target: 'topic_data', label: '依赖', weight: 1.0 },
  1365	      { source: 'topic_frontend', target: 'topic_backend', label: '使用', weight: 0.9 },
  1366	      { source: 'topic_backend', target: 'topic_data', label: '使用', weight: 1.0 },
  1367	      { source: 'topic_backend', target: 'topic_api', label: '提供', weight: 0.9 },
  1368	      { source: 'topic_backend', target: 'topic_auth', label: '集成', weight: 0.9 },
  1369	      { source: 'topic_backend', target: 'topic_monitor', label: '集成', weight: 0.8 },
  1370	      { source: 'topic_user', target: 'topic_frontend', label: '访问', weight: 1.0 },
  1371	      { source: 'topic_admin', target: 'topic_backend', label: '管理', weight: 0.9 },
  1372	      { source: 'topic_process', target: 'topic_backend', label: '实现于', weight: 0.8 },
  1373	      { source: 'topic_data_flow', target: 'topic_process', label: '贯穿', weight: 0.9 },
  1374	      { source: 'topic_data_flow', target: 'topic_data', label: '存储于', weight: 0.9 },
  1375	      { source: 'topic_constraint', target: 'topic_backend', label: '约束', weight: 1.0 },
  1376	      { source: 'topic_constraint', target: 'topic_frontend', label: '约束', weight: 0.9 },
  1377	      { source: 'topic_goal', target: 'topic_user', label: '服务于', weight: 0.9 },
  1378	      { source: 'topic_goal', target: 'topic_' + Date.now() + '_root', label: '达成', weight: 1.0 },
  1379	      { source: 'topic_deploy', target: 'topic_backend', label: '部署', weight: 0.9 },
  1380	      { source: 'topic_deploy', target: 'topic_frontend', label: '部署', weight: 0.9 },
  1381	      { source: 'topic_data_model', target: 'topic_data', label: '定义于', weight: 1.0 },
  1382	      { source: 'topic_api', target: 'topic_frontend', label: '被调用', weight: 0.8 },
  1383	      { source: 'topic_auth', target: 'topic_user', label: '验证', weight: 0.9 },
  1384	      { source: 'topic_monitor', target: 'topic_deploy', label: '监控', weight: 0.8 }
  1385	    ];
  1386	    
  1387	    const rootId = nodes[0].id;
  1388	    const rootEdges = edges.map(e => ({ ...e, source: e.source === 'topic_' + Date.now() + '_root' ? rootId : e.source }));
  1389	    
  1390	    return {
  1391	      nodes,
  1392	      edges: rootEdges,
  1393	      summary: `${t}知识图谱包含${nodes.length}个核心概念和${edges.length}条关系，覆盖了从用户、组件、流程到约束和目标的完整维度。`
  1394	    };
  1395	  }
  1396	}
  1397	
  1398	let gatewayInstance = null;
  1399	
  1400	function getGateway() {
  1401	  if (!gatewayInstance) {
  1402	    gatewayInstance = new LLMGateway();
  1403	  }
  1404	  return gatewayInstance;
  1405	}
  1406	
  1407	module.exports = { LLMGateway, getGateway, PROVIDER_PRESETS, LatencyWarmRouter, ROUTING_STRATEGIES };