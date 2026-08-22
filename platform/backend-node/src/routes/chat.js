'use strict';

/**
 * 路由域：AI 对话
 * /ai/chat 统一对话入口（LLM 网关 + 本地降级回复）
 */
module.exports = function registerChatRoutes(ctx) {
  const { url, gateway, alliance, webSearchService, artifactService, uid, readJSON, writeJSON, ok, readBody, reg, pagerank } = ctx;


  function buildAIReply(input) {
    const text = (input || '').toLowerCase();
    if (text.indexOf('pagerank') !== -1 || text.indexOf('中心性') !== -1) {
      return 'PageRank 是一种基于链接结构的节点影响力评估算法，采用阻尼系数 0.85 的迭代公式：PR(v) = (1-d)/N + d * Σ PR(u)/C(u)。在本系统中可通过 /graph/pagerank 端点实时计算。';
    }
    if (text.indexOf('社区') !== -1 || text.indexOf('community') !== -1) {
      return '本系统使用 Label Propagation 标签传播算法进行社区发现，时间复杂度接近线性，适合大规模图谱。可通过 /graph/communities 调用。';
    }
    if (text.indexOf('璇玑') !== -1) {
      return '双璇玑十四维治理体系：业务侧 7 维 + 研发侧 7 维，通过融合引擎 D04 汇聚并由验证网关 n_gate 进行闸门校验。';
    }
    if (text.indexOf('caomei') !== -1 || text.indexOf('草莓') !== -1 || text.indexOf('需求') !== -1) {
      return 'Caomei 需求编译器将自然语言需求编译为流程蓝图，支持精化迭代与模板复用。';
    }
    if (text.indexOf('你好') !== -1 || text.indexOf('hello') !== -1 || text.indexOf('hi') !== -1) {
      return '你好！当前系统未配置外部 AI 引擎（LLM），所以我还无法进行真正的智能对话。请在「LLM 配置」页面启用并填写 API Key（推荐豆包 doubao-pro / DeepSeek / OpenAI 之一），即可获得真实的 AI 对话能力。本系统支持知识图谱分析、算子执行、浏览器自动化、MCP 兼容等能力。';
    }
    if (text.indexOf('图谱') !== -1 || text.indexOf('graph') !== -1) {
      return '当前图谱包含 23 个节点与 30 条边，覆盖融合引擎、联盟、算子、AI 任务、商城等多种节点类型。可以查询邻居、最短路径或计算中心性。';
    }
    if (text.indexOf('豆包') !== -1 || text.indexOf('doubao') !== -1) {
      return '豆包（Doubao）是字节跳动推出的大语言模型系列，基于豆包大模型底座。在本系统中可通过「算子智能体」的 LLM 网关配置火山引擎 Provider 来调用豆包模型（支持 doubao-pro-32k、doubao-pro-128k、doubao-lite-32k 等）。前往「LLM 配置」页面添加火山引擎 Provider 后即可使用。';
    }
    if (text.indexOf('deepseek') !== -1 || text.indexOf('千问') !== -1 || text.indexOf('qwen') !== -1 || text.indexOf('智谱') !== -1 || text.indexOf('zhipu') !== -1) {
      return '本系统支持多种主流大模型：DeepSeek（深度求索）、千问（阿里云）、智谱AI、豆包（火山引擎）、OpenAI 等。可前往「LLM 配置」页面添加对应 Provider 后使用。所有 API Key 均采用 AES-256-GCM 加密存储。';
    }
    if (text.indexOf('llm') !== -1 || text.indexOf('大模型') !== -1 || text.indexOf('模型') !== -1) {
      return '本系统内置 LLM 网关，支持配置多种大模型 Provider（DeepSeek、火山引擎、阿里云千问、智谱AI、OpenAI 等）。前往「LLM 配置」页面可添加、启用、切换 Provider，并查看用量统计和请求日志。';
    }
    if (text.indexOf('算法') !== -1 || text.indexOf('algorithm') !== -1) {
      return '本系统内置多种图算法实现：PageRank（节点影响力）、Label Propagation（社区发现）、BFS（最短路径）、度中心性、激活传播等。可通过 API 直接调用，也可在 AI 对话中请求算法分析。';
    }
    if (text.indexOf('算子') !== -1 || text.indexOf('operator') !== -1) {
      return '算子（Operator）是本系统的核心抽象，支持函数算子、线性算子、聚合算子等类型。可通过「算子中心」注册和管理算子，在 AI 对话中推荐算子，也可在工作流中编排执行。';
    }
    if (text.indexOf('浏览器') !== -1 || text.indexOf('browser') !== -1) {
      return '本系统支持浏览器自动化能力，可通过 AI 指令自动执行网页操作（导航、点击、提取、截图等）。前往「浏览器自动化」页面创建会话，或在对话中请求浏览器任务。';
    }
    if (text.indexOf('mcp') !== -1) {
      return '本系统兼容 MCP（Model Context Protocol）协议，支持以标准 MCP 工具的形式暴露系统能力（算子、图谱分析、浏览器自动化等）。可通过 /mcp 端点进行工具列表查询和调用。';
    }
    if (text.indexOf('知识') !== -1 || text.indexOf('知识库') !== -1 || text.indexOf('kb') !== -1) {
      return '本系统集成云盘知识库功能，支持文档上传、分类管理、实体抽取、版本对比、语义搜索等能力。可在「知识库」页面管理文档，对话中也可自动将对话内容整理进知识图谱。';
    }
    return `已收到你的请求："${input || ''}"。

本系统是算子统一智能平台，支持以下核心能力：
- 📊 知识图谱分析（PageRank、社区发现、中心性计算）
- 🔌 算子执行与编排（算法算子、数据流算子、工作流算子）
- 🤖 AI 对话（本地智能引擎 + 外部 LLM 网关）
- 🌐 浏览器自动化（网页操作、数据提取）
- 🔗 MCP 协议兼容（标准工具接入）
- 📝 需求编译（Caomei 自然语言 → 流程蓝图）
- 🛒 算子商城（算子市场、模板复用）
- 📚 知识库管理（文档、实体、版本）

请告诉我具体需求，我会为你提供针对性的帮助。`;
  }

  reg('post', '/ai/chat', async (req, res) => {
    const body = await readBody(req);
    const messages = body.messages || (body.message ? [{ role: 'user', content: body.message }] : []);
    const last = messages.length ? messages[messages.length - 1].content : '';
    const sessionId = body.sessionId || body.session_id || uid('sess');

    let reply = null;
    let aiMetadata = null;
    let aiPowered = false;

    // 0. 联网搜索（body.web_search 为真时）：先检索实时信息，再注入 LLM 上下文
    let webSearchContext = null;
    let webSearchInfo = null;
    // 本地制品模式（document / code）：AI 对话中自动在本机创建文档/代码文件
    const artifactMode = body.artifact_mode === 'document' || body.artifact_mode === 'code' ? body.artifact_mode : null;
    const wantWebSearch = !!(body.web_search || body.webSearch);
    if (wantWebSearch && last) {
      if (webSearchService.isReady()) {
        try {
          const searchResult = await webSearchService.search(last);
          webSearchContext = webSearchService.buildSearchContext(last, searchResult);
          webSearchInfo = {
            enabled: true,
            engine: searchResult.engine_name,
            query: last,
            duration_ms: searchResult.duration_ms,
            sources: searchResult.results.map((r) => ({ title: r.title, url: r.url }))
          };
        } catch (e) {
          console.warn('[ai/chat] web search failed, continuing without it:', e.message);
          webSearchInfo = { enabled: false, error: e.message };
        }
      } else {
        webSearchInfo = { enabled: false, error: '联网搜索未启用或未完成配置（可在 LLM 配置页设置）' };
      }
    }

    // 1. 优先尝试专家联盟（指定专家类型时）
    if (body.expertType || body.expert_id) {
      try {
        const expertId = body.expert_id || `${body.expertType}-expert`;
        const expertResult = await alliance.consult(expertId, messages, {
          sessionId,
          temperature: body.temperature,
          maxTokens: body.maxTokens,
          webSearchContext
        });
        reply = expertResult.response;
        aiMetadata = { ...(expertResult.metadata || {}), expert: expertResult.expert, ai_powered: true };
        if (webSearchInfo) aiMetadata.web_search = webSearchInfo;
        aiPowered = true;
        ok(res, { reply, sessionId, expert: expertResult.expert, metadata: aiMetadata });
        return;
      } catch (e) {
        // Fall through to gateway
      }
    }

    // 2. 尝试 LLM 网关（有激活的「真实 AI」 Provider 时）
    //    注意：只有 gateway.isRealAI() 为真（即已配置并启用外部大模型）才走真实调用，
    //    否则不应让本地关键词假回复伪装成 AI 回答。
    const hasRealAI = typeof gateway.isRealAI === 'function' ? gateway.isRealAI() : !!gateway.activeProvider;
    if (!aiPowered && hasRealAI) {
      try {
        const result = await gateway.chat({
          messages,
          sessionId,
          expertType: body.expert_type || body.expertType,
          systemPrompt: body.system_prompt || body.systemPrompt,
          webSearchContext,
          temperature: body.temperature,
          maxTokens: body.maxTokens
        });
        reply = result.content;
        aiPowered = true;
        aiMetadata = {
          ...(result.metadata || {}),
          usage: result.usage,
          model: result.model,
          provider: result.provider,
          ai_powered: true
        };
        if (webSearchInfo) aiMetadata.web_search = webSearchInfo;
      } catch (e) {
        console.warn('[ai/chat] LLM gateway failed, falling back to local:', e.message);
      }
    }

    // 3. 降级到本地兜底（仅在完全未配置任何真实 AI 引擎时）
    if (!aiPowered) {
      reply = buildAIReply(last);
      aiMetadata = { ai_powered: false, fallback: true };
    }

    // 4. 本地制品模式（文档/代码）：五步流水线落盘 + 回执（失败不伤主链路）
    let artifactResult = null;
    if (artifactMode) {
      artifactResult = await artifactService.process({
        mode: artifactMode,
        message: last,
        session_id: sessionId,
        overwrite: !!body.overwrite
      });
      if (artifactResult.created.length) {
        reply += artifactService.buildReplySuffix(artifactResult);
        aiMetadata = aiMetadata || {};
        aiMetadata.artifacts = {
          mode: artifactMode,
          created: artifactResult.created.map((c) => ({
            filename: c.filename,
            rel_path: c.rel_path,
            size: c.size,
            sha256: c.sha256.slice(0, 12),
            overwritten: c.overwritten
          })),
          skipped: artifactResult.skipped
        };
      } else if (artifactResult.skipped.length) {
        aiMetadata = aiMetadata || {};
        aiMetadata.artifacts = { mode: artifactMode, created: [], skipped: artifactResult.skipped };
      }
    }

    // 持久化会话
    const sessions = readJSON('dialogue_sessions.json', []);
    let sess = sessions.find((s) => s.id === sessionId);
    if (!sess) {
      sess = { id: sessionId, title: last.slice(0, 20) || '新会话', messages: [], updatedAt: new Date().toISOString() };
      sessions.push(sess);
    }
    sess.messages = sess.messages.concat([
      { role: 'user', content: last, ts: new Date().toISOString() },
      { role: 'assistant', content: reply, ts: new Date().toISOString(), ai_powered: aiPowered }
    ]);
    sess.updatedAt = new Date().toISOString();
    writeJSON('dialogue_sessions.json', sessions);

    ok(res, { reply, sessionId, metadata: aiMetadata });
  });

};
