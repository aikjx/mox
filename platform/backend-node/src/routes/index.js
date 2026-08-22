'use strict';

/**
 * 路由域装配清单（配置前置）
 *
 * 新增业务域三步：
 *   1. 本目录新建 <domain>.js，导出 register<Domain>Routes(ctx)，内部 const {...} = ctx 解构依赖
 *   2. 在下方 DOMAINS 登记一行（顺序即注册顺序，保持与既有路由优先级一致）
 *   3. 重启服务，路由自动生效
 *
 * ctx 注入清单见 api-server.js 的 registerRoutes()。
 */
const DOMAINS = [
  ['system', '系统与状态', require('./system')],
  ['graph', '知识图谱', require('./graph')],
  ['chat', 'AI 对话', require('./chat')],
  ['web-search', '联网搜索', require('./web-search')],
  ['artifacts', '本地制品', require('./artifacts')],
  ['optimizer', '无穷维度优化', require('./optimizer')],
  ['ai-platform', 'AI 平台资源', require('./ai-platform')],
  ['browser-market', '浏览器与市场', require('./browser-market')],
  ['integration', '集成通道', require('./integration')],
  ['expert-alliance', '专家联盟', require('./expert-alliance')],
  ['expert-graph', '专家图谱', require('./expert-graph')],
  ['orchestration', '编排协作', require('./orchestration')],
  ['ai-enhanced', '16 模块 AI 增强', require('./ai-enhanced')],
  ['tasks', '任务管理', require('./tasks')],
  ['kb', '知识库', require('./kb')],
  ['auto-tasks', '自动任务', require('./auto-tasks')],
  ['modules-admin', '模块与存储管理', require('./modules-admin')],
  ['security', '安全审计', require('./security')],
  ['ai-engine', 'AI 引擎核心', require('./ai-engine')],
  ['ai-integrated', '智能集成引擎', require('./ai-integrated')],
  ['ai-ultimate', '终极 AI 引擎', require('./ai-ultimate')],
  ['auto-dev', '自动开发引擎', require('./auto-dev')],
  ['services', '服务管理', require('./services')],
];

function registerAllRoutes(ctx) {
  for (const [file, name, register] of DOMAINS) {
    try {
      register(ctx);
    } catch (e) {
      console.error(`[routes] 域 ${name}(${file}) 注册失败:`, e);
      throw e;
    }
  }
  console.log(`[routes] ${DOMAINS.length} 个业务域装配完成`);
}

module.exports = { DOMAINS, registerAllRoutes };
