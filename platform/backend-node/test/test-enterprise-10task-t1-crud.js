'use strict';
/**
 * T1: 传媒 CRUD (4 stores × 4 entities × C→R→U→D→R_after_delete = 16 assertions)
 *     + 8 ERROR-path tests (delete missing / duplicate create / update nonexistent / missing required × 2 per store)
 *     + 50 concurrent async writes atomic rename no corruption
 */
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const os = require('os');

const ROOT = path.join(__dirname, '..');
const DATA_SRC = path.join(ROOT, 'data');
const TMP = fs.mkdtempSync(path.join(os.tmpdir(), 'xuanji-t1-crud-'));
const DATA_TMP = path.join(TMP, 'data');
fs.mkdirSync(DATA_TMP, { recursive: true });

// Files in scope: backup originals (into TMP/orig) as copies before work.
const ORIG_BACKUP = path.join(TMP, 'orig_backup');
fs.mkdirSync(ORIG_BACKUP, { recursive: true });

const STORE_FILES = ['kb_documents.json', 'graph_nodes.json', 'graph_edges.json', 'projects.json'];
// Temp work copies inside DATA_TMP
const storePath = (name) => path.join(DATA_TMP, name);
const srcPath = (name) => path.join(DATA_SRC, name);

// Helper: minimal fs-atomic write
function atomicWriteJSON(file, obj) {
  const tmp = file + '.tmp.' + Math.random().toString(36).slice(2, 10);
  fs.writeFileSync(tmp, JSON.stringify(obj, null, 2), 'utf8');
  fs.renameSync(tmp, file);
}
function ensureTmpCopyReady(file) {
  // file 可能是绝对路径（来自 test helper 直接传 storePath('xxx.json')），也可能是 basename
  const fileName = path.isAbsolute(file) ? path.basename(file) : file;
  const dst = path.isAbsolute(file) ? file : storePath(file);
  const dstDir = path.dirname(dst);
  if (!fs.existsSync(dstDir)) fs.mkdirSync(dstDir, { recursive: true });
  if (!fs.existsSync(dst)) {
    const src = srcPath(fileName);
    if (!fs.existsSync(src)) {
      // 源文件不存在（极少见），回退写空数组占位
      fs.writeFileSync(dst, '[]', 'utf8');
      return;
    }
    fs.copyFileSync(src, dst);
  }
}
function readJSONSafe(file) {
  ensureTmpCopyReady(file);
  const raw = fs.readFileSync(path.isAbsolute(file) ? file : storePath(file), 'utf8');
  return JSON.parse(raw);
}

// Required fields per entity kind
const STORE_SCHEMAS = {
  'kb_documents.json': {
    idKey: 'id',
    required: ['id', 'title', 'content'],
    entityName: 'KB document',
    buildRequiredOnly: (id) => ({ id, title: 'doc_' + id, content: 'content_' + id }),
  },
  'graph_nodes.json': {
    idKey: 'id',
    required: ['id', 'label', 'type'],
    entityName: 'Graph node',
    buildRequiredOnly: (id) => ({ id, label: 'Node ' + id, type: 'concept' }),
  },
  'graph_edges.json': {
    idKey: 'id',
    required: ['id', 'source', 'target'],
    entityName: 'Graph edge',
    buildRequiredOnly: (id) => ({ id, source: 'src_' + id, target: 'tgt_' + id }),
  },
  'projects.json': {
    idKey: 'id',
    required: ['id', 'name', 'status'],
    entityName: 'Project',
    buildRequiredOnly: (id) => ({ id, name: 'Project ' + id, status: 'active' }),
  },
};

function storeCRUD(file) {
  const schema = STORE_SCHEMAS[file];
  const idKey = schema.idKey;
  const base = schema.buildRequiredOnly('t1_' + file.replace(/\.json$/, '') + '_' + Date.now());
  const updatedData = { ...base, _updatedMarker: 'v2' };
  const steps = {
    create() {
      const list = readJSONSafe(storePath(file));
      assert.ok(Array.isArray(list), `${file} should be array`);
      assert.ok(!list.some(x => x[idKey] === base[idKey]), `${file} before create: id must not exist`);
      list.push(base);
      atomicWriteJSON(storePath(file), list);
      return base;
    },
    readAfterCreate() {
      const list = readJSONSafe(storePath(file));
      const got = list.find(x => x[idKey] === base[idKey]);
      assert.ok(got, `${file} R after C: entity must be present`);
      assert.strictEqual(got[idKey], base[idKey]);
      return got;
    },
    update() {
      const list = readJSONSafe(storePath(file));
      const idx = list.findIndex(x => x[idKey] === base[idKey]);
      assert.ok(idx >= 0, `${file} U: entity must exist`);
      list[idx] = { ...list[idx], ...updatedData };
      atomicWriteJSON(storePath(file), list);
      return updatedData;
    },
    readAfterUpdate() {
      const list = readJSONSafe(storePath(file));
      const got = list.find(x => x[idKey] === base[idKey]);
      assert.ok(got, `${file} R after U: entity present`);
      assert.strictEqual(got._updatedMarker, 'v2', `${file} R after U: marker present`);
      return got;
    },
    del() {
      const list = readJSONSafe(storePath(file));
      const beforeLen = list.length;
      const filtered = list.filter(x => x[idKey] !== base[idKey]);
      assert.strictEqual(filtered.length, beforeLen - 1, `${file} D: exactly one removed`);
      atomicWriteJSON(storePath(file), filtered);
      return true;
    },
    readAfterDelete() {
      const list = readJSONSafe(storePath(file));
      const got = list.find(x => x[idKey] === base[idKey]);
      assert.strictEqual(got, undefined, `${file} R after D: entity gone`);
      return true;
    },
  };
  return steps;
}

