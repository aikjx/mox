/* eslint-disable */
// 先 RED：T0 DryRun 失败（脚本/数据尚未就绪）→ 然后 GREEN 实现
const fs = require('fs');
const path = require('path');
const assert = require('assert');

describe('T0 评分基础设施（DryRun RED→GREEN）', function () {
  const root = path.resolve(__dirname, '..');
  const def = path.join(root, 'data', 'enterprise_10task_definitions.json');
  const script = path.join(root, 'scripts', 'run-10task-rubric.ps1');
  const dataDir = path.join(root, 'data');

  it('T0-RED-1: definitions.json 文件存在且可被 JSON.parse（先 FAIL）', function () {
    assert.ok(fs.existsSync(def), `[RED 必须先看到文件] 缺少: ${def}`);
    const parsed = JSON.parse(fs.readFileSync(def, 'utf8'));
    assert.ok(Array.isArray(parsed.tasks) && parsed.tasks.length === 10, `10 类任务定义必须为 10 项，实际 ${parsed.tasks ? parsed.tasks.length : 0}`);
    assert.ok(parsed.enterpriseThresholds.totalScoreMin === 90, `totalScoreMin 应为 90，实际 ${parsed.enterpriseThresholds.totalScoreMin}`);
    assert.ok(parsed.enterpriseThresholds.perTaskMin === 8, `perTaskMin 应为 8`);
  });

  it('T0-RED-2: 10 类任务 id 严格为 t1..t10 顺序不缺不重', function () {
    const parsed = JSON.parse(fs.readFileSync(def, 'utf8'));
    const ids = parsed.tasks.map(t => t.id).sort();
    const expected = ['t1','t10','t2','t3','t4','t5','t6','t7','t8','t9'].sort();
    assert.deepStrictEqual(ids, expected, `任务 id 集合不匹配：\n期望 ${JSON.stringify(expected)}\n实际 ${JSON.stringify(ids)}`);
    for (const t of parsed.tasks) {
      assert.ok(typeof t.rule.tr === 'string' && t.rule.tr.length >= 10, `${t.id} rule.tr 不能为空`);
      assert.ok(typeof t.rubric.dimension === 'string' && t.rubric.passThreshold >= 3, `${t.id} rubric 缺字段`);
    }
  });

  it('T0-RED-3: run-10task-rubric.ps1 存在且 DryRun 表头 10 类打印', async function () {
    // 用 node 子进程运行；如果脚本不存在必然 fail 这就是 RED 阶段的作用
    assert.ok(fs.existsSync(script), `[RED→GREEN] 评分脚本不存在: ${script}`);
    // DryRun 输出必须含 "T1..T10" 10 个 id 字符串
    const { execSync } = require('child_process');
    // Windows 环境：pwsh = PowerShell 7+（脚本 #requires -Version 7）。若缺失则回退 powershell
    const psBin = (() => { try { return execSync('where pwsh', { encoding: 'utf8', stdio: ['ignore','pipe','ignore'] }).split(/\r?\n/)[0].trim() || 'powershell'; } catch { return 'powershell'; } })();
    const out = execSync(`"${psBin}" -NoProfile -ExecutionPolicy Bypass -File "${script}" -DryRun`, {
      cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe']
    });
    for (let i = 1; i <= 10; i++) {
      assert.ok(out.includes(`T${i}`) || out.includes(`t${i}`), `DryRun 输出缺少 T${i}。\nActual: ${out.slice(0, 1500)}`);
    }
    assert.ok(out.includes('企业准入总阈值 >= 90'), '缺少总阈值提示');
    assert.ok(out.includes('单项最低 >= 8'), '缺少单项阈值提示');
  });

  it('T0-RED-4: data 目录下 enterprise_10task_scores.json 的写入能力（脚本 DryRun 创建空壳）', function () {
    const score = path.join(dataDir, 'enterprise_10task_scores.json');
    // GREEN 阶段 DryRun 已经生成过空壳
    assert.ok(fs.existsSync(score), `DryRun 应生成 score json 空壳：${score}`);
    const s = JSON.parse(fs.readFileSync(score, 'utf8'));
    assert.ok(typeof s.meta === 'object' && s.meta.schemaVersion === 1, 'schema version 缺失');
  });
});
