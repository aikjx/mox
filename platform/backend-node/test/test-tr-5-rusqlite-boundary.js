'use strict';
/**
 * T5.1 rusqlite 收拢边界：ai-agent / primiflow-core Cargo.toml 无 rusqlite
 *
 * RED ： grep rusqlite 两文件 应 存在（≥1 匹配） -> 断言失败（RED）
 * GREEN： grep rusqlite 两文件 应 0 匹配（断言 0 = 0，GREEN）
 */
const assert = require('assert');
const fs = require('fs');
const path = require('path');

const REPO = path.join(__dirname, '..', '..', '..');

describe('T5.1 · rusqlite 收拢：ai-agent / primiflow-core Cargo.toml 不得出现 rusqlite', function () {
  const targets = [
    path.join(REPO, 'platform', 'services', 'ai-agent', 'Cargo.toml'),
    path.join(REPO, 'platform', 'services', 'primiflow-core', 'Cargo.toml')
  ];

  it('T5.1: 两 crate Cargo.toml 对 rusqlite 0 匹配（GREEN：0；RED：≥1）', function () {
    const matches = [];
    for (const t of targets) {
      assert.ok(fs.existsSync(t), `文件不存在：${t}`);
      const lines = fs.readFileSync(t, 'utf8').split(/\r?\n/);
      lines.forEach((l, n) => {
        if (/\brusqlite\b/.test(l)) matches.push(`${path.basename(path.dirname(path.dirname(t)))}/${path.basename(path.dirname(t))}/Cargo.toml:${n + 1}:${l.trim()}`);
      });
    }
    assert.strictEqual(matches.length, 0,
      `发现 rusqlite 直接依赖 ${matches.length} 处：\n  - ${matches.join('\n  - ')}`);
  });
});