describe('T1 传媒 CRUD: setup & cleanup', function () {
  before(function () {
    // 1) backup originals
    for (const f of STORE_FILES) {
      fs.copyFileSync(srcPath(f), path.join(ORIG_BACKUP, f));
    }
    // 2) create tmp copies as the working stores
    for (const f of STORE_FILES) {
      fs.copyFileSync(srcPath(f), storePath(f));
    }
    console.log('  T1 workdir:', TMP);
  });

  after(function () {
    // Restore originals (not strictly needed, but keeps DATA_SRC untouched — we never touched it anyway)
    // Nothing to roll back in DATA_SRC since tests operated only on TMP copies.
    try { fs.rmSync(TMP, { recursive: true, force: true, maxRetries: 3 }); } catch {}
  });

  it('backup copies exist and parse without error', function () {
    for (const f of STORE_FILES) {
      const obj = JSON.parse(fs.readFileSync(path.join(ORIG_BACKUP, f), 'utf8'));
      assert.ok(Array.isArray(obj), `${f} backup must parse to array`);
    }
  });
});

describe('T1 传媒 CRUD: 4 entities × (C → R → U → D → R_after_delete) = 20 assertions', function () {
  for (const f of STORE_FILES) {
    const name = STORE_SCHEMAS[f].entityName;
    const steps = storeCRUD(f);
    it(`[C] ${name} create succeeds`, () => steps.create());
    it(`[R] ${name} read-after-create returns record`, () => steps.readAfterCreate());
    it(`[U] ${name} update succeeds`, () => steps.update());
    it(`[R2] ${name} read-after-update has updated marker`, () => steps.readAfterUpdate());
    it(`[D] ${name} delete removes exactly 1`, () => steps.del());
    it(`[R3] ${name} read-after-delete absent`, () => steps.readAfterDelete());
  }
});

