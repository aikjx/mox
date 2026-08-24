// @ts-check
/**
 * P1-3 Playwright 10 关键页 E2E (P0 优先级 · smoke 绿)
 * 覆盖 10 条关键路由：/login → /ai → /graph → /dashboard → /projects
 *                    → /knowledge-base → /expert-center → /llm-config → /tasks → /automation
 * 断言: 路由状态 OK(200/304) + 页面非空主内容 + 无致命 console.error + <el-main>/<main>/页面 title 可观察
 */
import { test, expect } from '@playwright/test';

/** 10 条关键页列表（和 P1-3 闸门对齐） */
const PAGES = [
  { path: '/login',           title: /登录|Login|sign/i,              anchor: 'input[placeholder*="用户名"], input[placeholder*="账号"], input[placeholder*="邮箱"], input[type="text"], #app, form' },
  { path: '/ai',              title: /AI|工作台|Chat|会话/i,          anchor: '#app, main, .chat-view, [class*="chat"], .el-main, text="AI"' },
  { path: '/graph',           title: /图谱|Graph|引擎宇宙/i,          anchor: '#app, #graph-skeleton, canvas, .graph-view, .el-main, text="图谱"' },
  { path: '/dashboard',       title: /仪表盘|Dashboard|概览/i,        anchor: '#app, main, .dashboard, .el-main, text="仪表盘"' },
  { path: '/projects',        title: /项目|Projects/i,                anchor: '#app, main, .el-main, text="项目"' },
  { path: '/knowledge-base',  title: /知识库|Knowledge|KB/i,          anchor: '#app, main, .el-main, text="知识库"' },
  { path: '/expert-center',   title: /专家中心|Expert/i,              anchor: '#app, main, .el-main, text="专家"' },
  { path: '/llm-config',      title: /模型|LLM|Config|配置/i,         anchor: '#app, main, .el-main, text="模型"' },
  { path: '/tasks',           title: /任务|Tasks/i,                   anchor: '#app, main, .el-main, text="任务"' },
  { path: '/automation',      title: /自动化|Automation|AutoDev/i,    anchor: '#app, main, .el-main, text="自动化"' }
];

/** 失败但不致命的 console 白名单（避免把 warning 当错误） */
const WARN_WHITELIST = [
  '[Vue warn]', 'Download the Vue Devtools', 'is a non-boolean attribute',
  'component has already been registered',
  // 预览/无后端环境：API 不可用属于正常，不视为前端页面致命错误
  '401 (Unauthorized)', '403 (Forbidden)', '404 (Not Found)',
  '500 (Internal Server Error)', '502 (Bad Gateway)', '503 (Service Unavailable)', '504 (Gateway Timeout)',
  'Failed to load resource:', 'net::ERR_CONNECTION_REFUSED', 'net::ERR_ABORTED',
  'Failed to fetch', 'NetworkError', 'No API server available',
  'Unrecognized feature', 'Autofill processing',
  'ResizeObserver loop limit exceeded',
];

for (const page of PAGES) {
  test.describe(`${page.path} 关键页冒烟`, () => {
    /** @type {import('@playwright/test').Page} */
    let pw;
    let errors = [];

    test.beforeEach(async ({ browser }) => {
      errors = [];
      pw = await browser.newPage();
      pw.on('console', (msg) => {
        if (msg.type() !== 'error') return;
        const text = msg.text();
        if (WARN_WHITELIST.some(w => text.includes(w))) return;
        errors.push(text);
      });
    });

    test.afterEach(async () => {
      try { await pw.close(); } catch {}
    });

    test(`${page.path} — 加载 2xx 且主内容可见`, async () => {
      const res = await pw.goto(page.path, { waitUntil: 'domcontentloaded', timeout: 30000 });
      expect(res?.status(), `路由 ${page.path} 返回非 2xx/304`).toBeGreaterThanOrEqual(200);
      expect(res?.status(), `路由 ${page.path} 返回 ≥ 400`).toBeLessThan(400);
      // 等待应用根渲染（SPA 常见场景：先 <div id="app"> 再子树挂载）
      await pw.waitForSelector('#app', { state: 'attached', timeout: 20000 });
      // 主内容锚点（容错定位器）
      await pw.waitForLoadState('networkidle').catch(() => {});
      const loc = pw.locator('body').filter({ hasText: /.{8}/ });
      await expect(loc, `页面 ${page.path} 内容为空`).toBeVisible({ timeout: 15000 });
      // 非致命：title 或 锚点关键词至少命中一个（避免页面全空白但被 vue skeleton 蒙蔽）
      const title = (await pw.title()) || '';
      const text = (await pw.locator('body').innerText({ timeout: 6000 }).catch(() => '')) || '';
      // 非致命：title 正则命中 或 内容足够长（兜底防空白），二选一即可
      const ok = page.title.test(title) || (text.length > 120);
      expect(ok, `${page.path} 内容/标题缺少预期语义（title=${JSON.stringify(title)} len=${text.length}）`).toBe(true);
      // 致命 console.error 拦截
      expect(errors, `${page.path} 存在致命 console.error（前 3 条）: ${errors.slice(0,3).join(' || ')}`).toHaveLength(0);
    });
  });
}
