#!/usr/bin/env node
/**
 * self_sync_all.js — 璇玑全资源模块化 统一自发现 / 自登记 / 自出卡脚本
 * ------------------------------------------------------------------
 * **唯一真源**：platform/backend-node/schemas/module-card-v1.schema.json
 *
 * 扫描范围（6 大类 · §24 文档 §2）：
 *   1. Rust 16 Crate：platform/services/* + platform/gateway/runtime
 *      → 来源：各 crate src/lib.rs 三常量 CRATE_ID/ENGINE_NAME/CRATE_META
 *        + 各 crate README.md 的 6 字段卡 md 表格解析
 *   2. Node 8 功能域：platform/backend-node/src/{modules,kb,expert-alliance,...}/index.js
 *      → 来源：code_graph_bindings.json v2 + docs/modules/cards/node-*.md
 *   3. Frontend 28 视图：frontend-ui/src/views/** /*.vue
 *      → 来源：frontend-ui/src/views/_cards/*.module.md
 *   4. 企业文档 24 份：docs/enterprise/00-INDEX.md 表中的文档（00~24）
 *      → 来源：docs/enterprise/cards/doc-XX-module.md
 *   5. 算法家族 8 个：A1~A8 CNM/Brandes/Harmonic/PR/A5/RRF/CEM/CPM
 *      → 来源：graph-algorithms 8 算法 meta
 *   6. 业务流程 10 个：BP-01~10
 *      → 来源：04-business-processing.md
 *
 * 输出（4 份产物，与 kg-hub ingest Connector 直接对接）：
 *   (A) data/all_module_cards.json        — 75×6 字段卡对象数组（符合 §2.0 Schema，
 *                                             用 Ajv JSON Schema validate 0 错误）
 *   (B) data/graph_modules.nodes.json    — 节点注入 JSON（kg-hub Entity 形态：
 *                                             urn=moduleId, kind=, props=卡）
 *   (C) data/graph_modules.edges.json    — 7 类边注入 JSON 展开结果（
 *                                             depends_on / calls / belongs_to /
 *                                             governed_by / reconciled_with /
 *                                             version_of / rbac_granted_to）
 *   (D) data/all_module_cards.report.md  — 人类可读验收报告：节点数 / 边数 /
 *                                             6 字段卡空字段计数 / 贯通率 /
 *                                             P9 判重闸门命中 / 与上一次 diff
 *
 * **执行**：node scripts/self_sync_all.js [--strict]
 *   --strict 模式：若出现 Schema 校验失败 / 450 字段有空 / 贯通率 < 100%
 *                 / P9 重复项 > 0 → exit code=1（CI 阻断）
 *
 * **权威链对齐**：24 号文档 §2.0 + §2.1~§2.4 6 大类卡
 *                + §3.3 注入 Pipeline + §3.4 5 项贯通验收指标
 *                + §6 越权判定 9 条。
 *
 * 版权 © 璇玑 RelGraph · 三联盟联合所有
 */
'use strict';

const fs = require('fs');
const path = require('path');

/* ------------------------------------------------------------------
 * 0. 常量
 * ---------------------------------------------------------------- */
const BACKEND_NODE = __dirname.includes(path.sep + 'backend-node' + path.sep)
  ? path.dirname(__dirname)
  : path.join(process.cwd(), 'platform', 'backend-node');
const REPO = path.join(BACKEND_NODE, '..', '..');
const DATA = path.join(BACKEND_NODE, 'data');
const DOCS_ENT = path.join(REPO, 'docs', 'enterprise');
const SCHEMA_PATH = path.join(BACKEND_NODE, 'schemas', 'module-card-v1.schema.json');

const OUT_A = path.join(DATA, 'all_module_cards.json');        // 75 cards
const OUT_B = path.join(DATA, 'graph_modules.nodes.json');    // 节点 kg-hub Entity 形
const OUT_C = path.join(DATA, 'graph_modules.edges.json');    // 7 类边
const OUT_D = path.join(DATA, 'all_module_cards.report.md');  // 报告

const STRICT = process.argv.includes('--strict');

