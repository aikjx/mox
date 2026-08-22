'use strict';

/**
 * 路由域：项目全息图谱（Project Atlas）
 * 整个项目机器图谱化：24 业务域 + 4 模块 + 18 引擎 + 15 算法 + 34 数据资产 + 34 文档，
 * 全部关联本地代码路径；AI 对话经专家联盟架构师专家图谱增强回答。
 */
module.exports = function registerAtlasRoutes(ctx) {
  const { ok, fail, readBody, log, reg } = ctx;
  const atlas = require('../project-atlas');
  const { getAlliance } = require('../expert-alliance');

  // 完整全息图谱（129 节点 + 173 边 + 统计）
  reg('get', '/atlas', (req, res) => {
    ok(res, atlas.getAtlas());
  });

  // 无破窗验证（145 项：动态比对路由域/数据目录/代码路径/文档/连通性）
  reg('get', '/atlas/verify', (req, res) => {
    ok(res, atlas.verifyAtlas());
  });

  // 单域全景：功能/引擎/算法/数据/文档一屏尽览
  reg('get', '/atlas/domains/:id', (req, res, params) => {
    const detail = atlas.getDomainDetail(params.id);
    if (!detail) return fail(res, 404, `业务域不存在: ${params.id}`);
    ok(res, detail);
  });

  // 影响面分析：改动一个节点波及哪些引擎/算法/数据/文档
  reg('get', '/atlas/impact/:id', (req, res, params) => {
    const result = atlas.impact(params.id);
    if (!result) return fail(res, 404, `图谱节点不存在: ${params.id}`);
    ok(res, result);
  });

  // 图谱资产检索（自然语言关键词）
  reg('get', '/atlas/search', (req, res) => {
    const q = require('url').parse(req.url, true).query;
    if (!q.q) return fail(res, 400, 'q 为必填');
    ok(res, atlas.searchAtlas(q.q));
  });

  // ===== 业务处理流程图谱化（理清系统：全流程清单/单流程全景） =====

  // 全系统流程清单：步骤数/降级链/关联域/标准锚点
  reg('get', '/atlas/flows', (req, res) => {
    ok(res, atlas.getFlows());
  });

  // 单流程全景：步骤链 + 每步委托引擎/数据读写 + 降级链
  reg('get', '/atlas/flows/:id', (req, res, params) => {
    const detail = atlas.getFlowDetail(params.id);
    if (!detail) return fail(res, 404, `业务流程不存在: ${params.id}`);
    ok(res, detail);
  });

  // ===== 通用流程注册（EAF-STD-001 接入：其他模块向通用 AI 知识图谱注册业务流程） =====

  // 流程预检：不落盘校验（接入方自助检查是否符合建模不变式）
  reg('post', '/atlas/flows/precheck', async (req, res) => {
    const body = await readBody(req).catch(() => ({}));
    ok(res, atlas.precheckFlow(body.flow || body, body));
  });

  // 注册业务流程：V1-V8 校验 → 持久化 → 图谱重建 → W9 复验（不合规 400 拒绝并逐条指名）
  reg('post', '/atlas/flows', async (req, res) => {
    const body = await readBody(req).catch(() => ({}));
    const flow = body.flow || body;
    try {
      const result = atlas.registerFlow(flow, { overwrite: body.overwrite === true });
      if (!result.accepted) {
        return fail(res, 400, result.reason, { errors: result.errors });
      }
      ok(res, result);
    } catch (e) {
      fail(res, 500, `流程注册失败: ${e.message}`);
    }
  });

  // 移除运行时注册的流程（代码基线流程不可移除）
  reg('delete', '/atlas/flows/:id', (req, res, params) => {
    try {
      const result = atlas.removeFlow(params.id);
      if (!result.removed) return fail(res, 404, result.reason);
      ok(res, result);
    } catch (e) {
      fail(res, 500, `流程移除失败: ${e.message}`);
    }
  });

  // ===== 项目治理（"一切皆是项目"：项目实体/生命周期/健康度量/域归属） =====

  // 项目清单：每项目健康度量 + 生命周期状态分布
  reg('get', '/atlas/projects', (req, res) => {
    ok(res, atlas.getProjects());
  });

  // 生命周期状态机自描述（合法状态 + 合法流转边；须注册于 :id 之前避免被参数路由拦截）
  reg('get', '/atlas/projects-lifecycle', (req, res) => {
    ok(res, atlas.LIFECYCLE);
  });

  // 项目全景：归属域逐个展开（功能/引擎/数据/文档）+ 流程 + 健康度量
  reg('get', '/atlas/projects/:id', (req, res, params) => {
    const detail = atlas.getProjectDetail(params.id);
    if (!detail) return fail(res, 404, `项目不存在: ${params.id}`);
    ok(res, detail);
  });

  // 项目预检：不落盘校验（P1-P6 建模不变式）
  reg('post', '/atlas/projects/precheck', async (req, res) => {
    const body = await readBody(req).catch(() => ({}));
    ok(res, atlas.precheckProject(body.project || body, body));
  });

  // 创建项目：P1-P6 校验 → 持久化 → 图谱重建 → W10 复验
  reg('post', '/atlas/projects', async (req, res) => {
    const body = await readBody(req).catch(() => ({}));
    const project = body.project || body;
    try {
      const result = atlas.createProject(project, { overwrite: body.overwrite === true });
      if (!result.accepted) {
        return fail(res, 400, result.reason, { errors: result.errors });
      }
      ok(res, result);
    } catch (e) {
      fail(res, 500, `项目创建失败: ${e.message}`);
    }
  });

  // 生命周期流转：状态机合法边校验（不可逆）
  reg('post', '/atlas/projects/:id/transition', async (req, res, params) => {
    const body = await readBody(req).catch(() => ({}));
    if (!body.to) return fail(res, 400, 'to 为必填（目标生命周期状态）');
    try {
      const result = atlas.transitionProject(params.id, body.to);
      if (!result.accepted) return fail(res, 400, result.reason);
      ok(res, result);
    } catch (e) {
      fail(res, 500, `生命周期流转失败: ${e.message}`);
    }
  });

  // 域归属移交：把域从当前项目移交给目标项目（保持 P2 唯一归属）
  reg('post', '/atlas/projects/:id/domains', async (req, res, params) => {
    const body = await readBody(req).catch(() => ({}));
    if (!body.domain) return fail(res, 400, 'domain 为必填（待移交域 id）');
    try {
      const result = atlas.assignDomain(params.id, body.domain);
      if (!result.accepted) return fail(res, 400, result.reason);
      ok(res, result);
    } catch (e) {
      fail(res, 500, `域归属移交失败: ${e.message}`);
    }
  });

  // 移除运行时项目（基线不可移除；孤儿域防护；?reassignTo= 承接项目级联移交）
  reg('delete', '/atlas/projects/:id', (req, res, params) => {
    try {
      const q = Object.fromEntries(new URL(req.url, 'http://x').searchParams.entries());
      const result = atlas.removeProject(params.id, { reassignTo: q.reassignTo });
      if (!result.removed) return fail(res, 404, result.reason);
      ok(res, result);
    } catch (e) {
      fail(res, 500, `项目移除失败: ${e.message}`);
    }
  });

  // AI 图谱对话：架构师专家 + 全息图谱上下文增强回答
  reg('post', '/atlas/consult', async (req, res) => {
    const body = await readBody(req);
    if (!body.question) return fail(res, 400, 'question 为必填');
    try {
      const alliance = getAlliance();
      const result = await alliance.consultAtlas(body.question, {
        temperature: body.temperature,
        problemContext: body.context
      });
      ok(res, result);
    } catch (e) {
      fail(res, 500, `图谱咨询失败: ${e.message}`);
    }
  });

  // ===== 图谱自管理（自己管理自己）=====

  // 自管理预览：发现待登记/待清理资产（不落盘）
  reg('get', '/atlas/self-sync', (req, res) => {
    ok(res, atlas.discoverPending());
  });

  // 执行自管理同步：扫描→自动登记→图谱重建（幂等；POST 默认执行，body.dryRun=true 仅预览）
  reg('post', '/atlas/self-sync', async (req, res) => {
    const body = await readBody(req).catch(() => ({}));
    try {
      ok(res, atlas.selfSync({ dryRun: body.dryRun === true }));
    } catch (e) {
      fail(res, 500, `自管理同步失败: ${e.message}`);
    }
  });

  // 自愈验证：验证失败时自动同步修复后复验
  reg('post', '/atlas/self-heal', (req, res) => {
    try {
      ok(res, atlas.selfHealVerify());
    } catch (e) {
      fail(res, 500, `自愈失败: ${e.message}`);
    }
  });

  // 启动即自管理：路由装配完成（全部业务域就绪）后自动同步一次（幂等，无人值守）
  try {
    const bootSync = atlas.selfSync({ dryRun: false });
    if (bootSync.changed) {
      log(`Atlas self-sync (boot): registered domains=[${bootSync.discovered.domains.map(d => d.id).join(',')}] data=[${bootSync.discovered.dataFiles.join(',')}] docs=[${bootSync.discovered.docs.join(',')}]`);
    } else {
      log('Atlas self-sync (boot): clean, nothing to register');
    }
  } catch (e) {
    log(`Atlas self-sync (boot) failed: ${e.message}`);
  }

  log('Project atlas endpoints registered: graph, verify, domain detail, impact, search, AI consult, self-sync, self-heal');
};