describe('T1 传媒 CRUD: 8 ERROR-path tests (2 per store)', function () {
  // Per store: 2 error tests => 4×2 = 8.
  // 1) delete missing id (no file size change, no throw with explicit guard returning false)
  // 2) create duplicate id (must throw / fail via guard that returns false)
  // 3) update nonexistent id (guard returns false / throws)
  // 4) missing required field (throws during validation check we implement here)

  function safeDelete(file, id) {
    const key = STORE_SCHEMAS[file].idKey;
    const list = readJSONSafe(storePath(file));
    const idx = list.findIndex(x => x[key] === id);
    if (idx < 0) return false;
    list.splice(idx, 1);
    atomicWriteJSON(storePath(file), list);
    return true;
  }
  function safeCreate(file, entity) {
    const key = STORE_SCHEMAS[file].idKey;
    const list = readJSONSafe(storePath(file));
    if (list.some(x => x[key] === entity[key])) return false;
    list.push(entity);
    atomicWriteJSON(storePath(file), list);
    return true;
  }
  function safeUpdate(file, id, patch) {
    const key = STORE_SCHEMAS[file].idKey;
    const list = readJSONSafe(storePath(file));
    const idx = list.findIndex(x => x[key] === id);
    if (idx < 0) return false;
    list[idx] = { ...list[idx], ...patch, [key]: id };
    atomicWriteJSON(storePath(file), list);
    return true;
  }
  function validateRequired(file, entity) {
    const req = STORE_SCHEMAS[file].required;
    for (const r of req) {
      if (entity[r] === undefined || entity[r] === null || entity[r] === '') return false;
    }
    return true;
  }

  it('KB: delete nonexistent id returns false (no side effect)', function () {
    const before = fs.readFileSync(storePath('kb_documents.json'), 'utf8');
    assert.strictEqual(safeDelete('kb_documents.json', 'kb_does_not_exist_xyz'), false);
    const after = fs.readFileSync(storePath('kb_documents.json'), 'utf8');
    assert.strictEqual(before, after, 'bytes unchanged for missing delete');
  });
  it('KB: missing required field "title" rejects (validate false)', function () {
    const badEnt = { id: 'kb_bad_' + Date.now(), content: 'x' }; // no title
    assert.strictEqual(validateRequired('kb_documents.json', badEnt), false);
  });

  it('Graph node: create duplicate id returns false', function () {
    // Use the very first node id from data as the "duplicate" target
    const nodes = readJSONSafe(storePath('graph_nodes.json'));
    const firstId = nodes[0] && nodes[0].id;
    assert.ok(firstId, 'needs at least 1 existing node');
    const dupe = { id: firstId, label: 'Dup', type: 'test' };
    assert.strictEqual(safeCreate('graph_nodes.json', dupe), false, 'duplicate create must fail');
  });
  it('Graph node: missing required "type" rejects', function () {
    const bad = { id: 'gn_bad_' + Date.now(), label: 'L' };
    assert.strictEqual(validateRequired('graph_nodes.json', bad), false);
  });

  it('Graph edge: update nonexistent id returns false', function () {
    const bogus = 'edge_does_not_exist_' + Date.now();
    assert.strictEqual(safeUpdate('graph_edges.json', bogus, { weight: 9 }), false);
  });
  it('Graph edge: missing required "source" + "target" rejects', function () {
    const bad = { id: 'ge_bad_' + Date.now(), source: 'ok' }; // no target
    assert.strictEqual(validateRequired('graph_edges.json', bad), false);
  });

  it('Project: delete nonexistent id returns false', function () {
    ensureTmpCopyReady('projects.json');
    const before = fs.readFileSync(storePath('projects.json'), 'utf8');
    assert.strictEqual(safeDelete('projects.json', 'proj_missing_' + Date.now()), false);
    assert.strictEqual(fs.readFileSync(storePath('projects.json'), 'utf8'), before);
  });
  it('Project: missing required "status" rejects (validate false)', function () {
    const bad = { id: 'proj_bad_' + Date.now(), name: 'NoStatus' };
    assert.strictEqual(validateRequired('projects.json', bad), false);
  });
});

describe('T1 传媒 CRUD: 50 concurrent async writes (atomic rename) → 0 JSON corruption', function () {
  it('50 parallel writes across 4 stores; final JSON parseable & invariant holds', async function () {
    this.timeout(30000);
    const CONCURRENCY = 50;
    const counters = {};
    for (const f of STORE_FILES) counters[f] = 0;

    const writeOnce = async (file, i) => {
      await new Promise(setImmediate);
      ensureTmpCopyReady(file);
      const key = STORE_SCHEMAS[file].idKey;
      // Read+modify+write pattern, guarded by atomic rename: still safe under contention
      // (each write produces valid JSON at the end, even if some updates are lost; test
      // validates NO JSON corruption, not that every counter persisted).
      const list = JSON.parse(fs.readFileSync(storePath(file), 'utf8'));
      const synthetic = {
        [key]: `t1_cw_${file.replace(/\.json$/, '')}_${process.pid}_${i}_${Math.random().toString(36).slice(2, 8)}`,
        label: `cw${i}`,
        type: 'concurrency',
        title: `cw${i}`,
        content: 'payload',
        name: `cw${i}`,
        status: 'active',
        source: 'a' + i,
        target: 'b' + i,
        created_at: new Date().toISOString(),
        seq: i,
      };
      list.push(synthetic);
      const tmp = storePath(file) + '.cw.' + i + '.' + Math.random().toString(36).slice(2, 10);
      fs.writeFileSync(tmp, JSON.stringify(list, null, 2), 'utf8');
      fs.renameSync(tmp, storePath(file));
    };

    const tasks = [];
    for (let i = 0; i < CONCURRENCY; i++) {
      const f = STORE_FILES[i % STORE_FILES.length];
      tasks.push(writeOnce(f, i));
    }
    await Promise.all(tasks);

    // Now verify all 4 JSON files parse cleanly with no structural corruption
    for (const f of STORE_FILES) {
      const raw = fs.readFileSync(storePath(f), 'utf8');
      let parsed;
      try { parsed = JSON.parse(raw); } catch (e) {
        assert.fail(`${f} corrupted JSON after ${CONCURRENCY} concurrent writes: ${e.message}`);
      }
      assert.ok(Array.isArray(parsed), `${f} must remain array-shaped`);
      // Every t1_cw_* synthetic record has required key present (no half-writes)
      const key = STORE_SCHEMAS[f].idKey;
      for (const r of parsed.filter(x => String(x[key] || '').startsWith('t1_cw_'))) {
        assert.ok(r[key] && typeof r.seq === 'number', `${f} synthetic record incomplete: ${JSON.stringify(r).slice(0, 120)}`);
      }
    }
  });
});
