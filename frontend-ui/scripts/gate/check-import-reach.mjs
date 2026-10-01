#!/usr/bin/env node
/**
 * 入口可达性账 —— 从 main.js 沿 import 边走不完的文件即「孤儿」，为退役决策出证据。
 *
 * 为什么要有这本账：`check-view-hex.py` 按裸 hex 数量排存量大户，但排在最前面的
 * `MessageBubble.vue`（156 处）实际只有 Storybook 的 story 在 import 它 —— 给它换令牌
 * 是给死码抛光。同理 `docs/` 里点名要退役的旧页面到底还挂不挂在路由上，此前没人量过。
 *
 * 判据：roots = src/main.js + index.html 的 <script src>。沿 import/export-from/动态
 * import('字面量')/side-effect import 走闭包。走不到的 src/{vue,js,css} 即孤儿。
 * 排除项（不是"干净"，是"按定义不参与运行时"）：*.test.js、*.stories.js、__mocks__/。
 *
 * 已知边界（宁可多报也不静默漏）：
 *   - `import.meta.glob` 会把"没被任何 import 提到的文件"变成运行时可达 ⇒ 一旦发现就
 *     以 UNRESOLVED 记名并让账目作废（verdict 降级为 UNKNOWN），不假装走完了。
 *   - 模板里的组件名由 unplugin 自动解析、字符串拼的动态 import 都抓不到 ⇒ 只认字面量，
 *     非字面量的 `import(变量)` 同样记 UNRESOLVED。
 *
 * 用法：node scripts/gate/check-import-reach.mjs [--json] [--selftest]
 *   默认只报告不改红：这本账的第一版是决策证据，不是门禁。
 */
import fs from 'node:fs'
import path from 'node:path'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const SRC = path.join(ROOT, 'src')
const EXTS = ['.js', '.vue', '.ts', '.css']
// 动态 import 调用的前缀（含"魔法注释"：webpackChunkName / @vite-ignore 都是单行注释）。
// 注释体两条约束缺一不可：① 不许跨行（[^\n]）—— 跨行配对会把 /*a 与两千字符外的 b 结束符连成一对，
// 于是从 import( 一路吞过真代码（实测单次命中 4,600 字符）；② 体内不许出现注释结束符（(?!...) 拦下）——
// 只约束跨行仍不够：懒惰体会被回溯撑到**本行**最后一个结束符，把 /* a */ 与 /* b */ 之间的 pick(...) 当注释吃掉，
// 于是 'x' 被当成 import 的说明符收进账（实测 len=18、下一字符 p）。懒惰不等于最短匹配成功。
const DYN_HEAD = String.raw`\bimport\s*\(\s*(?:\/\*(?:(?!\*\/)[^\n])*\*\/\s*)*`
export const DYN_IMPORT_RE = new RegExp(DYN_HEAD + String.raw`['"]([^'"]+)['"]\s*\)`, 'g')
export const isNonLiteralDynamic = (text) => new RegExp(DYN_HEAD + String.raw`[^'"\s/)]`).test(text)

// 跨行具名 import 是一条独立的边，漏了就把活码判成孤儿（与下面 export 那条不对称 = 曾经的 bug）：
// `import {\n  a, b\n} from '@/utils/hitl-ws'` 在单行那条里必然失配（[^'"\n] 不许跨行），
// 于是 views/admin/panels/AdminHitl.vue（活）对 utils/hitl-ws.js 的引用整条消失 ⇒ 假死码。
// 两个约束缺一不可：
// ① 行锚 (?:^|\n)\s* 必须保留。第一版图省事只写 import\s+... ⇒ 文档注释里的示例代码
//    `composables/useTheme.js:10  *   import { useAppStore } from '@/stores'` 被当成真导入，
//    stores/index.js 连带它的两条 re-export（project.store/user.store）全部被判活 ⇒ **假活码**，
//    和它要修的假死码一样危险（后果从"没人用"变成"别删"）。变异体：删掉行锚 ⇒ 注释形态钉按名报红。
// ② 绑定列表用 [^{}] 而不是 [\s\S]*? / .[^{}]：这个类能跨行（真形态需要）又不会跑过自己的闭合括号。
//    诚实性声明：这一条在本仓库上**没有钉替它说话** —— 变异体 [^{}]* → [\s\S]*? 实测零红、
//    universe/seen/orphans 三个数全同（2026-09-25），因为仓库里不存在能区分两者的形态。
//    留着它是构造选择（少一种越界可能），不要把它当成"已被门禁保护"。
export const NAMED_IMPORT_RE = /(?:^|\n)\s*import\s+(?:type\s+)?(?:[\w$]+\s*,\s*)?\{[^{}]*\}\s*from\s*['"]([^'"]+)['"]/g

