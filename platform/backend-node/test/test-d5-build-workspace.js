'use strict';

/**
 * D5-BUILD：Cargo Workspace 构建契约一致性
 * TR:
 *   1) 顶层 workspace Cargo.toml members 清单：每个路径物理存在、有 Cargo.toml 文件、且子 crate 含 src/lib.rs 或 src/main.rs。
 *   2) 反向：platform/services/*、platform/gateway/* 里所有 Cargo.toml，要么在 workspace members 中，要么显式声明 workspace=false。
 *   3) `cargo metadata --no-deps --format-version 1` 成功返回 JSON，包含所有 workspace member 包。
 *   4) cargo metadata 中 package 数量 = 21（与 members 一致），没有 workspace 级 resolve_error / unstable_flags 告警。
 */
const fs = require('fs');
const path = require('path');
const { execSync, spawnSync } = require('child_process');
const assert = require('assert');

const REPO_ROOT = path.resolve(__dirname, '..', '..', '..');
const WS_TOML = path.join(REPO_ROOT, 'Cargo.toml');

function parseMembers(tomlText) {
  // 轻量 TOML 解析：只针对 [workspace] members = [ ... ]
  const m = tomlText.match(/members\s*=\s*\[([\s\S]*?)\]/);
  if (!m) throw new Error('未找到 workspace.members 数组');
  const block = m[1];
  const items = [];
  const re = /"([^"]+)"|'([^']+)'/g;
  let g;
  while ((g = re.exec(block))) items.push((g[1] || g[2] || '').trim());
  return items.filter(Boolean);
}

function dirHasCargoToml(dir) {
  return fs.existsSync(path.join(dir, 'Cargo.toml'));
}

function readCargoToml(dir) {
  try { return fs.readFileSync(path.join(dir, 'Cargo.toml'), 'utf8'); } catch (_) { return ''; }
}

function hasWorkspaceFalse(dir) {
  const t = readCargoToml(dir);
  return /\[workspace\][^\n]*\n\s*members\s*=/.test(t) === false && /workspace\s*=\s*false/.test(t);
}

function hasSrcRoot(dir) {
  return fs.existsSync(path.join(dir, 'src', 'lib.rs'))
    || fs.existsSync(path.join(dir, 'src', 'main.rs'))
    || fs.existsSync(path.join(dir, 'src', 'main')); // main 存在可能
}

function runCargoMetadata() {
  const r = spawnSync('cargo', ['metadata', '--no-deps', '--format-version', '1', '--quiet'], {
    cwd: REPO_ROOT,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    env: Object.assign({}, process.env, { CARGO_NET_OFFLINE: process.env.CARGO_D5_OFFLINE || 'true' }),
  });
  let stdout = (r.stdout || '').toString();
  let stderr = (r.stderr || '').toString();
  if (r.status !== 0) {
    // offline 失败则尝试 online
    if (/offline/.test(stderr) || /registry/i.test(stderr)) {
      const r2 = spawnSync('cargo', ['metadata', '--no-deps', '--format-version', '1', '--quiet'], {
        cwd: REPO_ROOT, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024,
      });
      return { status: r2.status, stdout: (r2.stdout || '').toString(), stderr: (r2.stderr || '').toString() };
    }
  }
  return { status: r.status, stdout, stderr };
}

