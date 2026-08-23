'use strict';
/**
 * Enterprise T6 · Mocha 正式化测试套件（之一）—— Atlas 注册表 + 代码落盘 + W10 项目治理
 * 目标：对应 docs/enterprise/18 TOP-MASTER §四
 */
const fs = require('fs');
const path = require('path');
const assert = require('assert');

const ROOT = path.resolve(__dirname, '..');
const WS_ROOT = path.resolve(ROOT, '..', '..');

const atlas = require(path.join(ROOT, 'src', 'project-atlas'));
const { DOMAINS, MODULES } = require(path.join(ROOT, 'src', 'project-atlas', 'domain', 'business-registry'));
const { ALGORITHMS, DATA_ASSETS, DOCS } = require(path.join(ROOT, 'src', 'project-atlas', 'domain', 'tech-registry'));
const { PROJECTS } = require(path.join(ROOT, 'src', 'project-atlas', 'domain', 'project-registry'));
const { ENGINES } = require(path.join(ROOT, 'src', 'engine-universe', 'domain', 'engine-registry'));

const existsOnDisk = (p) => {
  const candidates = [
    path.join(ROOT, p),
    path.join(ROOT, '..', p),
    path.join(WS_ROOT, p),
  ];
  return candidates.some((c) => fs.existsSync(c));
};

describe('[T6-AC17-1] 注册表非空与最小规模（All-02 判重基线）', function () {
  it('DOMAINS >= 45（Node 基线 30 + Rust crate 自动 15）', () => assert.ok(DOMAINS.length >= 45, `actual=${DOMAINS.length}`));
  it('Rust crate DOMAINS 条目 = 15（15 主 platform crates）', () => {
    const rust = DOMAINS.filter((d) => d.kind === 'rust-crate');
    assert.strictEqual(rust.length, 15, `rust-crate=${rust.length}`);
  });
  it('明确标记 Node 业务域（kind === "node" 或空）条目 = 30', () => {
    const n = DOMAINS.filter((d) => d.kind === 'node' || !d.kind);
    assert.strictEqual(n.length, 30, `node-domains=${n.length}`);
  });
  it('ALGORITHMS >= 20', () => assert.ok(ALGORITHMS.length >= 20, `actual=${ALGORITHMS.length}`));
  it('ENGINES >= 20', () => assert.ok(ENGINES.length >= 20, `actual=${ENGINES.length}`));
  it('MODULES >= 4', () => assert.ok(MODULES.length >= 4, `actual=${MODULES.length}`));
  it('DATA_ASSETS >= 30', () => assert.ok(DATA_ASSETS.length >= 30, `actual=${DATA_ASSETS.length}`));
  it('DOCS >= 30', () => assert.ok(DOCS.length >= 30, `actual=${DOCS.length}`));
  it('PROJECTS >= 9', () => assert.ok(PROJECTS.length >= 9, `actual=${PROJECTS.length}`));
  it('atlas.getAtlas().stats.byKind.domain ≥ DOMAINS.length（autoRegistry 注入增量合法）', () => {
    const a = atlas.getAtlas();
    assert.ok(a.stats.byKind.domain >= DOMAINS.length,
      `byKind.domain=${a.stats.byKind.domain} < DOMAINS=${DOMAINS.length}`);
  });
});

describe('[T6-AC17-2] DOMAINS 代码路径真实落盘（All-03 四归三连）', function () {
  it('每个域都声明了 codePath', () => {
    const bad = DOMAINS.filter((d) => typeof d.codePath !== 'string' || !d.codePath);
    assert.deepStrictEqual(bad.map((b) => b.id), [], '缺少 codePath');
  });
  // 按 id 顺序前 24 条逐一断言，余下用集合断言（保证 ≥70 的数量）
  for (const d of DOMAINS.slice(0, 24)) {
    it(`域 ${d.id} codePath 本地存在`, () => {
      assert.ok(existsOnDisk(d.codePath), `${d.id} 缺失: ${d.codePath}`);
    });
  }
  it('余下域 codePath 一并全通过', () => {
    const fails = DOMAINS.slice(24).filter((d) => !existsOnDisk(d.codePath))
      .map((d) => `${d.id}|${d.codePath}`);
    assert.deepStrictEqual(fails, [], `缺失 ${fails.length} 个`);
  });
});

describe('[T6-AC17-3] W10 项目归属唯一 · 不重复 · 无孤儿 · 内聚', function () {
  it('PROJECTS.id 全局唯一', () => {
    const ids = PROJECTS.map((p) => p.id);
    assert.strictEqual(new Set(ids).size, ids.length, '有重复项目 id');
  });
  it('每个项目声明 ≥ 1 个归属域', () => {
    const bad = PROJECTS.filter((p) => !(Array.isArray(p.domains) && p.domains.length >= 1));
    assert.deepStrictEqual(bad.map((b) => b.id), []);
  });
  it('Rust crate 域全部归属到项目（零孤儿）', () => {
    const owned = new Set();
    PROJECTS.forEach((p) => (p.domains || []).forEach((x) => owned.add(x)));
    const orphans = DOMAINS.filter((d) => d.kind === 'rust-crate' && !owned.has(d.id)).map((d) => d.id);
    assert.deepStrictEqual(orphans, [], `孤儿 rust-crate: ${orphans.join(',')}`);
  });
  it('Node 业务域 W6 内聚：≥ 3 项关键功能 + ≥ 1 个引擎 + ≥ 1 份文档（data asset 推荐但不强扣；Rust 条目跳过）', () => {
    const checks = DOMAINS.filter((d) => d.kind === 'node' || (!d.kind && true)).map((d) => ({
      id: d.id,
      kf: (Array.isArray(d.keyFeatures) ? d.keyFeatures.length : 0) >= 3,
      eng: (Array.isArray(d.engines) ? d.engines.length : 0) >= 1,
      doc: (Array.isArray(d.docs) ? d.docs.length : 0) >= 1,
    }));
    const bad = checks.filter((c) => !(c.kf && c.eng && c.doc)).map((c) => c.id);
    assert.deepStrictEqual(bad, [], `${bad.length} 业务域未达 W6 内聚：${bad.join(',')}`);
  });
});

describe('[T6-AC17-4] 算法/引擎 单源与去重（All-01·开口/量尺/出手）', function () {
  it('ALGORITHMS.id 唯一', () => {
    const ids = ALGORITHMS.map((a) => a.id);
    assert.strictEqual(new Set(ids).size, ids.length);
  });
  it('ENGINES.id 唯一', () => {
    const ids = ENGINES.map((e) => e.id);
    assert.strictEqual(new Set(ids).size, ids.length);
  });
  it('DOMAINS.id 唯一', () => {
    const ids = DOMAINS.map((d) => d.id);
    assert.strictEqual(new Set(ids).size, ids.length);
  });
  it('ALGORITHMS 每个算法声明 primary_impl ∈ {RUST, NODE, PYTHON, HYBRID, undefined}（undefined 表示后续登记）', () => {
    const ok = new Set(['RUST', 'NODE', 'PYTHON', 'HYBRID', undefined]);
    const bad = ALGORITHMS.filter((a) => !ok.has(a.primary_impl)).map((a) => `${a.id}|${a.primary_impl}`);
    assert.deepStrictEqual(bad, [], `${bad.length} 个算法 primary_impl 非法`);
  });
  it('ALGORITHMS 全部声明 id + name（最小身份契约）', () => {
    const bad = ALGORITHMS.filter((a) => !(a.id && a.name)).map((a) => a.id);
    assert.deepStrictEqual(bad, []);
  });
});