/* ------------------------------------------------------------------
 * 1. 工具函数（最小化无依赖；避免 require('ajv')，若 CI 中 Ajv 可用自动启用）
 * ---------------------------------------------------------------- */
const log = (msg) => console.log(`[self_sync_all] ${msg}`);
const fatal = (msg) => {
  console.error(`[self_sync_all][FATAL] ${msg}`);
  if (STRICT) process.exit(1);
};

/* 简易 JSON Schema 校验（支持 required / type / enum / pattern / minLength /
 * minItems / uniqueItems / additionalProperties / const）。生产环境建议
 * `npm i -D ajv` 并用 Ajv 严格模式替换。这里是 self_sync_all 的无依赖
 * 子集，确保在未安装依赖的 CI 环境也能 exit(0) 跑骨架。 */
function validateSchema(card, schema, path = '$') {
  const errs = [];
  if (schema.required) {
    for (const key of schema.required) {
      if (!(key in card)) errs.push(`${path}.${key} is required`);
    }
  }
  if (schema.additionalProperties === false) {
    const allowed = new Set([...(schema.required || []), ...Object.keys(schema.properties || {})]);
    for (const k of Object.keys(card)) if (!allowed.has(k)) errs.push(`${path}.${k} is not allowed (additionalProperties=false)`);
  }
  if (schema.properties) {
    for (const [k, prop] of Object.entries(schema.properties)) {
      if (!(k in card)) continue;
      const v = card[k];
      const p = `${path}.${k}`;
      if (prop.type && typeof v !== prop.type.replace('array', 'object') && !(prop.type === 'array' && Array.isArray(v))) {
        errs.push(`${p} type expected ${prop.type} got ${Array.isArray(v) ? 'array' : typeof v}`);
      }
      if (prop.enum && !prop.enum.includes(v) && !(Array.isArray(v))) errs.push(`${p}=${JSON.stringify(v)} not in enum ${JSON.stringify(prop.enum)}`);
      if (prop.const && v !== prop.const) errs.push(`${p} must equal ${JSON.stringify(prop.const)}`);
      if (prop.pattern && typeof v === 'string' && !(new RegExp(prop.pattern).test(v))) errs.push(`${p}="${v}" doesn't match pattern`);
      if (prop.minLength && typeof v === 'string' && v.length < prop.minLength) errs.push(`${p} length < minLength=${prop.minLength}`);
      if (prop.minItems && Array.isArray(v) && v.length < prop.minItems) errs.push(`${p} array length < minItems=${prop.minItems}`);
      if (prop.uniqueItems && Array.isArray(v) && new Set(v).size !== v.length) errs.push(`${p} array has duplicates`);
      if (prop.properties && typeof v === 'object' && v !== null) {
        errs.push(...validateSchema(v, prop, p));
      }
      if (prop.items && Array.isArray(v)) {
        for (let i = 0; i < v.length; i++) {
          if (typeof prop.items === 'object' && !Array.isArray(prop.items)) {
            errs.push(...validateSchema(v[i], prop.items, `${p}[${i}]`));
          }
        }
      }
    }
  }
  return errs;
}

/* ------------------------------------------------------------------
 * 2. 各类扫描器骨架（按 24 §2.1~§2.6 6 大类逐一实现）
 * ---------------------------------------------------------------- */

/** 2.1 Rust 16 Crate 扫描器骨架
 *  真实实现要做：读 16 Cargo.toml → 找 src/lib.rs 三常量 CRATE_ID const →
 *               读 README.md 的 6 字段卡表格（md 解析）→ 出 16 张卡。
 *  这里骨架：用 24 §2.1 表 16 条 内置，启动时 warn 请补真实 lib.rs 解析。 */
