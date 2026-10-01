#!/usr/bin/env node
/**
 * 路由链接账 —— 界面里"点得到、但路由表没有"的目标（死链），为 #26/#5 的接线与退役裁决出证据。
 *
 * 为什么要有这本账：可达性账只认 import 边，看不见 `<router-link to="/x">` 这类**字符串**引用。
 * 一个页面即使走不进闭包（判为孤儿），它挂在外壳导航上的链接照样能点；反过来，活页里指向
 * 不存在路由的链接不会让任何测试变红 —— vue-router 只在运行时告警。2026-09-25 的认证簇就是
 * 这么漏过去的：活登录页挂着 /register 与 /forgot-password 两个链接，而这两条 path 全仓无定义。
 *
 * 三态：ALIVE（命中路由表）/ DEAD（字面量目标但路由表无匹配）/ DYNAMIC（非字面量，只记名不判决）。
 * 路由表两个来源：src/router 下的模块（含 children 嵌套）+ src/modules 描述符里 routes 数组的字面 path。
 * 刻意不 import vue-router（拿不到 @ 别名下的 .vue），所以这是一本**文本账**：宁可将不可判的记 DYNAMIC，
 * 也不把"我看不见"写成"它不存在"。
 *
 * 用法：node scripts/gate/check-route-links.mjs [--json] [--selftest] [--all]
 *   默认只报告不改红（与可达性账同规格，先当决策证据）。
 */
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { analyze } from './check-import-reach.mjs'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const SRC = path.join(ROOT, 'src')
const REL = (p) => path.relative(SRC, p).replace(/\\/g, '/')
const isNoise = (rel) => /\.test\.js$/.test(rel) || /\.stories\.(js|mdx)$/.test(rel)
  || rel.includes('__mocks__') || rel.includes('node_modules')

const walk = (dir, out = []) => {
  if (!fs.existsSync(dir)) return out
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, e.name)
    if (e.isDirectory()) walk(full, out)
    else if (/\.(vue|js)$/.test(e.name)) out.push(full)
  }
  return out
}

// ---------- 路由表 ----------
const depthAt = (text, idx) => {
  let d = 0
  for (let i = 0; i < idx; i++) {
    const c = text[i]
    if (c === '{') d++
    else if (c === '}') d--
  }
  return d
}
// 描述符里 nav 数组也写 path（同一事实的第二源），必须只认 routes 数组那一段：
// 找到 `routes:` 后面紧跟的 `[`，配对走到它的 `]`，落在这个跨度里的 path 才算一条路由。
const routesSpans = (text) => {
  const spans = []
  const re = /\broutes\s*:\s*/g
  for (let m = re.exec(text); m; m = re.exec(text)) {
    const open = text.indexOf('[', m.index + m[0].length)
    if (open < 0) continue
    let d = 0, i = open
    for (; i < text.length; i++) {
      if (text[i] === '[') d++
      else if (text[i] === ']') { d--; if (d === 0) break }
    }
    spans.push([open, i])
  }
  return spans
}
const PATH_RE = /\bpath\s*:\s*['"]([^'"]*)['"]/g
const NAME_RE = /\bname\s*:\s*['"]([^'"]*)['"]/g

// 通配/兜底路由（`:pathMatch(.*)*`、`*`）会接住任意 path，绝不能进 ALIVE 的匹配器集合 ——
// 一条 fallback 就能把整本死链账洗成 0，而这正是这本账唯一要防的假绿。它只改变"后果"的措辞。
export const isCatchAll = (p) => /\(\.\*\)|\*/.test(p)

// path → 匹配器：`:param` 吃一段（不跨 `/`），结尾斜杠可有可无。只喂非通配 path。
export function compileRoute(p) {
  let src = ''
  const segs = p.split('/')
  for (let i = 0; i < segs.length; i++) {
    const s = segs[i]
    if (i > 0) src += '\\/'
    src += s.startsWith(':') ? '[^/]+' : s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }
  return new RegExp('^' + src + '\\/?$')
}

