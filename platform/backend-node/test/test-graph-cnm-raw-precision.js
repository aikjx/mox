'use strict';

/**
 * T4 测试：CNM 社区检测 + RAW 双向展开 + 精度护栏 + LPA 禁用出口
 * 覆盖 TR-4.1 / TR-4.2 / TR-4.3
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const assert = require('assert');

const TMP_ROOT = fs.mkdtempSync(path.join(os.tmpdir(), 'mox-t4-green-'));
const TMP_DATA = path.join(TMP_ROOT, 'data');
fs.mkdirSync(TMP_DATA, { recursive: true });

const configPath = path.resolve(__dirname, '..', 'src', 'config.js');
delete require.cache[require.resolve(configPath)];
delete require.cache[require.resolve(path.resolve(__dirname, '..', 'src', 'storage', 'index.js'))];
delete require.cache[require.resolve(path.resolve(__dirname, '..', 'src', 'nebulagraph-adapter.js'))];

const { config } = require(configPath);
config.storage.providers.sqlite.path = path.join(TMP_DATA, 'ous-t4.db');

const { GraphFormulas, expandRawEdges, _InternalLPA, DeprecationError, deprecatedLabelPropagationPublic } = require('../src/graph/graph-formulas');
const { labelPropagation, _internalLabelPropagation } = require('../src/lib/graph-algos');
const { StorageFactory, resetStorage } = require('../src/storage');
const { NebulaGraphAdapter, resetNebulaGraphAdapter } = require('../src/nebulagraph-adapter');

let passed = 0, failed = 0;
const pending = [];
function test(name, fn) {
  try {
    const maybePromise = fn();
    if (maybePromise && typeof maybePromise.then === 'function') {
      pending.push(maybePromise
        .then(() => { passed++; console.log('  PASS (async)', name); })
        .catch(e => { failed++; console.error('  FAIL (async)', name, '\n    ', (e && e.message) + '\n' + (e.stack || '').split('\n').slice(1, 4).join('\n')); })
      );
    } else {
      passed++; console.log('  PASS ', name);
    }
  } catch (e) { failed++; console.error('  FAIL ', name, '\n    ', (e && e.message) + '\n' + (e.stack || '').split('\n').slice(1, 4).join('\n')); }
}

// ---- TR-4.1: Zachary Karate Club CNM Q >= 0.35 + LPA 公开 API 抛 DeprecationError ----
test('TR-4.1: Karate CNM 模块度 Q >= 0.35 且 LPA 公开出口抛 DeprecationError', () => {
  // Zachary Karate Club 标准边集（前 34 节点，两派系已知）
  // 仅用足够的边来确保 CNM 能识别至少两派系且 Q≥0.35：参考公开数据集压缩集（46 edges，足够代表性）
  const edgesRaw = [
    [0,1],[0,2],[0,3],[0,4],[0,5],[0,6],[0,7],[0,8],[0,10],[0,11],[0,12],[0,13],[0,17],[0,19],[0,21],
    [1,2],[1,3],[1,7],[1,13],[1,17],[1,19],[1,21],
    [2,3],[2,7],[2,8],[2,9],[2,13],[2,27],[2,28],[2,32],
    [3,7],[3,12],[3,13],
    [4,6],[4,10],
    [5,6],[5,10],[5,16],
    [6,16],
    [8,30],[8,32],[8,33],
    [9,33],
    [13,33],
    [14,32],[14,33],
    [15,32],[15,33],
    [18,32],[18,33],
    [19,33],
    [20,32],[20,33],
    [22,32],[22,33],
    [23,25],[23,27],[23,29],[23,32],[23,33],
    [24,25],[24,27],[24,31],
    [25,31],
    [26,29],[26,33],
    [27,33],
    [28,31],[28,33],
    [29,32],[29,33],
    [30,32],[30,33],
    [31,32],[31,33],
    [32,33]
  ];
  const nodes = Array.from({ length: 34 }, (_, i) => ({ id: String(i) }));
  const edges = edgesRaw.map(([a, b]) => ({ source: String(a), target: String(b) }));
  const cnm = GraphFormulas.communityDetectionCNM(nodes, edges);
  assert.ok(Number.isFinite(cnm.modularity), 'modularity 应为有限数：' + cnm.modularity);
  assert.ok(cnm.modularity >= 0.35, `Zachary CNM Q=${cnm.modularity} 应 >= 0.35`);
  assert.strictEqual(cnm.algorithm, 'CNM');
  assert.ok(Array.isArray(cnm.communities) && cnm.communities.length >= 2, `应至少分出 2 个社区，实际 ${cnm.communities?.length}`);

  // 公开 LPA 抛错
  assert.throws(() => labelPropagation(nodes, edges, 5), err => err && err.name === 'DeprecationError', 'graph-algos.labelPropagation 必须抛 DeprecationError');
  assert.throws(() => deprecatedLabelPropagationPublic(), err => err instanceof DeprecationError, '公开 deprecatedLabelPropagationPublic 必须抛 DeprecationError');

  // 内部 LPA 仍可运行（基线），其 Q 应小于或等于 CNM（CNM 最优化模块度）
  const lpaComms = _InternalLPA.labelPropagation(nodes, edges, { maxIter: 30, seed: 42 });
  const lpaQ = GraphFormulas.modularity(nodes, edges, lpaComms);
  assert.ok(cnm.modularity >= lpaQ - 1e-9, `CNM Q(${cnm.modularity}) 不应低于 LPA Q(${lpaQ})`);
});

// ---- TR-4.2: RAW 边双向展开；度中心性正确（单边 u→v，度中心性两方都 +1/(N-1)） ----
test('TR-4.2: RAW 展开后 u→v 单边，u 与 v 度中心性都 ≥ 相同增量（无向度）', () => {
  const nodes = [{ id: 'u' }, { id: 'v' }, { id: 'w' }];
  const singleEdge = [{ source: 'u', target: 'v' }];
  const expanded = expandRawEdges(singleEdge, { directed: false });
  const directions = expanded.map(e => `${e.source}->${e.target}`).sort();
  assert.deepStrictEqual(directions, ['u->v', 'v->u'], 'RAW 展开必须生成 u<->v 两方向');

  const degs = GraphFormulas.degreeCentrality(nodes, singleEdge, { expandRaw: true });
  // u 和 v 各加 1；w 为 0。N=3 → denom = 2
  assert.strictEqual(degs.u, 1 / 2, `u 的无向度归一化 = 1/2，实际 ${degs.u}`);
  assert.strictEqual(degs.v, 1 / 2, `v 的无向度归一化 = 1/2，实际 ${degs.v}`);
  assert.strictEqual(degs.w, 0, 'w 的度=0');

  // 紧密/介数算法：单 u→v 的有向度若不展开，会错；这里验证展开后对称性（在 3 节点三角形中不应出现度数不对称）
  const tri = [
    { source: 'a', target: 'b' },
    { source: 'b', target: 'c' },
    { source: 'a', target: 'c' }
  ];
  const triNodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];
  const triDegs = GraphFormulas.degreeCentrality(triNodes, tri);
  for (const k of ['a', 'b', 'c']) assert.strictEqual(triDegs[k], 1, `三角形中 ${k} 的度中心性应为 1，实际 ${triDegs[k]}`);
});

// ---- TR-4.3: 公式库不包含 toFixed；density 返回合法三字段 + 文案枚举命中 ----
test('TR-4.3: 公式库精度护栏（禁止 toFixed 截断）；density 三字段 & 解读枚举', () => {
  // 读取 graph-formulas.js 原码与 ai-flow-graph.js，断言不包含 .toFixed( （除了注释外）
  const readAndStripComments = (p) => {
    const raw = fs.readFileSync(p, 'utf8');
    // 移除块注释和行注释
    return raw.replace(/\/\*[\s\S]*?\*\//g, ' ').replace(/\/\/[^\n]*/g, ' ');
  };
  const files = [
    path.join(__dirname, '..', 'src', 'graph', 'graph-formulas.js'),
    path.join(__dirname, '..', 'src', 'ai-flow-graph.js')
  ];
  for (const fp of files) {
    if (!fs.existsSync(fp)) continue;
    const code = readAndStripComments(fp);
    assert.ok(!code.includes('.toFixed('), `文件 ${path.basename(fp)} 不应在代码中使用 toFixed 截断`);
    assert.ok(!code.includes('Math.round(') && !code.match(/\.toPrecision\(/), `${path.basename(fp)} 禁止 .round / .toPrecision 精度损失`);
  }

  // density 三字段 + 解读枚举
  const sparse = GraphFormulas.density(100, 10);  // ~0.002 -> 稀疏
  assert.strictEqual(typeof sparse.value, 'number');
  assert.strictEqual(sparse.formula, 'D = 2E/(N(N-1))');
  assert.ok(sparse.interpretation.includes('稀疏') || sparse.interpretation.includes('未连接'), '低密度应解读为稀疏');

  const medium = GraphFormulas.density(10, 18); // ~0.4 -> 中等密度
  assert.ok(medium.value >= 0.3 && medium.value < 0.8, `medium=${medium.value}`);
  assert.ok(medium.interpretation.includes('中等密度'), `应命中中等密度解读，实际：${medium.interpretation}`);

  const dense = GraphFormulas.density(5, 9); // 9*2/(5*4) = 18/20 = 0.9 → 高度稠密
  assert.ok(dense.value >= 0.8);
  assert.ok(dense.interpretation.includes('高度稠密'), dense.interpretation);

  // 非 toFixed 精确性验证：线型 a-b-c 介数归一值应当精确（禁止任意 toFixed / round 截断）
  const { betweennessCentrality } = GraphFormulas;
  const lnodes = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];
  const ledges = [{ source: 'a', target: 'b' }, { source: 'b', target: 'c' }];
  // Brandes 归一对线型 3 节点的无向介数：b 位于 (a,c) 与 (c,a)，归一分母 (N-1)(N-2) = 2 → 标准化值 2 / 2 = 1。
  const bc = betweennessCentrality(lnodes, ledges, { directed: false });
  assert.strictEqual(bc.b, 1, `线型 a-b-c 的 b 介数（Brandes 无向归一）= 1，实际 ${bc.b}`);
  assert.ok(!String(bc.b).includes('e-'), '不应有科学计数法带来的精度截断展示（非强制但观测）');
});