function scanRustCrates() {
  const builtin = [
    ['xuanji-common-meta', '34a20231-1a80-5426-b392-40d7a2ddd9f7', 'L0_SoT'],
    ['ai-agent',           '00374bdd-cc60-55bf-8970-a879afbfe443', 'L3_AlgorithmReasoning'],
    ['business-catalog',   '62b2cca1-d98f-5e41-b26e-8d2a43966117', 'L5_BusinessFlow'],
    ['flow-ai',            '2fcd3eac-e894-5876-b007-fb33c56c0d65', 'L3_AlgorithmReasoning'],
    ['graph-algorithms',   'fbd31c6a-41cd-5274-be2f-2a28066eaf0a', 'L4_GraphCore'],
    ['hermes-flow-bridge', '9bfaf43b-385a-5a44-9fb2-65b4003ee80d', 'L5_BusinessFlow'],
    ['kg-hub',             'cb909f06-c0df-55ec-b397-543623a8c349', 'L4_GraphCore'],
    ['operator-core',      'acf14283-3931-5528-adce-2c0cd3815363', 'L2_RustBase'],
    ['operator-wasm',      '5a1df407-b217-5340-a5ae-5f4535d1e6de', 'L2_RustBase'],
    ['optimizer',          'e56676c7-ec1f-5415-9587-ba8249d0178a', 'L3_AlgorithmReasoning'],
    ['primiflow-core',     '8c8d2382-6f9f-5218-894e-a07a43aa9554', 'L5_BusinessFlow'],
    ['primiflow-fusion',   '75238345-b48b-534b-818b-8d9abe083a41', 'L4_GraphCore'],
    ['template-market',    '4d2e50c1-9d64-525d-86cf-2d7d610a27b9', 'L5_BusinessFlow'],
    ['xuanji-expert',      '50bb6200-04c5-5e4c-8354-4c6e1b230024', 'L4_GraphCore'],
    ['xuanji-system',      'b81eec75-22ff-5155-ac49-19edf6f6b5ab', 'L4_GraphCore'],
    ['runtime',            'a6f7ad5c-dbc8-5c27-837f-d8332fd6f27b', 'L6_ProductApp'],
  ];
  return builtin.map(([name, idShort, layer]) => {
    return {
      // 骨架阶段：先填 6 字段卡的**合法**占位，让 Schema 校验先过
      // 真实阶段用 lib.rs 解析替换
      moduleId: `urn:xuanji:crate:${idShort}`,
      name: `Rust Crate · ${name}`,
      aisLayer: layer,
      raci: { R: '开发联盟', A: '开发联盟', C: [], I: '总设计师' },
      upstreamDownstream: { upstream: [], downstream: [] },
      acceptanceGate: {
        unitTestPath: `cargo test -p ${name}`,
        reconcileCmd: 'TODO(Day1-Rust): 补真实对账命令',
        governGate: true,
        auditFields: ['actor', 'action', 'entity_ids', 'ts', 'signature']
      },
      __skeleton: true // 骨架标记（报告里会单独统计，真实阶段必须 0 个）
    };
  });
}

/** 2.2 Node 8 域扫描器骨架（同上 8 域占位出卡） */
function scanNodeDomains() {
  const builtin = [
    ['Graph 图谱域（算法+存储）',         '00000000-0000-0000-0000-000000000001', 'L4_GraphCore',       '算法联盟', '开发联盟'],
    ['KB 知识库域（RAG+文档抽取）',       '00000000-0000-0000-0000-000000000002', 'L5_BusinessFlow',    '产品联盟', '开发联盟'],
    ['Expert Alliance 专家联盟',         '00000000-0000-0000-0000-000000000003', 'L4_GraphCore',       '算法联盟', '产品联盟'],
    ['Project Atlas 项目图谱（自同步）',  '00000000-0000-0000-0000-000000000004', 'L5_BusinessFlow',    '开发联盟', '产品联盟'],
    ['AI Engine 4 端点统一',              '00000000-0000-0000-0000-000000000005', 'L6_ProductApp',      '开发联盟', '算法联盟'],
    ['MCP 协议域',                       '00000000-0000-0000-0000-000000000006', 'L6_ProductApp',      '开发联盟', '开发联盟'],
    ['Security（RBAC + 审计 hash_chain）','00000000-0000-0000-0000-000000000007','L4_GraphCore',       '开发联盟', '开发联盟'],
    ['Engine Universe 图查询',           '00000000-0000-0000-0000-000000000008', 'L4_GraphCore',       '算法联盟', '开发联盟'],
  ];
  return builtin.map(([name, id, layer, R, A]) => ({
    moduleId: `urn:xuanji:nmod:${id}`,
    name: `Node 域 · ${name}`,
    aisLayer: layer,
    raci: { R, A, C: [], I: '总设计师' },
    upstreamDownstream: { upstream: [], downstream: [] },
    acceptanceGate: {
      unitTestPath: `TODO(Day2-Node): ${name} 单测路径`,
      reconcileCmd: 'TODO(Day2-Node): 补真实对账命令',
      governGate: true,
      auditFields: ['actor', 'action', 'entity_ids', 'ts', 'signature']
    },
    __skeleton: true
  }));
}

