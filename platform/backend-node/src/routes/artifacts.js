'use strict';
const path = require('path');
const crypto = require('crypto');
const fs = require('fs');

/**
 * 路由域：本地制品
 *  - 旧：/ai/artifact/* 文档与代码制品自动创建
 *  - 新增（D2-OPS 游戏/网站 制品发布管线）：/artifacts REST 接口
 *      GET /artifacts?kind=... 列表
 *      GET /artifacts/:id 详情
 *      GET /artifacts/:id/file 下载源文件
 *      POST /artifacts 创建/发布（按 title 幂等）
 *      GET /artifacts/html/:filename 直接渲染发布的静态 HTML（供浏览器打开真实游戏页）
 */
module.exports = function registerArtifactsRoutes(ctx) {
  // ctx 未必包含 fs/path 顶层字段，回退到 Node 原生 require
  const { artifactService, ok, fail, readBody, appendLog, reg, uid, readJSON, writeJSON } = ctx;
  const DATA_DIR = (ctx && ctx.DATA_DIR) || path.resolve(__dirname, '..', '..', 'data');
  const ARTIFACTS_DIR = path.join(DATA_DIR, 'artifacts');
  try { fs.mkdirSync(ARTIFACTS_DIR, { recursive: true }); } catch (_) { /* ignore */ }
  const STORE_FILE = path.join(DATA_DIR, 'artifacts.json');
  const uidOf = (s) => crypto.createHash('sha1').update(String(s || '')).digest('hex').slice(0, 12);

  function readStore() { return readJSON('artifacts.json', []); }
  function writeStore(arr) { writeJSON('artifacts.json', arr); }
  function seedOnce() {
    // 启动时一次性落地默认模板（T5 游戏井字棋），保证 artifacts?kind=game 总有 ≥1 返回
    try {
      const tpl = path.join(ARTIFACTS_DIR, 'tictactoe.html');
      if (!fs.existsSync(tpl)) {
        const html = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<title>井字棋 · Xuanji T5 Game</title>
<style>
  * { box-sizing: border-box; }
  body { font-family: system-ui, "PingFang SC", sans-serif; margin: 0; padding: 2em; text-align: center; background: linear-gradient(135deg,#1e293b,#334155); color: #f1f5f9; min-height: 100vh;}
  h1 { margin: 0 0 0.2em; font-size: 28px; }
  .sub { opacity: .8; margin-bottom: 1em; }
  section { width: 360px; margin: 0 auto; padding: 16px; background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.1); border-radius: 12px; backdrop-filter: blur(4px); }
  table { margin: 1em auto; border-collapse: collapse; }
  td { width: 80px; height: 80px; border: 2px solid rgba(255,255,255,0.3); font-size: 36px; font-weight: bold; cursor: pointer; text-align: center; vertical-align: middle; user-select: none; transition: background .15s; }
  td:hover { background: rgba(255,255,255,0.08); }
  td.x { color: #38bdf8; }
  td.o { color: #f87171; }
  #controls { margin: 12px 0; }
  button { padding: 8px 16px; font-size: 14px; border-radius: 8px; border: 0; background: #0ea5e9; color: white; cursor: pointer; font-weight: 600; }
  button:hover { background: #0284c7; }
  .status { margin: 10px 0; font-size: 18px; min-height: 28px; }
  .win { color: #4ade80; }
  label { margin: 0 8px; opacity: .9; cursor: pointer; }
</style>
</head>
<body>
  <h1>🎮 井字棋</h1>
  <div class="sub">璇玑 AIS 平台 · T5 游戏生成 参考落地实现（纯前端可玩）</div>
  <section>
    <div id="controls">
      <button id="reset">🔄 重新开始</button>
      <label>先手 <input type="radio" name="first" value="X" checked> X</label>
      <label><input type="radio" name="first" value="O"> O</label>
    </div>
    <table id="b" aria-label="井字棋棋盘"></table>
    <div id="s" class="status">X 先手</div>
  </section>
<script>
  const board = document.getElementById('b');
  const statusEl = document.getElementById('s');
  let turn = 'X', grid = Array(9).fill(''), gameOver = false;
  const WIN = [[0,1,2],[3,4,5],[6,7,8],[0,3,6],[1,4,7],[2,5,8],[0,4,8],[2,4,6]];
  function render() {
    [...board.children].forEach((tr, i) => [...tr.children].forEach((td, j) => {
      const idx = i*3 + j;
      td.textContent = grid[idx] || '';
      td.classList.remove('x','o');
      if (grid[idx] === 'X') td.classList.add('x');
      if (grid[idx] === 'O') td.classList.add('o');
    }));
  }
  function checkWin() {
    for (const [a,b,c] of WIN) {
      if (grid[a] && grid[a] === grid[b] && grid[b] === grid[c]) return { winner: grid[a], line: [a,b,c] };
    }
    return null;
  }
  for (let i = 0; i < 9; i++) {
    if (i % 3 === 0) board.appendChild(document.createElement('tr'));
    const td = document.createElement('td');
    td.dataset.i = i;
    td.onclick = function (e) {
      const idx = +e.currentTarget.dataset.i;
      if (gameOver || grid[idx]) return;
      grid[idx] = turn;
      render();
      const w = checkWin();
      if (w) {
        gameOver = true;
        statusEl.textContent = '🎉 ' + w.winner + ' 获胜！';
        statusEl.classList.add('win');
        return;
      }
      if (grid.every(v => v)) { gameOver = true; statusEl.textContent = '🤝 平局'; return; }
      turn = turn === 'X' ? 'O' : 'X';
      statusEl.classList.remove('win');
      statusEl.textContent = turn + ' 走';
    };
    board.lastElementChild.appendChild(td);
  }
  document.getElementById('reset').onclick = function () {
    const first = document.querySelector('input[name=\"first\"]:checked').value;
    turn = first; grid = Array(9).fill(''); gameOver = false;
    statusEl.classList.remove('win'); statusEl.textContent = turn + ' 先手';
    render();
  };
  render();
<\/script>
</body>
</html>`;
        fs.writeFileSync(tpl, html, 'utf8');
      }
      // 登记到 artifacts 元数据（按 title 幂等）
      const store = readStore();
      const exists = store.find(a => a.title === '井字棋 · Xuanji T5 Game');
      if (!exists) {
        const htmlContent = fs.readFileSync(tpl, 'utf8');
        store.push({
          id: 't5-default-tictactoe-' + uidOf('tictactoe|t5|default'),
          kind: 'game',
          title: '井字棋 · Xuanji T5 Game',
          author: 'Xuanji Platform',
          tags: ['game', 'tictactoe', 't5-default', 'zero-dependency'],
          description: '纯前端井字棋，可直接在浏览器中打开 /artifacts/html/tictactoe.html 游玩（T5 游戏生成参考落地实现）。',
          url: '/artifacts/html/tictactoe.html',
          htmlSource: htmlContent,
          sourcePath: path.relative(DATA_DIR, tpl).replace(/\\/g, '/'),
          size: htmlContent.length,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        });
        writeStore(store);
      }
    } catch (e) { /* ignore */ }
  }
  seedOnce();

  // ==================== 本地制品引擎（文档/代码自动创建） ====================
  reg('get', '/ai/artifact/config', async (req, res) => {
    ok(res, artifactService.getConfig());
  });

  reg('get', '/ai/artifact/list', async (req, res) => {
    ok(res, artifactService.listArtifacts());
  });

  reg('post', '/ai/artifact/create', async (req, res) => {
    const body = await readBody(req);
    if (!body.message || !String(body.message).trim()) {
      fail(res, 400, '缺少 message 参数');
      return;
    }
    if (body.artifact_mode !== 'document' && body.artifact_mode !== 'code') {
      fail(res, 400, 'artifact_mode 必须为 document 或 code');
      return;
    }
    try {
      const result = await artifactService.process({
        mode: body.artifact_mode,
        message: body.message,
        session_id: body.session_id || body.sessionId || null,
        overwrite: !!body.overwrite
      });
      appendLog({
        type: 'artifact',
        msg: 'create',
        mode: body.artifact_mode,
        created: result.created.length,
        skipped: result.skipped.length
      });
      ok(res, result);
    } catch (e) {
      fail(res, 500, '制品创建失败: ' + e.message);
    }
  });

  // ==================== D2-OPS RESTful 制品发布管线 ====================
  reg('get', '/artifacts', (req, res) => {
    const u = new URL(req.url, 'http://localhost');
    const kind = u.searchParams.get('kind') || u.searchParams.get('type');
    const author = u.searchParams.get('author');
    let arr = readStore();
    if (kind) arr = arr.filter(a => a && a.kind === kind);
    if (author) arr = arr.filter(a => a && String(a.author || '').includes(author));
    ok(res, arr, { total: arr.length, kind: kind || null, generatedAt: new Date().toISOString() });
  });

  reg('get', '/artifacts/:id', (req, res) => {
    const id = req.params && req.params.id ? req.params.id : decodeURIComponent(req.url.split('/artifacts/')[1] || '').split('?')[0];
    const arr = readStore();
    const a = arr.find(x => x && x.id === id);
    if (!a) return fail(res, 404, 'artifact id=' + id + ' 不存在');
    ok(res, a);
  });

  reg('get', '/artifacts/html/:filename', (req, res) => {
    const raw = decodeURIComponent(req.url.split('/artifacts/html/')[1] || '').split('?')[0];
    const filename = raw.replace(/\.\./g, '').replace(/^[\/\\]+/, '');
    const fp = path.join(ARTIFACTS_DIR, filename);
    try {
      if (!fs.existsSync(fp)) return fail(res, 404, 'HTML file not found: ' + filename);
      const stat = fs.statSync(fp);
      const src = fs.readFileSync(fp, 'utf8');
      const hdrs = { 'Content-Type': 'text/html; charset=utf-8', 'Content-Length': Buffer.byteLength(src), 'Last-Modified': stat.mtime.toUTCString() };
      res.writeHead(200, hdrs); res.end(src);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('get', '/artifacts/:id/file', (req, res) => {
    const id = decodeURIComponent(req.url.split('/artifacts/')[1] || '').replace(/^([^/?]+)\/file.*$/, '$1');
    const arr = readStore();
    const a = arr.find(x => x && x.id === id);
    if (!a) return fail(res, 404, 'artifact id=' + id + ' 不存在');
    // HTML 内容直接发（支持下载/内联渲染双用途）；sourcePath 磁盘文件优先
    let src = null, filename = (a.title || a.id) + '.html';
    if (a.sourcePath) {
      const fp = path.isAbsolute(a.sourcePath) ? a.sourcePath : path.join(DATA_DIR, a.sourcePath);
      if (fs.existsSync(fp)) { try { src = fs.readFileSync(fp, 'utf8'); filename = path.basename(fp); } catch (_) {} }
    }
    if (!src && typeof a.htmlSource === 'string') src = a.htmlSource;
    if (!src) return fail(res, 404, 'artifact has no file content');
    const hdrs = { 'Content-Type': 'text/html; charset=utf-8', 'Content-Length': Buffer.byteLength(src), 'Content-Disposition': `inline; filename=\"${filename}\"` };
    res.writeHead(200, hdrs); res.end(src);
  });

  reg('post', '/artifacts', async (req, res) => {
    const body = await readBody(req);
    if (!body || typeof body !== 'object') return fail(res, 400, '需要 JSON 负载');
    const title = (body.title || '').toString().trim();
    const kind = (body.kind || 'artifact').toString().trim();
    if (!title) return fail(res, 400, '必须提供 title');
    const allowedKinds = new Set(['game','site','dashboard','document','code','report','service','flow','artifact']);
    if (!allowedKinds.has(kind)) return fail(res, 400, 'kind 必须是: ' + Array.from(allowedKinds).join('/'));
    const arr = readStore();
    const existing = arr.find(a => a && a.title === title && a.kind === kind);
    if (existing) {
      appendLog({ type: 'artifact', msg: 'publish-dup', id: existing.id, kind });
      return ok(res, existing, { created: false, duplicated: true, note: '相同 title + kind 已存在，返回既有记录（幂等）' });
    }
    // 落地 HTML 文件（若 htmlSource 存在），保证 /artifacts/html/:filename 可访问
    let sourcePath = null, url = null;
    const slug = title.replace(/[^\w\u4e00-\u9fa5-]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 60) || ('art-' + uidOf(title + '|' + Date.now()));
    const filename = `${slug}.html`;
    if (typeof body.htmlSource === 'string' && body.htmlSource.length > 100) {
      const fp = path.join(ARTIFACTS_DIR, filename);
      try {
        fs.writeFileSync(fp, body.htmlSource, 'utf8');
        sourcePath = 'artifacts/' + filename;
        url = '/artifacts/html/' + filename;
      } catch (e) { return fail(res, 500, '写入 HTML 失败: ' + e.message); }
    } else if (typeof body.source === 'string' && body.source.length > 100) {
      const fp = path.join(ARTIFACTS_DIR, filename);
      try {
        fs.writeFileSync(fp, body.source, 'utf8');
        sourcePath = 'artifacts/' + filename;
        url = '/artifacts/html/' + filename;
      } catch (e) { return fail(res, 500, '写入 HTML 失败: ' + e.message); }
    }
    const id = kind + '-' + slug + '-' + uidOf(title + '|' + kind + '|' + Date.now());
    const entry = {
      id, kind, title,
      author: body.author ? String(body.author).slice(0, 120) : 'anonymous',
      tags: Array.isArray(body.tags) ? body.tags.filter(x => typeof x === 'string').slice(0, 20) : [],
      description: body.description ? String(body.description).slice(0, 800) : null,
      url,
      htmlSource: typeof body.htmlSource === 'string' ? body.htmlSource.slice(0, 2_000_000) : (body.source || null),
      sourcePath,
      size: (typeof body.htmlSource === 'string' ? body.htmlSource.length : (body.source ? body.source.length : 0)),
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
    arr.push(entry);
    writeStore(arr);
    appendLog({ type: 'artifact', msg: 'publish-new', id, kind, size: entry.size });
    ok(res, entry, { created: true, duplicated: false });
  });
};