describe('D5-BUILD：Cargo Workspace 成员一致性 + cargo metadata 校验', function () {
  this.timeout(180000);

  let members, memberSet;
  before(function () {
    const txt = fs.readFileSync(WS_TOML, 'utf8');
    members = parseMembers(txt);
    memberSet = new Set(members);
    console.log('[D5] workspace members count:', members.length);
    assert.ok(members.length > 0, 'workspace members 不能为空');
  });

  it('1) workspace.members 中每个路径都存在、有 Cargo.toml、具备 src 入口（lib.rs 或 main.rs）', function () {
    const bad = [];
    for (const rel of members) {
      const abs = path.join(REPO_ROOT, rel);
      if (!fs.existsSync(abs)) { bad.push(`${rel}: 目录不存在`); continue; }
      if (!dirHasCargoToml(abs)) { bad.push(`${rel}: 缺少 Cargo.toml`); continue; }
      if (!hasSrcRoot(abs)) { bad.push(`${rel}: 缺少 src/lib.rs 或 src/main.rs`); continue; }
    }
    assert.deepStrictEqual(bad, [], '以下 workspace 成员存在结构异常: ' + bad.join(' ; '));
  });

  it('2) 反向：platform/services/* 与 platform/gateway/* 下的 Cargo.toml 均在 workspace members 中（或明确 workspace=false）', function () {
    const searchRoots = [
      path.join(REPO_ROOT, 'platform', 'services'),
      path.join(REPO_ROOT, 'platform', 'gateway'),
    ];
    const orphans = [];
    for (const root of searchRoots) {
      if (!fs.existsSync(root)) continue;
      for (const name of fs.readdirSync(root)) {
        const dir = path.join(root, name);
        if (!fs.statSync(dir).isDirectory()) continue;
        if (!dirHasCargoToml(dir)) continue;
        // 计算相对路径 (反斜 -> 正斜)
        let rel = path.relative(REPO_ROOT, dir).split(path.sep).join('/');
        if (memberSet.has(rel)) continue;
        if (hasWorkspaceFalse(dir)) continue;
        // 子项目（如 business-court-docs）可能在其他目录有自己的 workspace；仅当有 package 声明但未加入成员/未标 workspace=false 时举报
        const tt = readCargoToml(dir);
        if (/\[package\]/.test(tt) && !/\[\s*workspace\s*\]/.test(tt) && !/workspace\s*=\s*false/.test(tt)) {
          orphans.push(rel);
        }
      }
    }
    assert.deepStrictEqual(orphans, [], '存在未纳入 workspace 且未标 workspace=false 的孤儿 crate: ' + orphans.join(','));
  });

  it('3) cargo metadata 成功执行、不报错（exit code 0）', function () {
    this.timeout(180000);
    const r = runCargoMetadata();
    if (r.status !== 0) {
      console.warn('[D5] cargo metadata stderr:', r.stderr.slice(0, 1000));
    }
    assert.strictEqual(r.status, 0, `cargo metadata 非零退出: code=${r.status}; stderr=${r.stderr.slice(0, 500)}`);
    assert.ok(r.stdout.trim().startsWith('{'), 'cargo metadata stdout 必须是 JSON 对象，实际 prefix=' + r.stdout.trim().slice(0, 50));
  });

  it('4) cargo metadata packages 数量与 workspace members 一致（21），每个包 manifest_path 与成员路径对应', function () {
    this.timeout(180000);
    const r = runCargoMetadata();
    if (r.status !== 0) { this.skip(); return; }
    const j = JSON.parse(r.stdout);
    assert.ok(Array.isArray(j.packages), 'metadata 无 packages 数组');
    const memExpected = members.length;
    // packages 可能包含 workspace 外的被 path_dep 引入；只统计 workspace_members 字段（若有）。
    const wsMembers = Array.isArray(j.workspace_members) ? j.workspace_members : null;
    const pkgs = j.packages;
    console.log(`[D5] workspace_members field=${wsMembers ? wsMembers.length : 'N/A'}；packages=${pkgs.length}；members manifest=${memExpected}`);
    if (wsMembers) {
      // 用 workspace_members 作更准确的对照：每个 id 应对应一个 package
      assert.strictEqual(wsMembers.length, memExpected, `workspace_members 数量 ${wsMembers.length} ≠ members ${memExpected}`);
      for (const id of wsMembers) {
        const pkg = pkgs.find(p => p.id === id);
        assert.ok(pkg, `workspace_member ${id} 在 packages 中找不到`);
        // manifest_path 绝对路径，判断是否与成员目录匹配
        const mf = (pkg.manifest_path || '').replace(/\\/g, '/');
        const matched = members.some(rel => mf.endsWith(`/${rel}/Cargo.toml`));
        assert.ok(matched, `workspace 成员包 manifest_path=${mf} 不匹配任何 member 目录: ` + members.join(','));
      }
    } else {
      // 回退：直接 count packages
      assert.strictEqual(pkgs.length, memExpected, 'packages 数量不一致: packages=' + pkgs.length + ' members=' + memExpected);
    }
  });

  it('5) cargo metadata 无 workspace resolve_error、无 fatal 错误字段', function () {
    this.timeout(180000);
    const r = runCargoMetadata();
    if (r.status !== 0) { this.skip(); return; }
    const j = JSON.parse(r.stdout);
    assert.ok(!j.resolve_error || j.resolve_error === null || j.resolve_error === '', 'resolve_error: ' + String(j.resolve_error));
  });
});