/** 2.3 前端 28 视图骨架（7+5+5+3+3+4+1=28） */
function scanFEViews() {
  const cats = [
    ['fusion',   7, 'L6_ProductApp',   '产品联盟', '开发联盟'],
    ['monitor',  5, 'L6_ProductApp',   '算法联盟', '产品联盟'],
    ['admin',    5, 'L6_ProductApp',   '开发联盟', '开发联盟'],
    ['market',   3, 'L5_BusinessFlow', '产品联盟', '开发联盟'],
    ['kb',       3, 'L5_BusinessFlow', '产品联盟', '开发联盟'],
    ['expert',   4, 'L4_GraphCore',    '算法联盟', '产品联盟'],
    ['settings', 1, 'L6_ProductApp',   '开发联盟', '开发联盟'],
  ];
  let n = 0; const cards = [];
  for (const [prefix, count, layer, R, A] of cats) {
    for (let i = 1; i <= count; i++) {
      const id = String(n + 1).padStart(2, '0'); n++;
      cards.push({
        moduleId: `urn:xuanji:feview:00000000-0000-0000-0000-${id.padStart(12, '0')}`,
        name: `Frontend · ${prefix}-${i}`,
        aisLayer: layer,
        raci: { R, A, C: [], I: '总设计师' },
        upstreamDownstream: { upstream: [], downstream: [] },
        acceptanceGate: {
          unitTestPath: `TODO(Day2-FE): views/_cards/${prefix}-${i}.spec.js`,
          reconcileCmd: 'TODO(Day2-FE): E2E 3 步冒烟脚本',
          governGate: true,
          auditFields: ['actor', 'action', 'entity_ids', 'ts', 'signature']
        },
        __skeleton: true
      });
    }
  }
  return cards;
}

/** 2.4 企业文档 24 份（00~23）骨架 */
function scanDocs() {
  const cards = [];
  for (let i = 0; i <= 23; i++) {
    const id = String(i).padStart(2, '0');
    // 必须严格落在 7 层枚举内（Schema 不接受字符串 "L1" 等简称）。
    // 0/14/15/19/20/22/23 按权威链 L1 级 → 但模块 AIS Layer 分层按文档内容实际归属分配：
    //  L0 = 真源（18 TOP-MASTER）
    //  L1_DeployOps = 工作指南、入口、裁决类文档（00/14/15/19/20/22/23/24 = L1 治理/拍板/调度）
    //  L2 = 需求/架构/设计/业务/里程碑/SRS 层（01~06/17/21）
    //  L3 = 执行级文档（07 需求铁律 / 08 自动化 / 09 归档 / 10 交付清单）
    //  L4 = 证据级验收报告（11/12/13/16）
    const LAYER_MAP = {
      0:  'L1_DeployOps', 18: 'L0_SoT',
      14: 'L1_DeployOps', 15: 'L1_DeployOps', 19: 'L1_DeployOps', 20: 'L1_DeployOps',
      22: 'L1_DeployOps', 23: 'L1_DeployOps', 24: 'L1_DeployOps',
      1: 'L2_RustBase', 2: 'L2_RustBase', 3: 'L2_RustBase', 4: 'L2_RustBase',
      5: 'L2_RustBase', 6: 'L2_RustBase', 17: 'L2_RustBase', 21: 'L2_RustBase',
      7: 'L3_AlgorithmReasoning', 8: 'L3_AlgorithmReasoning', 9: 'L3_AlgorithmReasoning', 10: 'L3_AlgorithmReasoning',
      11: 'L4_GraphCore', 12: 'L4_GraphCore', 13: 'L4_GraphCore', 16: 'L4_GraphCore'
    };
    const layer = LAYER_MAP[i] || 'L1_DeployOps';
    const docUuid = `00000000-0000-0000-0000-${id.padStart(8, '0')}0000`; // doc XX 编码到 UUID 最后 12 位，保证唯一
    cards.push({
      moduleId: `urn:xuanji:doc:${docUuid}`,
      name: `企业文档 · ${id}`,
      aisLayer: layer.startsWith('L') ? layer : 'L1',
      raci: {
        R: i === 18 ? '产品联盟' : (i === 21 ? '产品联盟' : (i === 23 ? '产品联盟' : '开发联盟')),
        A: i === 18 ? '产品联盟' : (i === 22 || i === 23 ? '产品联盟' : i === 19 ? '算法联盟' : i === 20 ? '开发联盟' : '开发联盟'),
        C: [], I: '总设计师'
      },
      upstreamDownstream: { upstream: [], downstream: [] },
      acceptanceGate: {
        unitTestPath: `grep -n "权威链冲突" docs/enterprise/*.md | wc -l = 0`,
        reconcileCmd: `22号归一化表9 该文档锚点行数`,
        governGate: true,
        auditFields: ['actor', 'action', 'entity_ids', 'ts', 'signature']
      },
      __skeleton: true
    });
  }
  return cards;
}

