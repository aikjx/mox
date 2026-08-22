'use strict';

/**
 * 通用流程注册用例（application 层 · EAF-STD-001 接入服务）
 * ------------------------------------------------------------------
 * 任何模块按 EAF-STD-001 标准向通用 AI 知识图谱注册业务流程：
 *   校验（V1-V8 不变式）→ 持久化（auto 层 flows）→ 图谱重建 → W9 复验。
 *
 * 与 self-sync 同层持久化（atlas_auto_registry.json 的 flows 键），
 * 代码注册表（flow-registry.js）为不可变基线，运行时注册进 auto 覆盖层。
 *
 * 依赖注入（可测性）：registryIO / rebuild / getView / verify
 * 由 index.js 装配时注入；测试可注入 stub。
 */

const { validateFlow } = require('../domain/flow-validator');

function createFlowRegistrationService({ registryIO, rebuild, getView, verify }) {

  /** 注册上下文：合并视图的合法域/引擎/数据/流程 id 集合 */
  function validationContext(allowOverwrite) {
    const view = getView();
    return {
      domainIds: new Set(view.domains.map(d => d.id)),
      engineIds: new Set(view.engineIds),
      dataFiles: new Set(view.dataAssets.map(x => x.file)),
      flowIds: new Set(view.flows.map(f => f.id)),
      allowOverwrite
    };
  }

  /**
   * 注册业务流程（EAF-STD-001 §6 接入契约）
   * @param {object} flow {id,name,domain,standard?,steps[],transitions[]}
   * @param {object} options {overwrite?:boolean}——同 id 覆盖（默认拒绝）
   * @returns {accepted:boolean, flow?, errors?, verification?}
   */
  function registerFlow(flow, options = {}) {
    const { valid, errors } = validateFlow(flow, validationContext(options.overwrite === true));
    if (!valid) {
      return { accepted: false, errors, reason: 'EAF-STD-001 建模不变式校验未通过（V1-V8）' };
    }

    const auto = registryIO.read();
    const flows = (auto.flows || []).filter(f => f.id !== flow.id); // 覆盖语义：移除旧条目
    flows.push({
      ...flow,
      registeredAt: new Date().toISOString(),
      registeredBy: 'flow-registration-service',
      runtime: true // 运行时注册标记（区别于代码基线）
    });
    registryIO.write({ ...auto, flows });
    rebuild();

    // 注册后立即 W9 复验（注册不得引入破窗）
    const verification = verify();
    return {
      accepted: true,
      flow: flow.id,
      stepCount: flow.steps.length,
      degradeCount: flow.transitions.filter(t => t.type === 'degrade').length,
      graphNodes: getView().flows.reduce((s, f) => s + f.steps.length, 0),
      verification: { ok: verification.ok, total: verification.summary.total, failed: verification.summary.failed }
    };
  }

  /** 移除运行时注册的流程（代码基线流程不可移除） */
  function removeFlow(flowId) {
    const auto = registryIO.read();
    const flows = auto.flows || [];
    const target = flows.find(f => f.id === flowId);
    if (!target) {
      return { removed: false, reason: `流程不存在或属于代码基线（不可移除）: ${flowId}` };
    }
    registryIO.write({ ...auto, flows: flows.filter(f => f.id !== flowId) });
    rebuild();
    const verification = verify();
    return {
      removed: true, flow: flowId,
      verification: { ok: verification.ok, total: verification.summary.total, failed: verification.summary.failed }
    };
  }

  /** 预检：不落盘校验（接入方自助检查） */
  function precheckFlow(flow, options = {}) {
    return validateFlow(flow, validationContext(options.overwrite === true));
  }

  return { registerFlow, removeFlow, precheckFlow };
}

module.exports = { createFlowRegistrationService };
