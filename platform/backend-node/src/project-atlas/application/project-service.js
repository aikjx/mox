'use strict';

/**
 * 项目治理用例（application 层 · "一切皆是项目"承载）
 * ------------------------------------------------------------------
 * 项目实体的运行时治理：创建 / 域归属调整 / 生命周期流转 / 健康度量。
 * 代码基线项目（project-registry.js）不可变；运行时项目进 auto 层 projects 键。
 *
 * 每次变更后：持久化 → 图谱重建 → W10 复验（变更不得引入破窗）。
 *
 * 依赖注入（可测性）：registryIO / rebuild / getView / verify
 * 由 index.js 装配时注入；测试可注入 stub。
 */

const {
  canTransition, validateProject, auditDomainOwnership, projectHealth
} = require('../domain/project-registry');

function createProjectService({ registryIO, rebuild, getView, verify }) {

  /** 校验上下文：合并视图的合法域/项目 id 集合 */
  function validationContext(allowOverwrite) {
    const view = getView();
    return {
      domainIds: new Set(view.domains.map(d => d.id)),
      projectIds: new Set(view.projects.map(p => p.id)),
      allowOverwrite
    };
  }

  /** 项目清单：每项目健康度量 + 生命周期（含合并视图统计） */
  function listProjects() {
    const view = getView();
    const verifyState = verify();
    const projects = view.projects.map(p => {
      const health = projectHealth(p, view, verifyState);
      return {
        id: p.id, name: p.name, vision: p.vision || null,
        status: p.status,
        runtime: p.runtime === true,
        registeredAt: p.registeredAt || null,
        ...health
      };
    });
    return {
      projects,
      stats: {
        total: projects.length,
        runtimeRegistered: projects.filter(p => p.runtime).length,
        byStatus: Object.fromEntries(
          [...new Set(projects.map(p => p.status))].map(s => [s, projects.filter(p => p.status === s).length])
        ),
        avgScore: projects.length ? Math.round(projects.reduce((s, p) => s + p.score, 0) / projects.length) : 0,
        totalDomains: projects.reduce((s, p) => s + p.domainCount, 0)
      }
    };
  }

  /** 项目全景：归属域逐个展开（功能/引擎/数据/文档）+ 流程 + 健康度量 */
  function getProjectDetail(projectId) {
    const view = getView();
    const project = view.projects.find(p => p.id === projectId);
    if (!project) return null;

    const domainById = new Map(view.domains.map(d => [d.id, d]));
    const domains = (project.domains || []).map(id => {
      const d = domainById.get(id);
      if (!d) return { id, missing: true };
      return {
        id: d.id, name: d.name, codePath: d.codePath,
        keyFeatures: d.keyFeatures || [],
        engines: d.engines || [],
        dataAssets: d.dataAssets || [],
        docs: d.docs || [],
        auto: d.auto === true
      };
    });
    const flows = view.flows
      .filter(f => (project.domains || []).includes(f.domain))
      .map(f => ({ id: f.id, name: f.name, stepCount: f.steps.length, standard: f.standard || null }));

    const verifyState = verify();
    return {
      id: project.id, name: project.name, vision: project.vision || null,
      status: project.status,
      runtime: project.runtime === true,
      registeredAt: project.registeredAt || null,
      domains, flows,
      health: projectHealth(project, view, verifyState)
    };
  }

  /** 创建项目：P1-P6 校验 → 持久化 → 重建 → W10 复验 */
  function createProject(project, options = {}) {
    const { valid, errors } = validateProject(project, validationContext(options.overwrite === true));
    if (!valid) {
      return { accepted: false, errors, reason: '项目建模不变式校验未通过（P1-P6）' };
    }

    // P2 前置：新项目不得抢夺已归属域（唯一归属）
    const view = getView();
    const owner = new Map();
    for (const p of view.projects) for (const d of (p.domains || [])) owner.set(d, p.id);
    const conflicts = project.domains.filter(d => owner.has(d) && !isOwnedBySelf(view, project.id, d));
    if (conflicts.length > 0) {
      return {
        accepted: false,
        reason: 'P2 域归属冲突（域须恰好归属一个项目）',
        errors: conflicts.map(d => ({ rule: 'P2', message: `域 ${d} 已归属项目 ${owner.get(d)}` }))
      };
    }

    const auto = registryIO.read();
    const projects = (auto.projects || []).filter(p => p.id !== project.id);
    projects.push({
      ...project,
      registeredAt: new Date().toISOString(),
      registeredBy: 'project-service',
      runtime: true
    });
    registryIO.write({ ...auto, projects });
    rebuild();

    const verification = verify();
    return {
      accepted: true,
      project: project.id,
      domainCount: project.domains.length,
      verification: { ok: verification.ok, total: verification.summary.total, failed: verification.summary.failed }
    };
  }

  /** 生命周期流转：状态机合法边校验（不可逆；代码基线项目不可变更） */
  function transitionProject(projectId, toStatus) {
    const view = getView();
    const project = view.projects.find(p => p.id === projectId);
    if (!project) return { accepted: false, reason: `项目不存在: ${projectId}` };

    // 基线保护优先于状态机校验（报错语义准确指名根因）
    const target = findProjectSlot(registryIO, projectId);
    if (!target) return { accepted: false, reason: `项目属于代码基线（状态不可变更）: ${projectId}` };

    if (project.status === toStatus) {
      return { accepted: false, reason: `项目已处于目标状态: ${toStatus}` };
    }
    if (!canTransition(project.status, toStatus)) {
      return {
        accepted: false,
        reason: `生命周期流转非法（${project.status} → ${toStatus} 不可达，状态机不可逆）`
      };
    }

    target.entry.status = toStatus;
    target.entry.statusChangedAt = new Date().toISOString();
    registryIO.write(target.auto);
    rebuild();

    const verification = verify();
    return {
      accepted: true,
      project: projectId,
      from: project.status, to: toStatus,
      verification: { ok: verification.ok, total: verification.summary.total, failed: verification.summary.failed }
    };
  }

  /** 域归属调整：把域从当前项目移交给目标项目（保持 P2 唯一归属） */
  function assignDomain(projectId, domainId) {
    const view = getView();
    const target = view.projects.find(p => p.id === projectId);
    if (!target) return { accepted: false, reason: `项目不存在: ${projectId}` };
    if (!view.domains.some(d => d.id === domainId)) {
      return { accepted: false, reason: `域不存在于图谱: ${domainId}` };
    }
    if ((target.domains || []).includes(domainId)) {
      return { accepted: false, reason: `域已归属目标项目: ${domainId}` };
    }

    // 前置校验全部通过后再做变更（避免失败路径污染内存单一真相源）
    const auto = registryIO.read();
    const oldRuntime = (auto.projects || []).find(p => (p.domains || []).includes(domainId));
    const oldBaseline = view.projects.find(p =>
      p.runtime !== true && (p.domains || []).includes(domainId));

    const targetSlot = (auto.projects || []).find(p => p.id === projectId);
    if (!targetSlot) {
      return { accepted: false, reason: `目标项目为代码基线（不可运行时挂域）: ${projectId}` };
    }
    if (oldRuntime) {
      if (oldRuntime.domains.length - 1 < 2) {
        return { accepted: false, reason: `移交后源项目 ${oldRuntime.id} 域数 <2（违反 P6 内聚），请先合并项目` };
      }
    } else if (oldBaseline) {
      return {
        accepted: false,
        reason: `域 ${domainId} 归属代码基线项目 ${oldBaseline.id}（基线归属不可运行时调整，须改代码注册表）`
      };
    }

    // 执行移交：源项目移除 + 目标项目挂载
    if (oldRuntime) oldRuntime.domains = oldRuntime.domains.filter(d => d !== domainId);
    targetSlot.domains = [...(targetSlot.domains || []), domainId];
    registryIO.write(auto);
    rebuild();

    const verification = verify();
    return {
      accepted: true,
      project: projectId, domain: domainId,
      verification: { ok: verification.ok, total: verification.summary.total, failed: verification.summary.failed }
    };
  }

  /** 移除运行时项目（基线不可移除；移除不得造成孤儿域）
   *  options.reassignTo：级联移交承接项目 id（项目解散/合并场景：域整体移交后删除） */
  function removeProject(projectId, options = {}) {
    const auto = registryIO.read();
    const projects = auto.projects || [];
    const target = projects.find(p => p.id === projectId);
    if (!target) {
      return { removed: false, reason: `项目不存在或属于代码基线（不可移除）: ${projectId}` };
    }

    // 级联移交：域整体移交承接项目后删除（原子操作，一次落盘）
    if (options.reassignTo) {
      const receiver = projects.find(p => p.id === options.reassignTo);
      if (!receiver) {
        return { removed: false, reason: `承接项目不存在或属于代码基线: ${options.reassignTo}` };
      }
      const merged = [...new Set([...(receiver.domains || []), ...(target.domains || [])])];
      if (merged.length !== (receiver.domains || []).length + (target.domains || []).length) {
        return { removed: false, reason: '级联移交存在重复域归属（P2），请检查两项目域清单' };
      }
      receiver.domains = merged;
      registryIO.write({ ...auto, projects: projects.filter(p => p.id !== projectId) });
      rebuild();
      const verification = verify();
      return {
        removed: true, project: projectId, reassignedTo: options.reassignTo,
        movedDomains: target.domains.length,
        verification: { ok: verification.ok, total: verification.summary.total, failed: verification.summary.failed }
      };
    }

    // 常规移除：孤儿域防护（被移除项目的域须先移交；atlas-auto 容器域豁免，与 W10 口径一致）
    const view = getView();
    const remaining = view.projects.filter(p => p.id !== projectId);
    const domainIds = new Set(view.domains.map(d => d.id).filter(id => id !== 'atlas-auto'));
    const audit = auditDomainOwnership(remaining, domainIds);
    if (audit.orphans.length > 0) {
      return {
        removed: false,
        reason: `移除将造成孤儿域（先移交或指定 reassignTo 承接）: ${audit.orphans.join(',')}`
      };
    }

    registryIO.write({ ...auto, projects: projects.filter(p => p.id !== projectId) });
    rebuild();
    const verification = verify();
    return {
      removed: true, project: projectId,
      verification: { ok: verification.ok, total: verification.summary.total, failed: verification.summary.failed }
    };
  }

  /** 预检：不落盘校验 */
  function precheckProject(project, options = {}) {
    return validateProject(project, validationContext(options.overwrite === true));
  }

  return {
    listProjects, getProjectDetail, createProject,
    transitionProject, assignDomain, removeProject, precheckProject
  };
}

/** 在 auto 层定位运行时项目槽位（返回 {auto, entry} 或 null） */
function findProjectSlot(registryIO, projectId) {
  const auto = registryIO.read();
  const entry = (auto.projects || []).find(p => p.id === projectId);
  return entry ? { auto, entry } : null;
}

/** 判断域当前是否归属指定项目（覆盖语义下的自归属豁免） */
function isOwnedBySelf(view, projectId, domainId) {
  const p = view.projects.find(x => x.id === projectId);
  return !!p && (p.domains || []).includes(domainId);
}

module.exports = { createProjectService };