const SPECS = [
  /(?:^|\n)\s*import\s+[^'"\n]*?from\s*['"]([^'"]+)['"]/g,
  NAMED_IMPORT_RE,
  /(?:^|\n)\s*import\s*['"]([^'"]+)['"]/g,
  /(?:^|\n)\s*export\s+(?:\*|\{[^}]*\})\s+from\s*['"]([^'"]+)['"]/g,
  DYN_IMPORT_RE,
  /require\s*\(\s*['"]([^'"]+)['"]\s*\)/g
]
// CSS @import 是一条独立的边，漏了就会把活码判成孤儿：styles/themes/index.css（由 main.js
// 引入）用 @import 挂了 theme-dark/sky/cyberpunk 三份。判据少一种边形 = 假死码，比反向危险。
const CSS_SPECS = [
  /@import\s+(?:url\(\s*)?['"]([^'"]+?)['"]/g,
  /@import\s+url\(\s*([^'")\s]+?)\s*\)/g
]

const isExcluded = (rel) => /\.test\.js$/.test(rel) || /\.stories\.(js|mdx)$/.test(rel)
  || rel.includes('__mocks__') || rel.includes('node_modules')

// 全部源码文件（账的分母）
export function listUniverse(srcDir) {
  const out = []
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const full = path.join(d, e.name)
      if (e.isDirectory()) walk(full)
      else if (EXTS.includes(path.extname(e.name))) out.push(path.relative(srcDir, full).replace(/\\/g, '/'))
    }
  }
  walk(srcDir)
  return out.filter((r) => !isExcluded(r)).sort()
}

// 解析裸/别名/相对说明符；解析不到即 null（第三方包按 null 跳过）
export function resolveSpecifier(spec, fromRel, srcDir) {
  if (spec.startsWith('@/')) return pickExact(path.join(srcDir, spec.slice(2)), srcDir, spec)
  if (!spec.startsWith('./') && !spec.startsWith('../') && !spec.startsWith('/')) return null
  const base = spec.startsWith('/')
    ? path.join(srcDir, spec)
    : path.resolve(srcDir, path.dirname(fromRel), spec)
  return pickExact(base, srcDir, spec)
}

function pickExact(base, srcDir, spec) {
  const clean = base.replace(/\.js$/, '')
  const cands = spec.endsWith('.css') || spec.endsWith('.vue')
    ? [base]
    : EXTS.map((e) => clean + e)
      .concat(EXTS.map((e) => path.join(clean, 'index' + e)))
      .concat([base])
  for (const c of cands) {
    if (fs.existsSync(c) && fs.statSync(c).isFile()) {
      return path.relative(srcDir, c).replace(/\\/g, '/')
    }
  }
  return null
}

export function reachable(roots, srcDir) {
  const seen = new Set()
  const unresolved = []
  const queue = roots.map((r) => path.relative(srcDir, r).replace(/\\/g, '/')).filter((r) => !r.startsWith('..'))
  while (queue.length) {
    const rel = queue.shift()
    if (seen.has(rel) || isExcluded(rel)) continue
    const abs = path.join(srcDir, rel)
    if (!fs.existsSync(abs) || !fs.statSync(abs).isFile()) continue
    seen.add(rel)
    const text = fs.readFileSync(abs, 'utf8')
    if (text.includes('import.meta.glob')) unresolved.push(`${rel}: import.meta.glob`)
    if (isNonLiteralDynamic(text)) unresolved.push(`${rel}: 非字面量动态 import`)
    for (const re of SPECS.concat(CSS_SPECS)) {
      re.lastIndex = 0
      let m
      while ((m = re.exec(text))) {
        const spec = m[1]
        if (spec.includes('${') || spec.includes('+')) { unresolved.push(`${rel}: 拼接说明符 ${spec}`); continue }
        const r = resolveSpecifier(spec, rel, srcDir)
        if (r && !r.startsWith('..')) queue.push(r)
      }
    }
  }
  return { seen, unresolved }
}

export function analyze(srcDir) {
  const universe = listUniverse(srcDir)
  const roots = [path.join(srcDir, 'main.js')]
  const html = path.join(ROOT, 'index.html')
  if (fs.existsSync(html)) {
    const h = fs.readFileSync(html, 'utf8')
    const re = /<script[^>]+src=["']([^"']+)["']/g
    let m
    while ((m = re.exec(h))) {
      const p = m[1].replace(/^\//, '').replace(/^src\//, '')
      const abs = path.join(ROOT, p.startsWith('src/') ? p : path.join('src', p))
      if (fs.existsSync(abs)) roots.push(abs)
    }
  }
  const { seen, unresolved } = reachable(roots, srcDir)
  const orphans = universe.filter((r) => !seen.has(r))
  // 账外条目：seen 里出现了不属于分母的东西（如被 @import url() 拉进来的图片）
  const inUniv = new Set(universe)
  const offLedger = [...seen].filter((r) => !inUniv.has(r)).sort()
  const hexOf = (rel) => {
    // 与 check-view-hex.py 同口径：先剥 /* */ 注释，否则本工具的全集 hex 会比门禁的多出注释里的色值
    const t = fs.readFileSync(path.join(srcDir, rel), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '')
    return (t.match(/#[0-9a-fA-F]{3,8}\b/g) || []).length
  }
  const linesOf = (rel) => fs.readFileSync(path.join(srcDir, rel), 'utf8').split(/\r?\n/).length
  return { universe, seen, orphans, offLedger, unresolved, hexOf, linesOf }
}

function selftest() {
  const fails = []
  const a = analyze(SRC)
  const has = (rel) => a.seen.has(rel)
  // 正向钉：入口链必须走到（说明符形态各一种：相对 './'、别名 '@/'、动态 import）
  if (!has('App.vue')) fails.push('App.vue 判不可达 ⇒ 走边逻辑坏了（./App.vue）')
  if (!has('router/index.js')) fails.push('router/index.js 判不可达 ⇒ 别名/相对解析坏了')
  if (!has('views/expert/ExpertCenterView.vue')) fails.push('ExpertCenterView 判不可达 ⇒ 动态 import 边没走')
  if (!has('modules/expert-alliance/index.js')) fails.push('模块入口判不可达 ⇒ 模块树没进账')
  // CSS @import 边各钉一份：index.css 由 main.js 引入，其 @import 的三份必须同侧可达。
  // 变异体：把 SPECS.concat(CSS_SPECS) 改回 SPECS ⇒ 这三条按名报红（曾实测假判孤儿 207 处 hex）。
  for (const f of ['theme-dark.css', 'theme-sky.css', 'theme-cyberpunk.css']) {
    if (!has(`styles/themes/${f}`)) fails.push(`${f} 判不可达 ⇒ 漏了 CSS @import 这条边（唯一入口是 styles/themes/index.css）`)
  }
  // 反向钉：已核实只有 story 引用的组件必须留在孤儿账上
  if (has('components/MessageBubble.vue')) fails.push('MessageBubble 被判可达 ⇒ 假阳性（它只被 .stories.js 引用）')
  // 跨行具名 import 这条边的正反两钉（2026-09-25 实测：漏边 ⇒ utils/hitl-ws.js 假孤儿；漏行锚 ⇒
  // stores/index.js + project.store + user.store 三个假活码，两种病各由一条钉指名打破）：
  // 变异体 A：SPECS 里删掉 NAMED_IMPORT_RE ⇒ 第一条报红；变异体 B：把 (?:^|\n)\s* 改成 \b ⇒ 第二条报红。
  if (!has('utils/hitl-ws.js')) fails.push('utils/hitl-ws.js 判不可达 ⇒ 漏了跨行具名 import 这条边（唯一引用者 AdminHitl.vue:120-123：import { 换行 … } from）')
  if (has('stores/index.js')) fails.push('stores/index.js 被判可达 ⇒ 文档注释里的示例 import 被当成真边（composables/useTheme.js:10）⇒ 行锚被删，这本账会开始发"别删"的假指令')
  const capNamed = (s) => [...s.matchAll(NAMED_IMPORT_RE)].map((m) => m[1])
  const MULTI = "setup()\nimport {\n  hitlClient, HITL_ACTIONS\n} from '@/utils/hitl-ws'\nconst x = 1"
  const PROSE = "/**\n * 推荐新代码直接使用：\n *   import { useAppStore } from '@/stores'\n */\nconst y = 2"
  const gotMulti = capNamed(MULTI)
  if (gotMulti.length !== 1 || gotMulti[0] !== '@/utils/hitl-ws') fails.push(`跨行具名 import 没被抓到（捕获=${JSON.stringify(gotMulti)}）⇒ NAMED_IMPORT_RE 的跨行能力哑了，活码会被判孤儿`)
  const gotProse = capNamed(PROSE)
  if (gotProse.length !== 0) fails.push(`注释里的示例 import 被抓成了边（捕获=${JSON.stringify(gotProse)}）⇒ 行锚丢了，散文会变成第二条依赖边源`)
  // UNRESOLVED 必须为空：一旦引入就走不完闭包，该文件的账只能记 UNKNOWN。
  if (a.unresolved.length) fails.push(`UNRESOLVED ${a.unresolved.length} 处 ⇒ 账作废：${a.unresolved.join(' ; ')}`)
  // 动态 import 说明符形态的四向钉（M-K 的靶子改在这里，因为账本走不出裸包名、看不见这条边）：
  // 变异体 A：删掉 DYN_HEAD 的魔法注释段 ⇒ MAGIC 那两条报红（捕获为空 + 探测器把包名首字符当变量）。
  // 变异体 B：注释体改回跨行通配 [^\n]*? 或 [\s\S]*? ⇒ CROSSED 那条报红（本行/跨行的后一个结束符被配对，
  //          两个字面量之间的真代码当成注释吞掉，捕获到 'x'）。这两个形态各测一次，缺一漏一种病。
  const MAGIC = "x = import(\n  /* webpackChunkName: \"pkg\" */\n  /* @vite-ignore */\n  '3d-force-graph'\n).then(y)"
  const CROSSED = "x = import(\n  /* a */ pick('./mid.vue') /* b */\n  'x')"
  DYN_IMPORT_RE.lastIndex = 0
  const capOf = (s) => [...s.matchAll(DYN_IMPORT_RE)].map((m) => m[1])
  const magic = capOf(MAGIC)
  if (magic.length !== 1 || magic[0] !== '3d-force-graph') fails.push(`魔法注释形态没被认成字面量边（捕获=${JSON.stringify(magic)}）⇒ 动态 import 的注释前缀段被删/改坏，活边会被读成死边`)
  if (isNonLiteralDynamic(MAGIC)) fails.push('同一形态既被认成字面量边又被判非字面量 ⇒ 两半互相矛盾')
  if (isNonLiteralDynamic('// 缓存 Promise，避免重复 import()）')) fails.push('散文注释里的裸 import() 被判非字面量 ⇒ 假 UNRESOLVED 拖垮整本账')
  if (!isNonLiteralDynamic('import(modPath)')) fails.push('真非字面量 import(变量) 没被点名 ⇒ 探测器哑了')
  const crossed = capOf(CROSSED)
  if (crossed.length !== 0) fails.push(`注释体越过了自己的结束符，把两个字面量之间的真代码吞掉了（捕获=${JSON.stringify(crossed)}）⇒ 体内必须拦下注释结束符，且不许跨行`)
  // 导入态零输出：文件末 isMain 那道闸是本账能被别的门禁工具（check-route-links）复用的前提。
  // 变异体：`if (!isMain) {` → `if (false) {` ⇒ 下面这条按名报红。
  // 钉为什么必须待在本文件、不能待在使用方：使用方是 **static import** 本文件的，闸一坏，本文件就在
  // 对方进程里执行 main 分支、并在 selftest 末尾 process.exit(0) —— 实测对方 stdout 被这本账的第一屏
  // 占满、退出码仍是 0、对方自己的 selftest 一行都没跑。放在这里才是"闸坏 ⇒ 闸的账报红"。
  let imported = ''
  try {
    imported = execFileSync(process.execPath, ['--input-type=module', '-e',
      "import('./scripts/gate/check-import-reach.mjs').then(()=>console.log('IMPORTED-OK'))"],
    { cwd: ROOT, encoding: 'utf8' })
  } catch (e) { imported = String(e.stdout ?? '') }
  if (imported.trim() !== 'IMPORTED-OK') fails.push(`导入态出 ${imported.trim() ? imported.trim().split('\n').length : 0} 行（首行=${JSON.stringify(imported.split('\n')[0].slice(0, 30))}）⇒ isMain 那道闸被删/改坏，本账会在复用方进程里喷报告`)
  // 分母自证：孤儿 + 可达 = 全集，且三态互斥
  if (a.seen.size + a.orphans.length !== a.universe.length) fails.push(`账不平 seen ${a.seen.size} + orphans ${a.orphans.length} ≠ universe ${a.universe.length} ⇒ seen 里有账外条目: ${a.offLedger.join(', ') || '(空，说明 seen 里被 isExcluded 漏放行)'}`)
  if (a.universe.length < 100) fails.push(`分母只有 ${a.universe.length} 个文件 ⇒ 扫描集八成错了，不足以下"孤儿"结论`)
  console.log(`SELFTEST ${fails.length ? 'FAIL' : 'PASS'} universe=${a.universe.length} seen=${a.seen.size} orphans=${a.orphans.length} unresolved=${a.unresolved.length}`)
  for (const f of fails) console.log('  ! ' + f)
  process.exit(fails.length ? 1 : 0)
}

// 只有被"直接执行"时才出报告：别的门禁工具（check-route-links.mjs）要复用这本孤儿账，
// 一旦 import 就喷出 30 多行清单，读者的第一屏会被错的东西占满 —— 所以导入态必须一个字都不印。
// 变异体：删掉 isMain 这道闸 ⇒ 本文件 --selftest 里"导入态零输出"那条钉按名报红（钉在本文件，理由见那条）。
const SELF = fileURLToPath(import.meta.url)
const ENTRY = path.resolve(typeof process.argv[1] === 'string' ? process.argv[1] : '')
const isMain = process.platform === 'win32' ? SELF.toLowerCase() === ENTRY.toLowerCase() : SELF === ENTRY

if (!isMain) {
  // 被当作模块导入：只交出函数，不出报告、不占退出码
} else if (process.argv.includes('--selftest')) selftest()
else {
  const a = analyze(SRC)
  const rows = a.orphans.map((r) => ({ file: r, lines: a.linesOf(r), hex: a.hexOf(r) }))
    .sort((x, y) => y.hex - x.hex || x.file.localeCompare(y.file))
  if (process.argv.includes('--json')) console.log(JSON.stringify({ seen: a.seen.size, universe: a.universe.length, unresolved: a.unresolved, orphans: rows }, null, 2))
  else {
    console.log(`分母 ${a.universe.length} 个 src 文件（.vue/.js/.ts/.css，排除 *.test.js/*.stories.js/__mocks__），可达 ${a.seen.size}，孤儿 ${a.orphans.length}`)
    if (a.unresolved.length) console.log(`UNRESOLVED ${a.unresolved.length} 处（账不可信，先修说明符形态）：\n  ${a.unresolved.slice(0, 10).join('\n  ')}`)
    console.log('孤儿清单（hex = 该文件裸 hex 处数，是给死码抛光还是退役，按此排序裁决）：')
    for (const r of rows) console.log(`  ${String(r.hex).padStart(4)} hex  ${String(r.lines).padStart(5)} 行  ${r.file}`)
    const orphanHex = rows.reduce((s, r) => s + r.hex, 0)
    const allHex = a.universe.reduce((s, r) => s + a.hexOf(r), 0)
    console.log(`裸 hex 归属：孤儿 ${orphanHex} / 全集 ${allHex} ⇒ 活码占 ${allHex ? ((allHex - orphanHex) / allHex * 100).toFixed(1) : '0.0'}%`)
  }
}