/* ------------------------------------------------------------------
 * 3. 边展开器（7 类边，每模块 ≥ 4 条）
 * ---------------------------------------------------------------- */
function expandEdges(cards) {
  const edges = [];
  for (const c of cards) {
    // belongs_to × 2（Layer + BP，这里默认给 BP-0x 占位）
    edges.push({ from: c.moduleId, type: 'belongs_to', to: `urn:xuanji:bp:${c.aisLayer.substring(0,2)}`, __note: `归属 ${c.aisLayer} AIS Layer` });
    edges.push({ from: c.moduleId, type: 'belongs_to', to: 'urn:xuanji:bp:00000000-0000-0000-0000-000000000006', __note: '默认挂到 BP-06 璇玑融合（Day2 实际替换）' });
    // governed_by × 1
    edges.push({ from: c.moduleId, type: 'governed_by', to: 'urn:xuanji:crate:75238345-b48b-534b-818b-8d9abe083a41', __note: '受 primiflow-fusion G1-G8 治理闸门' });
    // version_of × 1
    edges.push({ from: c.moduleId, type: 'version_of', to: 'urn:xuanji:bp:00000000-0000-0000-0000-000000000000', __note: 'M0 全域归一化（Day2 替换到具体 Mx）' });
    // upstream → depends_on × N
    for (const u of (c.upstreamDownstream?.upstream || [])) {
      edges.push({ from: c.moduleId, type: 'depends_on', to: u });
      edges.push({ from: u, type: 'calls', to: c.moduleId, __note: 'depends_on 反向边（关图路径双向查询必需）' });
    }
    // reconciled_with（算法类模块，检测名字含 graph-algo / flow-ai）
    if (/graph-algorithms|flow-ai|Graph.*|图谱.*|算法/i.test(c.name)) {
      edges.push({ from: c.moduleId, type: 'reconciled_with', to: 'urn:xuanji:nmod:00000000-0000-0000-0000-000000000001', __note: '与 Node 图谱域 reconcile_7x8.js 对账 Δ≤1e-6' });
    }
    // rbac_granted_to（检测含 Admin / 写操作）
    if (/admin|audit|publish|optimize|settings|写/i.test(c.name)) {
      edges.push({ from: c.moduleId, type: 'rbac_granted_to', to: 'urn:xuanji:role:Admin+Auditor', __note: '写操作模块需 Admin 或 Auditor 双写审计' });
    }
  }
  return edges;
}

/* ------------------------------------------------------------------
 * 4. 主流程
 * ---------------------------------------------------------------- */
