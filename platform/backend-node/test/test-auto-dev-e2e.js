'use strict';

/**
 * 自动开发引擎端到端测试
 * 链路：POST /ai/engine/auto-dev（需求→架构图谱→代码渲染→落盘）
 *      → GET /projects（列表）→ GET /preview/...（静态预览）
 * 运行：node test/test-auto-dev-e2e.js
 */

const http = require('http');

const BASE = { host: '127.0.0.1', port: 3010 };

function request(method, path, body) {
  return new Promise((resolve, reject) => {
    const payload = body ? JSON.stringify(body) : null;
    const req = http.request(
      {
        ...BASE,
        method,
        path,
        headers: payload
          ? { 'Content-Type': 'application/json; charset=utf-8', 'Content-Length': Buffer.byteLength(payload) }
          : {}
      },
      (res) => {
        const chunks = [];
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => {
          const text = Buffer.concat(chunks).toString('utf8');
          let json = null;
          try {
            json = JSON.parse(text);
          } catch (e) {
            /* 非 JSON 响应（如预览 HTML） */
          }
          resolve({ status: res.statusCode, headers: res.headers, text, json });
        });
      }
    );
    req.on('error', reject);
    req.setTimeout(300000, () => req.destroy(new Error('请求超时（300s）')));
    if (payload) req.write(payload);
    req.end();
  });
}

function assert(cond, msg) {
  if (!cond) throw new Error('断言失败: ' + msg);
  console.log('  [PASS] ' + msg);
}

(async () => {
  console.log('===== 自动开发引擎 端到端实测 =====\n');
  const results = { passed: 0, failed: 0 };

  // ① 前置检查：服务在线
  console.log('[1] 服务健康检查');
  const health = await request('GET', '/ai/engine/capabilities');
  assert(health.status === 200, 'AI 引擎服务在线（GET /ai/engine/capabilities → 200）');

  // ② 一句话需求 → 全自动开发
  console.log('\n[2] 端到端开发：需求「开发一个企业官网」');
  const t0 = Date.now();
  const dev = await request('POST', '/ai/engine/auto-dev', {
    requirement: '开发一个企业官网',
    project_name: 'corp-site',
    overwrite: true
  });
  const elapsed = Date.now() - t0;
  assert(dev.status === 200, `开发请求成功（HTTP ${dev.status}，耗时 ${elapsed}ms）`);
  const result = dev.json && dev.json.data ? dev.json.data : dev.json;
  assert(result && result.project, '返回项目标识: ' + (result && result.project));
  assert(Array.isArray(result.files) && result.files.length > 0, `落盘文件数: ${result.files.length}`);
  assert(result.pipeline && result.pipeline.length === 5, `流水线五阶段完整: ${result.pipeline.join(' → ')}`);
  assert(result.architecture && result.architecture.pages.length > 0, `架构图谱页面数: ${result.architecture.pages.length}`);
  assert(result.preview_url_abs, '预览地址: ' + result.preview_url_abs);
  console.log('  [INFO] 站点名: ' + result.site_name);
  console.log('  [INFO] 页面: ' + result.architecture.pages.map((p) => p.id).join(', '));
  const navLabels = result.architecture.nav.map((n) => (typeof n === 'string' ? n : n.label || n.title || n.id || JSON.stringify(n)));
  console.log('  [INFO] 导航: ' + navLabels.join(' | '));
  const entLabels = result.architecture.entities.map((e) => (typeof e === 'string' ? e : e.name || e.id || JSON.stringify(e)));
  console.log('  [INFO] 实体: ' + entLabels.join(', '));
  console.log('  [INFO] 文件: ' + result.files.map((f) => f.filename).join(', '));
  if (result.skipped && result.skipped.length) {
    console.log('  [WARN] 跳过文件: ' + JSON.stringify(result.skipped));
  }
  results.passed += 6;

  // ③ 项目列表
  console.log('\n[3] 项目列表');
  const list = await request('GET', '/ai/engine/auto-dev/projects');
  assert(list.status === 200, '项目列表接口正常');
  const data = list.json && list.json.data ? list.json.data : list.json;
  const projects = data.projects || data;
  const found = Array.isArray(projects) && projects.some((p) => p.project === result.project || p.name === result.project);
  assert(found, `列表中包含项目 ${result.project}（共 ${Array.isArray(projects) ? projects.length : 0} 个，文件数 ${projects[0] ? projects[0].files : '?'}）`);
  results.passed += 2;

  // ④ 静态预览（HTML）
  console.log('\n[4] 在线预览验证');
  const preview = await request('GET', result.preview_url);
  assert(preview.status === 200, '预览页面可访问（HTTP 200）');
  assert((preview.headers['content-type'] || '').includes('text/html'), 'Content-Type: text/html');
  assert(preview.text.includes('<!DOCTYPE html') || preview.text.includes('<html'), '预览内容为合法 HTML');
  assert(preview.text.length > 500, `预览页面大小: ${preview.text.length} 字节`);
  assert(/corp|企业|官网|site/i.test(preview.text), '预览内容与需求主题匹配');
  assert(preview.headers['x-content-type-options'] === 'nosniff', '安全头 X-Content-Type-Options: nosniff 存在');
  results.passed += 6;

  // ⑤ CSS/JS 资源可访问（若存在）
  console.log('\n[5] 静态资源可访问性');
  const cssMatch = preview.text.match(/href="([^"]+\.css)"/);
  if (cssMatch) {
    const cssPath = '/ai/engine/auto-dev/preview/' + result.project + '/' + cssMatch[1].replace(/^\.\//, '');
    const css = await request('GET', cssPath);
    assert(css.status === 200 && css.text.length > 0, `样式表可访问: ${cssMatch[1]}（${css.text.length} 字节）`);
    results.passed += 1;
  } else {
    console.log('  [SKIP] 页面未引用外部 CSS（内联样式）');
  }

  // ⑥ 路径安全（逃逸防护）
  console.log('\n[6] 安全闸门验证');
  const evil = await request('GET', '/ai/engine/auto-dev/preview/../../api-server.js/x');
  assert(evil.status === 404, '路径逃逸被拒绝（' + evil.status + '）');
  const evil2 = await request('GET', '/ai/engine/auto-dev/preview/site-corp-site/..%2F..%2Fpackage.json');
  assert(evil2.status === 404, '编码逃逸被拒绝（' + evil2.status + '）');
  results.passed += 2;

  // ⑦ 汇总
  console.log('\n===== 实测汇总 =====');
  console.log(`通过: ${results.passed} 项，失败: ${results.failed} 项`);
  console.log('端到端开发耗时: ' + elapsed + 'ms');
  console.log('在线预览: ' + result.preview_url_abs);
  process.exit(results.failed > 0 ? 1 : 0);
})().catch((e) => {
  console.error('\n[FAIL] ' + e.message);
  process.exit(1);
});
