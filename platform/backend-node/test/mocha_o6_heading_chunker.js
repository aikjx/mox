'use strict';
/**
 * mocha 单元测试：T12 O6 Heading-Aware Chunker（T12 TR-1~TR-9）
 */
const assert = require('assert');
const { splitDocument, summarizeChunks, parseHeading, DEFAULT_MAX_CHARS } = require('../src/kb/domain/heading-chunker');

describe('[T12-AC1] parseHeading 标题解析（6 种形态）', function () {
  it('ATX H1/H3/H6', () => {
    assert.deepStrictEqual(parseHeading('# 引言').level, 1);
    assert.deepStrictEqual(parseHeading('###  设计目标  ').level, 3);
    assert.strictEqual(parseHeading('###  设计目标  ').text, '设计目标');
    assert.deepStrictEqual(parseHeading('###### tail').level, 6);
  });
  it('Setext H1/H2（依赖下一行判定，使用 ctx 注入）', () => {
    const r1 = parseHeading('Getting Started', { nextLineType: 'setext_h1' });
    assert.deepStrictEqual(r1.level, 1);
    assert.strictEqual(r1.text, 'Getting Started');
    const r2 = parseHeading('Install', { nextLineType: 'setext_h2' });
    assert.deepStrictEqual(r2.level, 2);
  });
  it('中文 第X章 / 第X节', () => {
    assert.ok(parseHeading('第一章 绪论'), '"第一章 绪论"应识别');
    assert.strictEqual(parseHeading('第一章 绪论').level, 1);
    assert.strictEqual(parseHeading('第一章 绪论').text, '绪论');
    const s = parseHeading('第 3 章 系统架构');
    assert.ok(s, '带空格的"第 3 章"应识别');
    assert.strictEqual(s.level, 1);
    assert.strictEqual(s.text, '系统架构');
  });
  it('一、二、 中文枚举', () => {
    const r = parseHeading('三、核心模块');
    assert.ok(r, '应解析为标题');
    assert.strictEqual(r.level, 2, `层级应为 2 (浅层中文章)，实际 ${r.level}`);
    assert.strictEqual(r.text, '核心模块');
  });
  it('1. / 1.1 / 1.1.1 数字分级', () => {
    const a = parseHeading('1. 背景');
    const b = parseHeading('1.2 目标');
    const c = parseHeading('3.4.2 细节');
    assert.ok(a, '"1. 背景"未识别');
    assert.strictEqual(a.level, 2, `1. → 层级应为 2，实际 ${a.level}`);
    assert.strictEqual(b?.level, 3, `1.2 → 层级应为 3，实际 ${b?.level}`);
    assert.strictEqual(c?.level, 4, `3.4.2 → 层级应为 4，实际 ${c?.level}`);
  });
  it('空行/正文不解析为标题', () => {
    assert.strictEqual(parseHeading(''), null);
    assert.strictEqual(parseHeading('这是一段普通正文而不是标题，字数远超 120，用来确保 parser 不会把长正文误判为数字编号型标题，比如 2024 年开始的一些工作'), null);
    assert.strictEqual(parseHeading('普通正文段落首句。'), null);
  });
});

describe('[T12-AC2] splitDocument 最小文档：H1 加两段 → 每 chunk heading_path 非空', function () {
  const doc = '# 项目介绍\n\n本项目是企业级 AI 平台。\n\n项目主要用于图谱分析与 Agent 编排。\n';
  it('返回 chunks >= 1，heading_path 首项 = "项目介绍"', () => {
    const chunks = splitDocument(doc);
    assert.ok(chunks.length >= 1, '至少 1 块');
    for (const c of chunks) {
      assert.ok(Array.isArray(c.heading_path), 'heading_path 必须是数组');
      assert.strictEqual(c.heading_path[0], '项目介绍');
    }
  });
  it('chunk 文本都应包含正文内容（不只是标题空壳）', () => {
    const chunks = splitDocument(doc);
    const allText = chunks.map(c => c.text).join('\n');
    assert.ok(allText.includes('企业级 AI 平台'), '含首段');
    assert.ok(allText.includes('图谱分析与 Agent 编排'), '含第二段');
  });
});

