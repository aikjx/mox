// mox 浏览器级验证 v3：官网管理中心（AI 中台）+ 配置台
const { chromium } = require('playwright');
const path = require('path');

const exe = 'C:\\Users\\mo\\AppData\\Local\\ms-playwright\\chromium-1228\\chrome-win64\\chrome.exe';
const website = 'file:///' + path.resolve(__dirname, '../frontend-ui/mox-website/index.html').replace(/\\/g, '/') + '#/admin';
const console_ = 'file:///' + path.resolve(__dirname, '../frontend-ui/mox-console/index.html').replace(/\\/g, '/');

(async () => {
  const browser = await chromium.launch({ headless: true, executablePath: exe, args: ['--headless=new', '--no-sandbox'] });
  let failed = 0;

  async function probe(page, url, name) {
    const errors = [];
    page.on('pageerror', e => errors.push('PAGEERROR: ' + e.message));
    page.on('console', m => { if (m.type() === 'error') errors.push('CONSOLE: ' + m.text()); });
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 10000 });
    await page.waitForTimeout(3000);
    const realErrors = errors.filter(e => !e.includes('favicon') && !e.includes('net::ERR_ABORTED'));
    if (realErrors.length) { failed++; console.log(`[${name}] JS ERRORS:`); realErrors.slice(0, 6).forEach(e => console.log('   - ' + e)); }
    else console.log(`[${name}] NO JS ERRORS`);
    return realErrors.length === 0;
  }

  // ---- 官网管理中心 AI 中台 ----
  {
    const page = await browser.newPage();
    const ok = await probe(page, website, '管理中心');
    if (ok) {
      const sig = await page.evaluate(() => ({
        hasAiPanel: !!document.querySelector('.ai-panel'),
        hasDashPanel: !!document.querySelector('.dash-panel'),
        aiHead: !!document.querySelector('.ai-head h2'),
        kpis: document.querySelectorAll('.dkpi').length,
        moreBtn: !!document.getElementById('moreBtn'),
        aiInput: !!document.getElementById('aiInput'),
        miniBars: document.querySelectorAll('.mini-bars .mb').length,
      }));
      console.log(`[管理中心] aiPanel=${sig.hasAiPanel} dashPanel=${sig.hasDashPanel} kpis=${sig.kpis} moreBtn=${sig.moreBtn} aiInput=${sig.aiInput} miniBars=${sig.miniBars}`);
      // 测试 AI 对话
      if (sig.aiInput) {
        await page.fill('#aiInput', '查询所有产品');
        await page.click('#aiInput + button');
        await page.waitForTimeout(2500);
        const ai = await page.evaluate(() => {
          const msgs = [...document.querySelectorAll('.ai-msg')];
          return { count: msgs.length, hasSql: msgs.some(m => m.textContent.includes('products')), hasRunBtn: !!document.querySelector('.ai-run-btn') };
        });
        console.log(`[管理中心] AI对话: msgs=${ai.count} hasSql=${ai.hasSql} runBtn=${ai.hasRunBtn}`);
      }
      // 测试更多弹框
      if (sig.moreBtn) {
        await page.click('#moreBtn');
        await page.waitForTimeout(500);
        const mp = await page.evaluate(() => ({
          open: document.getElementById('morePanel').classList.contains('open'),
          cards: document.querySelectorAll('.mp-card').length,
        }));
        console.log(`[管理中心] 更多弹框: open=${mp.open} cards=${mp.cards}`);
      }
    }
    await page.close();
  }

  // ---- 配置台 ----
  {
    const page = await browser.newPage();
    const ok = await probe(page, console_, '配置台');
    if (ok) {
      const sig = await page.evaluate(() => ({ kpis: document.querySelectorAll('.kpi').length, foot: document.getElementById('footState')?.textContent }));
      console.log(`[配置台] kpis=${sig.kpis} foot=${sig.foot}`);
    }
    await page.close();
  }

  await browser.close();
  console.log(failed ? `RESULT: ${failed} FAILED` : 'RESULT: BROWSER VERIFY PASSED');
  process.exit(failed ? 1 : 0);
})();