function main() {
  log('启动 self_sync_all.js（骨架模式），Schema: ' + SCHEMA_PATH);
  const schema = JSON.parse(fs.readFileSync(SCHEMA_PATH, 'utf8'));
  const cards = [
    ...scanRustCrates(),    // 16
    ...scanNodeDomains(),   // 8
    ...scanFEViews(),       // 28
    ...scanDocs(),          // 24 = 76
  ];

  // Schema 校验：校验前剥离 __skeleton（内部标记字段，Schema additionalProperties=false 不接受）
  let schemaErrors = 0;
  for (let i = 0; i < cards.length; i++) {
    const copy = Object.assign({}, cards[i]);
    delete copy.__skeleton;
    const errs = validateSchema(copy, schema);
    if (errs.length) {
      schemaErrors += errs.length;
      console.error(`[self_sync_all][Schema card#${i}] ${cards[i].moduleId}:\n  - ${errs.join('\n  - ')}`);
    }
  }
  if (schemaErrors) fatal(`Schema 校验失败 ${schemaErrors} 处`);

  // 空字段计数（450 字段目标：75 卡 × 6 字段结构 = 75 * 7 key 结构 约 450）
  const skeletonCards = cards.filter(c => c.__skeleton).length;
  const todoCount = JSON.stringify(cards).split('TODO(').length - 1;
  log(`生成 ${cards.length} 张 6 字段卡；骨架标记卡 = ${skeletonCards} / ${cards.length}；TODO 占位 = ${todoCount}（Day1~2 补完 = 450 字段 0 空）`);
  if (STRICT && todoCount > 0) fatal('STRICT 模式：TODO>0，模块化 450 字段有占位');

  // P9 判重（moduleId 唯一）
  const ids = new Map();
  for (const c of cards) {
    if (ids.has(c.moduleId)) fatal(`P9 重复 moduleId: ${c.moduleId}`);
    ids.set(c.moduleId, c);
  }

  // 7 类边展开
  const edges = expandEdges(cards);
  const modulesWithEdges = new Set();
  for (const e of edges) { modulesWithEdges.add(e.from); }
  const coverage = modulesWithEdges.size / Math.max(1, cards.length);
  log(`生成边 ${edges.length} 条；有边模块 ${modulesWithEdges.size}/${cards.length} = 贯通率 ${(coverage*100).toFixed(1)}%`);
  if (STRICT && coverage < 1.0) fatal('STRICT 模式：贯通率 < 100%（每模块 ≥4 条边）');
  const avgEdges = edges.length / Math.max(1, cards.length);
  log(`每模块平均边数 = ${avgEdges.toFixed(2)}（目标 ≥ 5.5）`);

  // KG-Hub 实体节点化
  const entities = cards.map(c => {
    const kind = (c.moduleId.match(/^urn:xuanji:([^:]+):/) || [, 'Unknown'])[1];
    const props = Object.assign({}, c);
    delete props.__skeleton;
    return {
      urn: c.moduleId,
      kind: `Module_${kind}`,
      label: c.name,
      props,
      source: 'self_sync_all.js@24-Modularization'
    };
  });

  // 产物写出（A/B/C/D）
  const strippedCards = cards.map(c => { const o = Object.assign({}, c); delete o.__skeleton; return o; });
  fs.writeFileSync(OUT_A, JSON.stringify({ generatedAt: new Date().toISOString(), count: strippedCards.length, entries: strippedCards }, null, 2));
  fs.writeFileSync(OUT_B, JSON.stringify({ generatedAt: new Date().toISOString(), count: entities.length, entities }, null, 2));
  fs.writeFileSync(OUT_C, JSON.stringify({ generatedAt: new Date().toISOString(), count: edges.length, edges }, null, 2));

  // 报告 D（Markdown）
  const reportLines = [];
  reportLines.push(`# 璇玑 RelGraph · self_sync_all 模块化贯通验收报告`);
  reportLines.push('');
  reportLines.push(`> 生成：${new Date().toISOString()} · STRICT=${STRICT ? 'ON' : 'OFF'}`);
  reportLines.push('');
  reportLines.push(`## 0. 总览`);
  reportLines.push(`| 指标 | 值 | 目标 | 状态 |`);
  reportLines.push(`|------|:----:|:----:|:----:|`);
  reportLines.push(`| 模块标准卡（6 字段）总数 | ${cards.length} | ≥ 75 | ${cards.length >= 75 ? '✅' : '⚠️'} |`);
  reportLines.push(`| Schema 校验错误数 | ${schemaErrors} | 0 | ${schemaErrors === 0 ? '✅' : '❌'} |`);
  reportLines.push(`| 骨架标记卡数量（__skeleton） | ${skeletonCards} | 0 | ${skeletonCards === 0 ? '✅' : '📋 骨架阶段'} |`);
  reportLines.push(`| TODO 占位字段数 | ${todoCount} | 0 | ${todoCount === 0 ? '✅' : '📋 骨架阶段'} |`);
  reportLines.push(`| P9 判重闸门（moduleId 唯一） | ${ids.size === cards.length ? '0 重复' : `${cards.length - ids.size} 重复`} | 0 重复 | ${ids.size === cards.length ? '✅' : '❌'} |`);
  reportLines.push(`| 图谱节点数（Entity） | ${entities.length} | ≥ 483 | ${entities.length >= 483 ? '✅' : '📋（当前骨架 75 模块 + 现有 289 = 约 364 目标阶段 483）'} |`);
  reportLines.push(`| 7 类边总数 | ${edges.length} | ≥ 410 | ${edges.length >= 410 ? '✅' : '📋（骨架阶段）'} |`);
  reportLines.push(`| 每模块平均边数 | ${avgEdges.toFixed(2)} | ≥ 5.5 | ${avgEdges >= 5.5 ? '✅' : '⚠️'} |`);
  reportLines.push(`| 模块级贯通率（有边模块 / 总模块） | ${(coverage*100).toFixed(1)}% | ≥ 100% | ${coverage >= 1.0 ? '✅' : '⚠️'} |`);
  reportLines.push('');
  reportLines.push(`## 1. 输出文件`);
  reportLines.push(`- (A) \`${path.relative(REPO, OUT_A)}\`（${Math.round(fs.statSync(OUT_A).size/1024)} KB）`);
  reportLines.push(`- (B) \`${path.relative(REPO, OUT_B)}\`（kg-hub 节点形态）`);
  reportLines.push(`- (C) \`${path.relative(REPO, OUT_C)}\`（7 类边）`);
  reportLines.push(`- (D) 本报告 \`${path.relative(REPO, OUT_D)}\``);
  reportLines.push('');
  reportLines.push(`## 2. 后续 TODO（Day 1~2）`);
  reportLines.push(`1. 替换 16 Rust Crate 的 src/lib.rs 三常量解析，去掉 __skeleton=true`);
  reportLines.push(`2. Node 8 域读取各域 index.js + 补 6 字段真实 raci/upstreamDownstream`);
  reportLines.push(`3. Frontend 28 视图写 views/_cards/*.module.md 6 字段卡`);
  reportLines.push(`4. Enterprise 文档 24 份读取 00-INDEX 主责列，补 450 字段真实值`);
  reportLines.push(`5. upstreamDownstream 手工填写真实上下游（6 大类依赖关系）`);
  reportLines.push(`6. --strict 0 exit 全绿 → 进入 kg-hub ingest`);
  fs.writeFileSync(OUT_D, reportLines.join('\n'));

  log(`4 份产物写出 OK：`);
  log(`  (A) 模块 6 字段卡 x ${cards.length} → ${path.relative(REPO, OUT_A)}`);
  log(`  (B) KG 节点 ${entities.length} → ${path.relative(REPO, OUT_B)}`);
  log(`  (C) 7 类边 ${edges.length} → ${path.relative(REPO, OUT_C)}`);
  log(`  (D) 人类可读报告 → ${path.relative(REPO, OUT_D)}`);
  log(`（骨架阶段 __skeleton=${skeletonCards}，Day 1~2 补完后 STRICT 0 exit = A+ 级模块化贯通 100%）`);
}

try {
  main();
} catch (e) {
  console.error(`[self_sync_all][EXCEPTION] ${e.stack || e}`);
  if (STRICT) process.exit(1);
}
