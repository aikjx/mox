// 快速验证：新闻详情页 + 产品详情页 + 案例详情页（mock 模式）
const { chromium } = require('playwright');
const path = require('path');
const exe = 'C:\\Users\\mo\\AppData\\Local\\ms-playwright\\chromium-1228\\chrome-win64\\chrome.exe';
const base = 'file:///' + path.resolve(__dirname, '../frontend-ui/mox-website/index.html').replace(/\\/g, '/');

(async () => {
  const browser = await chromium.launch({ headless: true, executablePath: exe, args: ['--headless=new', '--no-sandbox'] });
  let failed = 0;
  const checks = [
    { hash: '#/news-detail?id=1', expect: '墨行科技发布', name: '新闻详情 id=1' },
    { hash: '#/product-detail?id=2', expect: '墨行企业平台', name: '产品详情 id=2' },
    { hash: '#/case-detail?id=1', expect: '银行', name: '案例详情 id=1' },
    { hash: '#/news-detail?id=999', expect: '新闻不存在', name: '新闻详情 不存在' },
  ];
  for (const c of checks) {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.goto(base + c.hash, { waitUntil: 'domcontentloaded', timeout: 10000 });
    await page.waitForTimeout(1500);
    const text = await page.evaluate(() => document.querySelector('.view.active')?.innerText || document.body.innerText);
    const ok = text.includes(c.expect);
    const jsOk = errors.length === 0;
    console.log(`[${c.name}] expect="${c.expect}" found=${ok} jsErrors=${errors.length}`);
    if (!ok || !jsOk) { failed++; if(errors.length) console.log('  JS ERR:', errors[0]); }
    await page.close();
  }
  await browser.close();
  console.log(failed ? `RESULT: ${failed} FAILED` : 'RESULT: ALL PASSED');
  process.exit(failed ? 1 : 0);
})();