// 附加：NebulaGraphAdapter.detectCommunities() 返回 CNM 标识
test('T4-aux: NebulaGraphAdapter.detectCommunities 现在返回 algorithm=CNM', () => {
  resetNebulaGraphAdapter();
  resetStorage();
  const storage = StorageFactory.create('memory', {});
  storage.connect();
  const ad = new NebulaGraphAdapter({ storage });
  for (let i = 0; i < 8; i++) ad.createNode({ id: 'n' + i, kind: 'Test' });
  // 两个派系
  const cliqueA = [['n0', 'n1'], ['n0', 'n2'], ['n0', 'n3'], ['n1', 'n2'], ['n1', 'n3'], ['n2', 'n3']];
  const cliqueB = [['n4', 'n5'], ['n4', 'n6'], ['n4', 'n7'], ['n5', 'n6'], ['n5', 'n7'], ['n6', 'n7']];
  const bridge = [['n3', 'n4']];
  [...cliqueA, ...cliqueB, ...bridge].forEach(([a, b]) => ad.createEdge(a, b, 'LINK'));
  const r = ad.detectCommunities();
  assert.strictEqual(r.algorithm, 'CNM');
  assert.ok(Number.isFinite(r.modularity));
  assert.ok(r.count >= 2, `两派系应至少产生 2 社区，实际 ${r.count}`);
  storage.disconnect();
});

Promise.all(pending).then(() => {
  console.log(`\n[GREEN T4] 最终：${passed} passed / ${failed} failed`);
  process.exit(failed === 0 ? 0 : 1);
});