describe('[T12-AC3] cross_heading_chunks = 0：绝不跨节切割', function () {
  const doc = [
    '# 第一章 A',
    '',
    'p1 '.repeat(50),  // 约 150 字
    '',
    '# 第二章 B',
    '',
    'p2 '.repeat(50),
    '',
    '## 2.1 细节',
    '',
    'p3 '.repeat(80),
  ].join('\n');

  it('summarizeChunks().cross_heading_chunks 必须是 0', () => {
    const chunks = splitDocument(doc, { max_chars: 260, overlap: 40 });
    const s = summarizeChunks(chunks);
    // 若发现跨节，说明切块 bug
    assert.strictEqual(s.cross_heading_chunks, 0,
      `cross_heading_chunks 应 =0。 各块 path:\n` +
      chunks.map((c,i) => `  [${i}] ${c.heading_path.join('/')}`).join('\n'));
  });

  it('chunk heading_path 中若含 "第二章 B"，该 chunk 文本不应出现 "第一章"', () => {
    const chunks = splitDocument(doc, { max_chars: 300 });
    for (const c of chunks) {
      if (c.heading_path.includes('第二章 B')) {
        assert.ok(!c.text.includes('# 第一章 A'), `跨节混入：${c.text.slice(0,80)}`);
      }
    }
  });
});

describe('[T12-AC4] heading_path 层级正确', function () {
  const doc = [
    '# 一、平台',
    '## 1. 架构',
    '### 1.1 后端',
    '后端使用 Rust + Node.js',
    '### 1.2 前端',
    '前端使用 Vue3 + Vite',
    '## 2. 运维',
    '运维包括发布、回滚、灰度。',
  ].join('\n');
  it('后端块 path 至少 3 级', () => {
    const chunks = splitDocument(doc);
    const backend = chunks.find(c => c.text.includes('Rust'));
    assert.ok(backend, '找到后端相关 chunk');
    assert.ok(backend.heading_path.length >= 3, `backend path len ${backend.heading_path.length}: ${backend.heading_path}`);
    const last = backend.heading_path[backend.heading_path.length - 1];
    assert.ok(last.includes('后端'), `最后一级应包含"后端"，实际是：${last}`);
  });
  it('运维 path 深度：至少 2 级（["一、平台","2. 运维"]）', () => {
    const chunks = splitDocument(doc);
    const op = chunks.find(c => c.text.includes('灰度'));
    assert.ok(op);
    assert.ok(op.heading_path.length >= 2, `运维 path 应 ≥2 级：${op.heading_path}`);
  });
});

describe('[T12-AC5] 超长单节按 max_chars/overlap 滑窗切割（无 heading 漏网）', function () {
  const line = '这是一句普通中文测试文本，字数大约 30 字左右。';
  const paras = [];
  for (let i = 0; i < 40; i++) paras.push(line);
  const doc = `# 大章节\n\n${paras.join('\n')}`;
  const opts = { max_chars: 400, overlap: 50 };

  it('产出多块且所有块 heading_path[0]="大章节"，单块 chars ≤ max_chars+小余量', () => {
    // line 约 30 字，40 段 = 约 1200 字（含换行 1160）。# 大章节 + 换行 = 1200+ total
    // 设 max_chars=400，overlap=50 → 至少 3~4 块
    const chunks = splitDocument(doc, opts);
    assert.ok(chunks.length >= 3, `应切成 ≥3 块，实际 ${chunks.length}`);
    for (const c of chunks) {
      assert.strictEqual(c.heading_path[0], '大章节', '全部保持大章节根路径');
      // 允许 +25% 余量（heading 前缀 + 边界对齐）
      assert.ok(c.chars <= Math.round(opts.max_chars * 1.25) + 20,
        `单块超界：id=${c.id} chars=${c.chars} max=${opts.max_chars} 首行=${c.text.split('\n')[0]}`);
    }
  });
  it('overlap：相邻块应有重叠部分（避免滑窗断点遗漏句子）', () => {
    const chunks = splitDocument(doc, opts);
    const overlapMin = Math.max(5, opts.overlap - 5);
    let ok = 0;
    for (let i = 1; i < chunks.length; i++) {
      // 以 20 字符为单元，找 chunks[i-1] 尾部的一段在 chunks[i].text 中是否出现
      const tail = chunks[i - 1].text.slice(-overlapMin);
      if (tail && chunks[i].text.includes(tail)) ok++;
    }
    assert.ok(ok >= Math.floor((chunks.length - 1) * 0.5),
      `相邻重叠覆盖率应 ≥ 50%，实际 ${ok}/${chunks.length - 1}`);
  });
});

