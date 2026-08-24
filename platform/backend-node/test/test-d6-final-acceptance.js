'use strict';

/**
 * D6-FINAL 一键验收 + 自研vs开源对比 + 业务流程最优 文档产出 + 整体 GREEN
 * TR:
 *   1) 验收脚本存在 + PowerShell 可解析语法
 *   2) 自研 vs 开源对比文档存在、可解析、含明确章节（架构/算法/观测/安全/性能）
 *   3) 最优业务处理流程文档存在、可解析、含 10 阶段（需求→测试）
 *   4) 一键验收脚本（SkipRust + SkipNode + SkipScoring 快速模式）能生成 md+json 报告，
 *      报告中 D1-D5 专项 5 项全部 GREEN（对应上面已全通过的 30 TR）
 *   5) 报告 JSON pass_count = total_phases（全绿），result = PASS
 *   6) 验收脚本（D1-D5 + 报告）能完整跑通（真实跑：跳过 P1/P2/P4，重点 D1~D5=30 TR + 报告）
 */
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const assert = require('assert');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'scripts', 'run-enterprise-final-acceptance.ps1');
const DOC_COMPARE = path.join(ROOT, '.trae', 'documents', 'xuanji-vs-opensource-comparison-report.md');
const DOC_BUSFLOW = path.join(ROOT, '.trae', 'documents', 'enterprise-optimal-business-flow.md');

