'use strict';
/**
 * T1: Rust 16 crate 入璇玑三注册表 + self_sync 脚本
 * TR 1.1 business-registry.js rust 条目 = 16
 * TR 1.2 engine-registry.js rust 条目 = 16
 * TR 1.3 tech-registry 7 条算法 singleSource=true + main=rust
 * TR 1.4 运行 self_sync_rust.js 后 JSON entries ≥ 16 crate 且 ≥ 100 文件
 */
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const BACKEND = path.join(__dirname, '..');

describe('T1 · Rust 16 crates 三注册表 + self_sync', function () {
  this.timeout(60000);

  it('TR 1.1 business-registry.js: rust::<package> 条目 = 16', function () {
    const { DOMAINS } = require('../src/project-atlas/domain/business-registry.js');
    const rust = DOMAINS.filter(d => d.id && d.id.startsWith('rust::'));
    assert.strictEqual(rust.length, 16,
      `期望 16 条 rust:: 条目，实际 ${rust.length}。Ids: ${rust.map(r => r.id).join(', ')}`);
    for (const r of rust) {
      assert.strictEqual(r.kind, 'rust', `${r.id} .kind 必须为 rust`);
      assert.ok(r.codePath && r.codePath.length > 0, `${r.id} .codePath 缺失`);
      assert.ok(r.crateId && r.crateId.length === 36, `${r.id} .crateId 缺失或非 UUIDv5`);
      assert.ok(Array.isArray(r.owns_domain), `${r.id} .owns_domain 非数组`);
      assert.strictEqual(r.version, '3.0.0-ai-powered', `${r.id} .version 不为 3.0.0-ai-powered`);
      assert.ok(Array.isArray(r.tags), `${r.id} .tags 非数组`);
      assert.ok(r.tags.includes('rust'), `${r.id} .tags 缺失 'rust'`);
      assert.ok(r.tags.some(t => t.startsWith('ais::')), `${r.id} .tags 缺失 ais::<layer>`);
    }
    const ids = rust.map(r => r.id).sort();
    const expected = [
      'rust::ai-agent', 'rust::business-catalog', 'rust::flow-ai', 'rust::graph-algorithms',
      'rust::hermes-flow-bridge', 'rust::kg-hub', 'rust::operator-core', 'rust::operator-wasm',
      'rust::optimizer', 'rust::primiflow-core', 'rust::primiflow-fusion', 'rust::template-market',
      'rust::mox-expert', 'rust::mox-system', 'rust::mox-common-meta', 'rust::runtime'
    ].sort();
    assert.deepStrictEqual(ids, expected, '16 条 id 集合不匹配');
  });

  it('TR 1.2 engine-registry.js: engine::<ENGINE_NAME> 条目 = 16', function () {
    const { ENGINES } = require('../src/engine-universe/domain/engine-registry.js');
    const rust = ENGINES.filter(e => e.id && e.id.startsWith('engine::'));
    assert.strictEqual(rust.length, 16,
      `期望 16 条 engine:: 条目，实际 ${rust.length}。Ids: ${rust.map(r => r.id).join(', ')}`);
    for (const r of rust) {
      assert.strictEqual(r.kind, 'rust', `${r.id} .kind 必须为 rust`);
      assert.ok(r.engineName && r.engineName.startsWith('mox::'), `${r.id} .engineName 必须 mox::xxx`);
      assert.ok(r.crateId && r.crateId.length === 36, `${r.id} .crateId 非 UUIDv5`);
      assert.ok(r.path && /src\/lib\.rs$/.test(r.path), `${r.id} .path 需指向 .../src/lib.rs`);
    }
  });

  it('TR 1.3 tech-registry 7 条算法: CNM/PageRank/Brandes/Harmonic/degree/density/RAW_EXPAND 均 singleSource=true + main=rust + co_impl', function () {
    const { ALGORITHMS } = require('../src/project-atlas/domain/tech-registry.js');
    const keys = ['pagerank', 'cnm', 'brandes', 'harmonic', 'degree', 'density', 'expand'];
    const found = keys.map(k => ALGORITHMS.find(a => {
      const s = (a.id + ' ' + (a.name || '') + ' ' + (a.principle || '')).toLowerCase();
      if (k === 'expand') return s.includes('raw_expand') || s.includes('raw expand');
      return s.includes(k);
    }));
    for (let i = 0; i < keys.length; i++) {
      const a = found[i];
      assert.ok(a, `未找到算法 key=${keys[i]}`);
      assert.strictEqual(a.singleSource, true,
        `${a.id} singleSource 期望 true，实际 ${a.singleSource}`);
      assert.strictEqual(a.main, 'rust', `${a.id} main 期望 rust，实际 ${a.main}`);
      assert.deepStrictEqual(a.co_impl, ['node:GraphFormulas'],
        `${a.id} co_impl 期望 ['node:GraphFormulas']，实际 ${JSON.stringify(a.co_impl)}`);
      assert.strictEqual(a.main_crate, 'graph-algorithms', `${a.id} main_crate 期望 graph-algorithms`);
      assert.strictEqual(a.crateId, 'fbd31c6a-41cd-5274-be2f-2a28066eaf0a', `${a.id} crateId UUID 不匹配`);
    }
  });

  it('TR 1.4 self_sync_rust.js 生成 atlas_auto_registry_rust.json：entries ≥ 16 crate 且 ≥ 100 文件', function () {
    const script = path.join(__dirname, '..', 'src', 'project-atlas', 'scripts', 'self_sync_rust.js');
    const outJson = path.join(BACKEND, 'data', 'atlas_auto_registry_rust.json');
    if (!fs.existsSync(script)) {
      throw new Error(`self_sync 脚本不存在：${script}`);
    }
    execSync(`node "${script}"`, { cwd: BACKEND, stdio: ['ignore', 'pipe', 'inherit'] });
    assert.ok(fs.existsSync(outJson), `输出 JSON 不存在：${outJson}`);
    const content = JSON.parse(fs.readFileSync(outJson, 'utf8'));
    assert.ok(Array.isArray(content.entries), 'JSON shape: entries 非数组');
    assert.ok(content.entries.length >= 16,
      `crate 条目数 ${content.entries.length} < 16`);
    const totalFiles = content.entries.reduce((s, e) => s + (Array.isArray(e.files) ? e.files.length : 1), 0);
    assert.ok(totalFiles >= 100, `总文件数 ${totalFiles} < 100`);
    for (const e of content.entries) {
      assert.ok(e.crateName, 'entry 缺 crateName');
      assert.ok(e.crateId && e.crateId.length === 36, `${e.crateName || '?'} crateId 非 UUIDv5`);
      assert.ok(Array.isArray(e.fns), 'entry.fns 非数组');
      assert.ok(Array.isArray(e.structs), 'entry.structs 非数组');
      assert.ok(Array.isArray(e.consts), 'entry.consts 非数组');
      assert.ok(Array.isArray(e.files), 'entry.files 非数组');
      for (const f of e.files) {
        assert.ok(f.filePath, 'file 缺 filePath');
        assert.ok(Array.isArray(f.fns), 'file 缺 fns[]');
        assert.ok(Array.isArray(f.structs), 'file 缺 structs[]');
        assert.ok(Array.isArray(f.consts), 'file 缺 consts[]');
      }
    }
  });
});
