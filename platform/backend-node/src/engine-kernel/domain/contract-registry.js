'use strict';

/**
 * 引擎槽位契约注册表（domain 层 · 纯数据零 IO）
 * ------------------------------------------------------------------
 * 核心理念「一切皆可插件化」：系统的每类能力定义为一个槽位（Slot），
 * 每个槽位有一份标准契约（Contract）——方法签名 + 输入输出规范。
 * 任何满足契约的引擎（自研/第三方/云端/本地）都可插入槽位，
 * 切换引擎 = 换绑定，零代码改动，瞬间生效。
 *
 * 契约即文档：本文件就是接口规范文档（GET /engine-kernel/contracts/:slot 原样输出）。
 * 新增槽位三步：
 *   1. 这里登记 SLOT_CONTRACTS（含契约文档）
 *   2. infrastructure/plugin-repository.js 增加 adapter 实现（list/apply/health）
 *   3. 无需改任何调用方——调用方只依赖槽位契约，不依赖具体引擎
 */

const SLOT_CONTRACTS = [
  {
    id: 'ai-chat',
    name: 'AI 对话引擎',
    category: 'ai',
    adapter: 'llm-gateway',
    hotSwap: true,
    description: '大模型对话能力槽位。AI 对话、专家联盟咨询、无穷维度优化、图谱咨询等全部 LLM 调用统一经此槽位路由，切换引擎零代码改动。',
    contract: {
      methods: [
        {
          name: 'chat',
          input: {
            messages: 'Array<{role: "system"|"user"|"assistant", content: string}>（消息数组）',
            temperature: 'number?（0-2，默认取引擎配置）',
            max_tokens: 'integer?（默认取引擎配置）'
          },
          output: {
            content: 'string（回复正文）',
            usage: '{ total_tokens?: number }（用量统计）'
          }
        },
        {
          name: 'testConnection',
          input: '{ engineId: string }',
          output: '{ ok: boolean, latency_ms: number }（契约探活）'
        }
      ],
      switchExample: 'POST /engine-kernel/switch {"slot":"ai-chat","engineId":"deepseek"}',
      notes: '候选引擎动态来自 llm-gateway providers（openai/claude/doubao/qwen/kimi/deepseek/zhipu/gemini/local 及用户自装引擎）。'
    }
  },
  {
    id: 'storage',
    name: '持久化引擎',
    category: 'infrastructure',
    adapter: 'storage-config',
    hotSwap: true,
    description: '数据持久化槽位。全部业务域的 JSON/SQLite 双写存储经此槽位路由，可在 SQLite/MySQL/PostgreSQL 间瞬间切换。',
    contract: {
      methods: [
        {
          name: 'readJSON / writeJSON',
          input: '{ file: string, data?: any }（逻辑文件名 + 数据）',
          output: 'any | boolean（读返回数据，写返回成功标记；底层自动双写 SQLite）'
        }
      ],
      switchExample: 'POST /engine-kernel/switch {"slot":"storage","engineId":"sqlite"}',
      notes: '候选引擎动态来自 config.js storage.providers（sqlite/mysql/postgresql）。'
    }
  },
  {
    id: 'web-search',
    name: '联网搜索引擎',
    category: 'ai',
    adapter: 'web-search',
    hotSwap: true,
    description: '联网搜索槽位。AI 对话联网模式、知识时效性增强经此槽位路由，可在 Bing/DuckDuckGo/Tavily/博查/SearXNG 间瞬间切换。',
    contract: {
      methods: [
        {
          name: 'search',
          input: '{ query: string }（自然语言查询）',
          output: '{ results: Array<{title, url, snippet}> }'
        }
      ],
      switchExample: 'POST /engine-kernel/switch {"slot":"web-search","engineId":"bing"}',
      notes: '候选引擎动态来自 web-search-service SEARCH_ENGINES。'
    }
  },
  {
    id: 'pitch-detection',
    name: '音高检测引擎',
    category: 'audio',
    adapter: 'melody2score',
    hotSwap: true,
    description: '旋律转谱的核心音高检测槽位。Node 网关按当前绑定把 backend 参数注入 Python 转发请求，可在 crepe_onnx/pyin/torchcrepe/auto（自动降级）间瞬间切换。',
    contract: {
      methods: [
        {
          name: 'recognize',
          input: '{ file: Binary, backend: string（由槽位绑定自动注入）, robust?: boolean }',
          output: '{ jianpu, bpm, key, notes, confidence, backend }（结构化歌谱）'
        }
      ],
      switchExample: 'POST /engine-kernel/switch {"slot":"pitch-detection","engineId":"crepe_onnx"}',
      notes: '引擎在 Python FastAPI 子项目（melody2score/enterprise_api.py）内实现；绑定持久化于 engine_bindings.json，代理层注入 backend 表单字段。auto = 服务端自动降级（crepe_onnx→pyin）。'
    }
  }
];

/** 槽位索引 */
const SLOT_INDEX = Object.fromEntries(SLOT_CONTRACTS.map(s => [s.id, s]));

/** 三层商城层次定义 */
const MARKETPLACE_LAYERS = [
  { id: 'system', name: '系统商城', description: '系统内置引擎（随版本发布，开箱即用，全部自研）' },
  { id: 'cloud', name: '云端商城', description: '云端插件目录（可指向任意注册表 URL，安装即注册到本系统）' },
  { id: 'local', name: '本地商城', description: '本地安装的插件（JSON 清单声明，落盘 engine_plugins.json）' }
];

module.exports = { SLOT_CONTRACTS, SLOT_INDEX, MARKETPLACE_LAYERS };