// ---------- 链接提取 ----------
// 反引号模板**只**归 TPL_PUSH 管：两条判据都吃 `` ` `` 时同一个引用点会进两次账，
// 而模板里的 `/${path}` 会被这条当字面量收走 ⇒ 一个动态跳转被判成 DEAD（假死链比死链更难发现）。
const ROUTER_CALL = /(?:^|[^\w.$])(?:\$?router|router)\s*\.\s*(?:push|replace)\s*\(\s*(['"])([^'"]*)\1/g
const ROUTER_OBJ = /(?:^|[^\w.$])(?:\$?router|router)\s*\.\s*(?:push|replace)\s*\(\s*\{\s*(path|name)\s*:\s*(['"])([^'"]*)\2/g
const TPL_STATIC = /(^|[^\w:])to="([^"]+)"/g
const TPL_BOUND = /:to="([^"]+)"/g
const TPL_PUSH = /(?:^|[^\w.$])(?:\$?router|router)\s*\.\s*(?:push|replace)\s*\(\s*`([^`]*)`/g
const DYN_CALL = /(?:^|[^\w.$])(?:\$?router|router)\s*\.\s*(?:push|replace)\s*\(\s*[A-Za-z_$]/g

const isExternal = (t) => /^(https?:|mailto:|tel:|#|javascript:)/i.test(t)

export function extractLinks(text) {
  const hits = []
  const line = (idx) => text.slice(0, idx).split('\n').length
  const push = (kind, target, idx) => hits.push({ kind, target, line: line(idx) })
  for (const m of text.matchAll(TPL_STATIC)) if (m[2] && !isExternal(m[2]) && !m[2].includes('{{')) push('path', m[2], m.index)
  for (const m of text.matchAll(TPL_BOUND)) push('dynamic', m[1], m.index)
  for (const m of text.matchAll(ROUTER_CALL)) if (m[2]) push('path', m[2], m.index)
  for (const m of text.matchAll(ROUTER_OBJ)) push(m[1] === 'name' ? 'name' : 'path', m[3], m.index)
  for (const m of text.matchAll(TPL_PUSH)) push(m[1].includes('${') ? 'pattern' : 'path', m[1], m.index)
  for (const m of text.matchAll(DYN_CALL)) push('dynamic', m[0].trim(), m.index)
  return hits
}

export function stripQuery(t) {
  return t.split(/[?#]/)[0]
}

export function audit(srcDir) {
  const S = (p) => path.join(srcDir, p)
  const routeFiles = []
  for (const f of walk(S('router'))) {
    const rel = REL(f)
    if (isNoise(rel) || rel === 'router/index.js') continue
    routeFiles.push({ rel, text: fs.readFileSync(f, 'utf8') })
  }
  const descriptors = []
  for (const f of walk(S('modules'))) {
    const rel = REL(f)
    if (isNoise(rel) || !/\/index\.js$/.test(rel)) continue
    descriptors.push({ rel, text: fs.readFileSync(f, 'utf8') })
  }
  const table = collectRoutes(routeFiles, descriptors)
  const catchAll = table.entries.filter((e) => isCatchAll(e.full))
  const matchers = table.entries.filter((e) => !isCatchAll(e.full))
    .map((e) => ({ ...e, re: compileRoute(e.full) }))
  const dead = [], alive = [], dynamic = [], parented = [], dropped = []
  let extracted = 0
  for (const f of walk(srcDir)) {
    const rel = REL(f)
    if (isNoise(rel) || rel.startsWith('router/')) continue
    const text = fs.readFileSync(f, 'utf8')
    for (const h of extractLinks(text)) {
      extracted++
      if (h.kind === 'dynamic') { dynamic.push({ ...h, file: rel }); continue }
      if (h.kind === 'name') {
        ;(table.names.has(h.target) ? alive : dead).push({ ...h, file: rel })
        continue
      }
      const target = stripQuery(h.target)
      // 模板里带 ${} 的跳转一律不判决：拿探针把它洗成 ALIVE，等于用"我猜它指这条路由"
      // 顶替"它确实指这条路由"，猜错就是整片假活页。
      if (h.kind === 'pattern') { dynamic.push({ ...h, file: rel, target }); continue }
      if (matchers.some((m) => m.re.test(target))) alive.push({ ...h, file: rel, target })
      else if (matchers.some((m) => m.full.startsWith(target + '/'))) parented.push({ ...h, file: rel, target })
      else if (target.startsWith('/')) dead.push({ ...h, file: rel, target })
      // 走到这里就是"既不在任何态里、也没被记账"的样本 —— 必须留下名字，不能被循环吞掉：
      // 一个静默少计的账会让"死链 0 条"看着像好消息。
      else dropped.push({ ...h, file: rel, target })
    }
  }
  return { table, matchers, catchAll, dead, alive, dynamic, parented, dropped, extracted, routeFiles, descriptors }
}

// 两遍扫描共用同一套 path 匹配：router 目录整体算，描述符只算 routes 段那一个跨度。
function collectRoutes(routeFiles, descriptors) {
  const entries = []
  const names = new Set()
  for (const group of [[routeFiles, false], [descriptors, true]]) {
    const [files, routeOnly] = group
    for (const { rel, text } of files) {
      const spans = routeOnly ? routesSpans(text) : [[0, text.length]]
      const stack = []
      for (const m of text.matchAll(PATH_RE)) {
        const raw = m[1]
        if (!raw || !(spans.some(([a, b]) => m.index > a && m.index < b))) continue
        const d = depthAt(text, m.index)
        while (stack.length && stack[stack.length - 1].depth >= d) stack.pop()
        const parent = stack.length ? stack[stack.length - 1].full : null
        const full = raw.startsWith('/') || !parent ? raw : `${parent.replace(/\/$/, '')}/${raw}`
        const e = { depth: d, full, where: `${rel}:${text.slice(0, m.index).split('\n').length}` }
        stack.push(e)
        entries.push(e)
      }
      if (!routeOnly) for (const m of text.matchAll(NAME_RE)) names.add(m[1])
    }
  }
  return { entries, names }
}

function descriptorSources(a) {
  const rels = new Set(a.descriptors.map((d) => d.rel))
  const hit = new Set()
  for (const e of a.table.entries) {
    const rel = e.where.slice(0, e.where.lastIndexOf(':'))
    if (rels.has(rel)) hit.add(rel)
  }
  return hit
}

function report(a) {
  const orphans = new Set(analyze(SRC).orphans)
  const tag = (f) => (orphans.has(f) ? '  [孤儿页]' : '')
  // 扫到的 index.js 份数 ≠ 贡献路由的份数（src/modules/index.js 只是聚合器，没有 routes 段）。
  const fromDescriptors = descriptorSources(a)
  console.log(`路由表 ${a.table.entries.length} 条（router ${a.routeFiles.length} 份 + 模块描述符 ${fromDescriptors.size} 份，共扫 index.js ${a.descriptors.length} 份；name ${a.table.names.size} 个）`)
  console.log(`链接候选：提取 ${a.extracted} 处 = 活 ${a.alive.length} + 死 ${a.dead.length} + 父路径 ${a.parented.length} + 不可判 ${a.dynamic.length} + 漏计 ${a.dropped.length}`)
  if (a.dropped.length) {
    console.log('DROPPED（提取到了却没进任何态 ⇒ 上面那三个数都不能信）：')
    for (const d of a.dropped) console.log(`  ${d.file}:${d.line}  →  ${JSON.stringify(String(d.target).slice(0, 60))}`)
  }
  if (a.dead.length) {
    // 兜底路由决定了"死链"的真实后果：不是白屏，而是被 redirect 到某个不相关的页面 ——
    // 用户点了「注册」回到工作台，看着像"按钮坏了"，所以这类缺陷没有任何报错。
    const ca = a.catchAll.map((e) => e.full)
    console.log(`DEAD（路由表无真实落点；${ca.length} 条通配路由 ${ca.join(', ')} 会接住它们 ⇒ 后果是被 redirect 去别的页面，不是白屏）：`)
    for (const d of a.dead) console.log(`  ${d.file}:${d.line}  →  ${d.target}${tag(d.file)}`)
  }
  if (a.parented.length) {
    console.log('PARENT（只匹配到某条路由的前缀，vue-router 不保证有落点）：')
    for (const d of a.parented) console.log(`  ${d.file}:${d.line}  →  ${d.target}${tag(d.file)}`)
  }
  if (process.argv.includes('--all') && a.dynamic.length) {
    console.log('DYNAMIC（非字面量，只能人工看）：')
    for (const d of a.dynamic.slice(0, 40)) console.log(`  ${d.file}:${d.line}  →  ${String(d.target).slice(0, 60)}`)
  }
  return orphans
}

function selftest() {
  const fails = []
  const a = audit(SRC)
  const deadOf = (f, t) => a.dead.some((d) => d.file === f && d.target === t)
  const hasAlive = (t) => a.alive.some((x) => x.target === t)
  // 路由表必须非空且含已知条目：解析出 0 条时"一切皆死"会让这本账看着很严重、其实全错。
  const paths = new Set(a.table.entries.map((e) => e.full))
  if (a.table.entries.length < 20) fails.push(`路由表只有 ${a.table.entries.length} 条 ⇒ 八成解析错了，后面的死链判决全部无效`)
  for (const p of ['/login', '/alliance/console', '/403']) {
    if (!paths.has(p)) fails.push(`路由表里没有 ${p} ⇒ 派生断了（router/modules/public.js 或模块描述符没被吃到）`)
  }
  // 认证簇 2026-09-25 曾被这本账点名（活登录页挂 /register 与 /forgot-password 两条无定义路由）。
  // 该簇现已修，故自检改钉"修好的形态"而非"历史 bug 态"——把已消失的死链焊成"必须是死的"，
  // 修好后只会假报 FAIL、诱把人 revert。死链判据的非空转由下方 sentinel(任意 bogus path 不被
  // 匹配器吃下) + 匹配器正反例对 共同保证，不依赖某条真实链接此刻是死是活。
  if (!paths.has('/register')) fails.push('/register 不在路由表 ⇒ 与 public.js 现状矛盾（补好的注册入口又丢了）')
  if (!hasAlive('/register')) fails.push('/register 已在表内且登录页引用它却没判活 ⇒ 模板提取或匹配器断了一环')
  if (paths.has('/forgot-password')) fails.push('路由表里有 /forgot-password ⇒ 后端无此端点、页面已退役，不得再挂路由')
  // 反向钉：真在被引用的活路径不得进死账。
  if (!hasAlive('/login')) fails.push('/login 有引用却没被判活 ⇒ 引用点或匹配器丢了')
  for (const t of a.dead) if (paths.has(t.target)) fails.push(`死链判决自相矛盾：${t.target} 既在路由表又被判死`)
  // 描述符里的 nav 段不得当路由：expert-alliance/index.js 的 nav 与 routes 各写一遍 path，只能计一次。
  const alliance = a.table.entries.filter((e) => e.full === '/alliance/console').length
  if (alliance !== 1) fails.push(`/alliance/console 在路由表里出现 ${alliance} 次 ⇒ routes 段与 nav 段没分开了（第二源会掩盖死链）`)
  // 通配路由不得进 ALIVE 的匹配器集合：一条 /:pathMatch(.*)* 就能把整本死账洗成 0。
  if (a.catchAll.length < 1) fails.push('路由表里没有通配/兜底路由 ⇒ fallback 路由没被识别，DEAD 的后果措辞也失去依据')
  const sentinel = '/zz-not-a-route-9x'
  const washed = a.matchers.filter((m) => m.re.test(sentinel)).map((m) => m.full)
  if (washed.length) fails.push(`匹配器 ${washed.join(', ')} 吃下了任意 path ⇒ 死链会被兜底路由洗成 0 条`)
  // 匹配器语义正反例成对：只钉"命中"会让"什么都命中"也绿，只钉"不命中"会让"什么都进不去"也绿。
  for (const [p, ok, no] of [
    ['/a/:id', ['/a/x', '/a/x/'], ['/a', '/a/', '/a/x/y']],
    ['/market/:id', ['/market/7'], ['/market', '/market/7/8']],
  ]) {
    const re = compileRoute(p)
    for (const t of ok) if (!re.test(t)) fails.push(`${p} 应命中 ${t} 却没命中 ⇒ 真活页会被判成死链`)
    for (const t of no) if (re.test(t)) fails.push(`${p} 不该命中 ${t} 却命中了 ⇒ 死链会被多吃的一段假参数洗白`)
  }
  // 一个引用点只能进一个态：同一 file:line:target 落在两个桶 = 两条提取判据重叠，一本账出现两个数。
  const stated = new Map()
  for (const [state, list] of [['alive', a.alive], ['dead', a.dead], ['dynamic', a.dynamic], ['parented', a.parented]]) {
    for (const h of list) {
      const k = `${h.file}:${h.line}:${h.target}`
      if (stated.has(k)) fails.push(`${k} 同时进了 ${stated.get(k)} 与 ${state} ⇒ 两个提取器吃同一种形态`)
      else stated.set(k, state)
    }
  }
  // 总数守恒：提取到的每一处必须落在恰好一个态里。漏计会让"死链 0 条"冒充好消息。
  const bucketed = a.alive.length + a.dead.length + a.dynamic.length + a.parented.length + a.dropped.length
  if (bucketed !== a.extracted) fails.push(`提取 ${a.extracted} 处而各态相加 ${bucketed} ⇒ 两个计数器不同源，这条账的总数不可信`)
  if (a.dropped.length) fails.push(`${a.dropped.length} 处提取到却没进任何态：${a.dropped.slice(0, 5).map((d) => `${d.file}:${d.line}`).join(', ')}`)
  // 反引号模板里的跳转一律 DYNAMIC：既不判死（目标不是字面量）也不判活（探针命中只是猜测）。
  if (a.dead.some((h) => String(h.target).includes('${'))) fails.push('DEAD 里出现带 ${} 的模板目标 ⇒ 动态跳转被判成死链（假死链）')
  {
    const tpl = a.dynamic.filter((h) => h.file === 'views/project/ProjectsView.vue' && String(h.target).includes('${'))
    if (!tpl.length) fails.push('ProjectsView.vue 的模板跳转没进 DYNAMIC ⇒ 它被别的提取器当成了字面量，上面那条钉也就空转了')
  }
  // Array.prototype.push 不是路由跳转。
  if (extractLinks("list.push('/not-a-route-xyz')").length) fails.push('数组 push 被当成 router.push ⇒ 死链账会被业务数组灌满')
  // "可达性账导入态零输出"这条钉**不放在这里**，放在 check-import-reach.mjs 自己的 selftest 里：
  // 本文件 static import 那个账，闸一坏它就在本进程里跑完报告并 process.exit(0) —— 下面的判决行根本轮不到，
  // 实测 mutated 时本文件 --selftest 只输出对方那行 universe=… 且 rc=0（假绿）。见那文件里的钉与变异体注释。
  console.log(`SELFTEST ${fails.length ? 'FAIL' : 'PASS'} routes=${a.table.entries.length} extracted=${a.extracted} alive=${a.alive.length} dead=${a.dead.length} dynamic=${a.dynamic.length} dropped=${a.dropped.length}`)
  for (const f of fails) console.log('  ! ' + f)
  process.exit(fails.length ? 1 : 0)
}

const SELF = fileURLToPath(import.meta.url)
const ENTRY = path.resolve(typeof process.argv[1] === 'string' ? process.argv[1] : '')
const isMain = process.platform === 'win32' ? SELF.toLowerCase() === ENTRY.toLowerCase() : SELF === ENTRY

if (!isMain) {
  // 被导入：只交出函数
} else if (process.argv.includes('--selftest')) selftest()
else {
  const a = audit(SRC)
  if (process.argv.includes('--json')) console.log(JSON.stringify({ routes: a.table.entries.map((e) => e.full), dead: a.dead, parented: a.parented }, null, 2))
  else report(a)
}