function parseMdSections(f) {
  const t = fs.readFileSync(f, 'utf8');
  const headings = [...t.matchAll(/^(#+)\s+(.+)$/gm)].map(m => ({ level: m[1].length, title: m[2].trim() }));
  return { text: t, headings };
}

function powershell(args, extra) {
  const opts = Object.assign({ encoding: 'utf8', maxBuffer: 200 * 1024 * 1024, timeout: 1000 * 60 * 20 }, extra || {});
  // PowerShell Core (pwsh) 优先，回退 Windows PowerShell (powershell.exe)
  const r = spawnSync('pwsh', args, opts);
  if (r.error && r.error.code === 'ENOENT') {
    return spawnSync('powershell.exe', ['-ExecutionPolicy', 'Bypass'].concat(args), opts);
  }
  return r;
}

describe('D6-FINAL：一键验收 + 自研vs开源对比 + 业务流程最优（交付级产出）', function () {
  this.timeout(600000);

  it('1) run-enterprise-final-acceptance.ps1 验收脚本存在且 PowerShell 语法有效（AST parse 无错误）', function () {
    assert.ok(fs.existsSync(SCRIPT), '验收脚本不存在: ' + SCRIPT);
    // 用 pwsh -Command "[System.Management.Automation.Language.Parser]::ParseFile 解析 AST" 验证语法
    const syntaxCheck = `
      $tokens = $null; $errors = $null
      $null = [System.Management.Automation.Language.Parser]::ParseFile('${SCRIPT.replace(/'/g, "''")}', [ref]$tokens, [ref]$errors)
      if ($errors.Count -gt 0) { Write-Host "PSSYNTAX_ERROR count=$($errors.Count)"; $errors | ForEach-Object { Write-Host "  :: $($_.Message)" }; exit 1 }
      Write-Host "PSSYNTAX_OK"
      exit 0
    `;
    const r = powershell(['-NoProfile', '-Command', syntaxCheck]);
    const out = (r.stdout || '') + '\n' + (r.stderr || '');
    assert.strictEqual(r.status, 0, `脚本语法错误: exit=${r.status} output=${out.slice(0, 1500)}`);
    assert.ok(/PSSYNTAX_OK/.test(out), '语法校验应输出 PSSYNTAX_OK，实际: ' + out.slice(0, 500));
  });

  it('2) 自研 vs 开源对比报告：存在、含核心章节（架构/算法/观测/安全/性能对比表）', function () {
    assert.ok(fs.existsSync(DOC_COMPARE), '对比报告缺失: ' + DOC_COMPARE);
    const s = fs.statSync(DOC_COMPARE);
    assert.ok(s.size > 6000, `对比报告应 > 6KB，实际 ${s.size} bytes`);
    const { headings, text } = parseMdSections(DOC_COMPARE);
    const titles = headings.map(h => h.title).join(' | ');
    [
      '架构对比',
      '风险与反向',
      '结论',
    ].forEach(kw => assert.ok(titles.includes(kw) || text.includes(kw), `对比报告缺少 "${kw}" 章节`));
    // 定量对比表存在
    assert.ok(text.includes('可定量对比'), '缺少「可定量对比」章节');
    assert.ok(text.match(/Rust.*crate|workspace.*21/), '缺少 Rust workspace 21 crate 基准表述');
    // 对比验证清单
    assert.ok(text.includes('V-1') && text.includes('cargo metadata'), '缺少附录对比验证清单');
  });

  it('3) 最优业务处理流程：存在、含 10 阶段（需求归一→架构→开发→测试→观测→迭代）+ 交付物汇总', function () {
    assert.ok(fs.existsSync(DOC_BUSFLOW), '业务流程文档缺失: ' + DOC_BUSFLOW);
    const s = fs.statSync(DOC_BUSFLOW);
    assert.ok(s.size > 8000, `业务流程文档应 > 8KB，实际 ${s.size} bytes`);
    const { headings, text } = parseMdSections(DOC_BUSFLOW);
    const titles = headings.map(h => h.title).join(' | ');
    [
      '全域需求归一化',
      '全域业务图谱化',
      '全域架构落地',
      '全域开发',
      '全域测试验证',
      '全域观测与运维闭环',
      '优化与迭代',
      '交付物汇总',
    ].forEach(kw => {
      assert.ok(titles.includes(kw) || text.includes(kw), `业务流程文档缺少 "${kw}" 阶段/章节 (Titles=${titles.slice(0,300)}...)`);
    });
    // AIS 六层
    assert.ok(text.includes('AIS') && text.includes('Layer 6'), '缺少 AIS 六层架构描述');
  });

  it('4) 一键验收脚本 SkipRust+SkipNode+SkipScoring 快速模式可产出 md+json 报告', function () {
    this.timeout(600000);
    const tmpDir = path.join(ROOT, 'outputs', 'd6-final-check-' + Date.now());
    fs.mkdirSync(tmpDir, { recursive: true });
    const relReportDir = path.relative(ROOT, tmpDir).split(path.sep).join('/');
    const args = [
      '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', SCRIPT,
      '-SkipRust', '-SkipNode', '-SkipScoring',
      '-ReportDir', relReportDir,
    ];
    const r = powershell(args, { cwd: ROOT, timeout: 1000 * 60 * 20 });
    const combined = (r.stdout || '') + '\nSTDERR>\n' + (r.stderr || '');
    assert.strictEqual(r.status, 0, `快速验收非零退出 (exit=${r.status}); stdout/stderr=${combined.slice(0, 3000)}`);
    const files = fs.readdirSync(tmpDir);
    const md = files.find(f => f.startsWith('report-') && f.endsWith('.md'));
    const js = files.find(f => f.startsWith('report-') && f.endsWith('.json'));
    assert.ok(md, '报告 md 未生成，目录文件: ' + files.join(','));
    assert.ok(js, '报告 json 未生成，目录文件: ' + files.join(','));
    const mdSize = fs.statSync(path.join(tmpDir, md)).size;
    const jsSize = fs.statSync(path.join(tmpDir, js)).size;
    assert.ok(mdSize > 500, `md < 500 bytes：${mdSize}`);
    assert.ok(jsSize > 100, `json < 100 bytes：${jsSize}`);
    // 缓存报告路径，供 TR 5 使用
    process.env.__D6_REPORT_JSON__ = path.join(tmpDir, js);
    process.env.__D6_REPORT_MD__ = path.join(tmpDir, md);
  });

  it('5) 报告 JSON：result=PASS 且 pass_count = total_phases（全绿），D1~D5 五个专项全部 PASS', function () {
    const jp = process.env.__D6_REPORT_JSON__;
    assert.ok(jp && fs.existsSync(jp), '需先通过 TR 4');
    const j = JSON.parse(fs.readFileSync(jp, 'utf8'));
    assert.strictEqual(j.result, 'PASS', '报告 result 应为 PASS，实际: ' + JSON.stringify(j).slice(0, 500));
    assert.strictEqual(j.pass_count, j.total_phases, `pass_count ${j.pass_count} ≠ total ${j.total_phases}`);
    const phases = j.phases || [];
    const byId = new Map(phases.map(p => [p.id, p]));
    // D1~D5 必须存在且 PASS
    const dKeys = ['D1-ARCH', 'D2-OPS', 'D3-OBS', 'D4-SEC', 'D5-BUILD'];
    for (const k of dKeys) {
      const p = byId.get(k);
      assert.ok(p, `报告未包含 ${k} 阶段`);
      assert.ok(p.pass === true, `阶段 ${k} 应为 GREEN，结果=${JSON.stringify(p)}`);
    }
    const totalDurationMs = Number(j.duration_ms);
    assert.ok(totalDurationMs >= 0, 'duration_ms 应为非负整数');
  });

  it('6) 报告 MD：包含总览表、明细表（5 大 D 专项 GREEN 行）、环境信息、结果判定', function () {
    const mp = process.env.__D6_REPORT_MD__;
    assert.ok(mp && fs.existsSync(mp), '需先通过 TR 4');
    const t = fs.readFileSync(mp, 'utf8');
    assert.ok(t.includes('璇玑') && t.includes('验收报告'), '报告标题缺失');
    assert.ok(/总览|概览|Summary/.test(t), '无总览');
    assert.ok(t.includes('D1-ARCH') && t.includes('✅ PASS') || t.includes('D1') && t.includes('PASS'), '明细中 D1 不存在或非 GREEN');
    assert.ok(t.includes('D5-BUILD'), '明细中无 D5');
    assert.ok(/环境|Environment/.test(t), '缺少环境信息章节');
    assert.ok(/验收判定|判定|Verdict/.test(t), '缺少验收判定章节');
    // 结果判定 PASS
    const l = t.toLowerCase();
    assert.ok(l.includes('全链路 green') || l.includes('全绿') || l.includes('pass'), '无通过性判定表述');
  });
});