describe('[T12-AC6] 中文编号：第X章 + 第X节 正确出栈/压栈', function () {
  const doc = [
    '第一章 范围',
    '',
    '这里写第一章正文',
    '',
    '第二章 引用文件',
    '',
    '这里写第二章正文',
    '',
    '2.1 规范性引用',
    '',
    '国标、行标等',
  ].join('\n');
  it('存在两块 path 首项分别为 "范围" 和 "引用文件"', () => {
    const chunks = splitDocument(doc);
    const firsts = new Set(chunks.map(c => c.heading_path[0]).filter(Boolean));
    assert.ok(firsts.has('范围'), `应有 "范围"。实际首项集合：${[...firsts].join(' | ')}`);
    assert.ok(firsts.has('引用文件'), `应有 "引用文件"`);
  });
  it('存在一块 path 包含 "规范性引用"', () => {
    const chunks = splitDocument(doc);
    const ok = chunks.some(c => c.heading_path.includes('规范性引用'));
    assert.ok(ok, `应存在含 2.1 节点的 chunk。所有 paths：${chunks.map(c => c.heading_path.join('/')).join(' | ')}`);
  });
});

describe('[T12-AC7] summarizeChunks p95/p99 统计正确', function () {
  it('20 块 chars=100…2000 线性递增 → p99 接近最大值（≈1981）', () => {
    // arr[0]=100, arr[1]=200 ... arr[19]=2000. p99(0.99): pos=(19)*0.99=18.81
    // lo=18 (1900), hi=19 (2000), frac=0.81 → 1900*0.19 + 2000*0.81 = 1981
    const chunks = [];
    for (let i = 0; i < 20; i++) chunks.push({ chars: 100 + i * 100, text: '', heading_path: [], meta: {}, level: 0 });
    const s = summarizeChunks(chunks);
    assert.strictEqual(s.total_chunks, 20);
    assert.ok(Math.abs(s.p99_chars - 1981) <= 1, `p99 应约等于 1981，实际 ${s.p99_chars}`);
  });
  it('空 chunks 所有指标 = 0', () => {
    const s = summarizeChunks([]);
    assert.deepStrictEqual(s.total_chunks, 0);
    assert.deepStrictEqual(s.avg_chars, 0);
    assert.deepStrictEqual(s.p99_chars, 0);
    assert.deepStrictEqual(s.cross_heading_chunks, 0);
    assert.deepStrictEqual(s.distinct_headings, 0);
  });
});

describe('[T12-AC8] edge case：无标题纯文本', function () {
  it('仍可正确切块，无 crash；cross_heading_chunks=0', () => {
    // 300 行 × 3 字符（"a\n"）= ~600 chars。max=200 → 至少 3~4 块
    const doc = 'a\n'.repeat(300);
    const chunks = splitDocument(doc, { max_chars: 200 });
    assert.ok(chunks.length >= 3, `应产出足够块（≥3），实际 ${chunks.length}`);
    const s = summarizeChunks(chunks);
    assert.strictEqual(s.cross_heading_chunks, 0);
  });
  it('空字符串返回 []', () => {
    assert.deepStrictEqual(splitDocument(''), []);
  });
});

describe('[T12-AC9] prefer=length 定长降级与向后兼容', function () {
  const doc = '# A\n\n' + 'x '.repeat(2000);
  it('prefer=length 下所有块不超过 max_chars+小余量', () => {
    const chunks = splitDocument(doc, { max_chars: 300, prefer: 'length' });
    for (const c of chunks) {
      assert.ok(c.chars <= 320, `定长模式单块应 ≤320：chars=${c.chars}`);
    }
    assert.ok(chunks.length >= 10, `应切成足够定长块，实际 ${chunks.length}`);
  });
  it('默认 DEFAULT_MAX_CHARS = 800', () => {
    assert.strictEqual(DEFAULT_MAX_CHARS, 800);
  });
});
