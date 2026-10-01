#!/usr/bin/env node
/**
 * SFC 模板断链门禁 —— 检出「模板引用了 setup 从未导出的名字」与「块结构损坏」。
 *
 * 判据来自 Vue 编译产物：`compileScript(..., { inlineTemplate: true })` 后仍以 `_ctx.<name>`
 * 出现的标识符，就是 setup 作用域里查不到的名字（`$` 前缀是 Vue 有意走实例上下文的
 * $emit/$slots/$router…，属合法）。
 *
 * 两类缺陷都要拦：
 *   STRUCT   块结构坏了。`<script setup>` 漏掉 `</script>` 时 @vue/compiler-sfc 不报错，
 *            只是把 setup 内容判成空串 ⇒ 整块绑定丢失，模板全部退化为 `_ctx.x`，
 *            组件静默渲染成空（ProjectPicker.vue 曾以此形态入库）。
 *   DEADREF  块结构正常，但某个名字确实没人定义 ⇒ 轻则功能静默失效（prop 显式绑
 *            undefined 会吃到 default），重则渲染期 TypeError。
 *
 * 用法：node scripts/gate/check-sfc-dead-refs.mjs [--selftest]
 *
 * 已知边界：只有 `<script setup>` 能被判（`skippedNonSetup` 即此计数）。Options API 的
 * `<script>` 块里模板名要到运行时才解析，这里给不出断链结论 —— 被跳过的文件不是"干净"，
 * 是"未覆盖"。
 */
import { createRequire } from 'node:module'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const require = createRequire(path.join(ROOT, 'package.json'))
const { parse, compileScript } = require('vue/compiler-sfc')

const CTX_RE = /_ctx\.([A-Za-z_$][\w$]*)/g

// 分析单个 SFC 源码，返回 { struct: string[], deadRefs: string[] }
export function analyzeSfc(name, source) {
  const struct = []
  let descriptor
  try {
    descriptor = parse(source, { filename: name }).descriptor
  } catch (e) {
    return { struct: [`parse 抛错 ${String(e.message).slice(0, 100)}`], deadRefs: [] }
  }
  const parseErrors = (parse(source, { filename: name }).errors || []).length
  if (!descriptor.scriptSetup) return { struct: [], deadRefs: [], skipped: true }
  if (parseErrors > 0) struct.push(`SFC parse 报错 ${parseErrors} 处`)
  if (!descriptor.scriptSetup.content.trim()) struct.push('script setup 内容为空（多半是块未闭合）')

  let code
  try {
    code = compileScript(descriptor, { id: name, inlineTemplate: true }).content
  } catch (e) {
    return { struct: [...struct, `compile 抛错 ${String(e.message).slice(0, 100)}`], deadRefs: [] }
  }
  const deadRefs = []
  let m
  CTX_RE.lastIndex = 0
  while ((m = CTX_RE.exec(code))) {
    if (!m[1].startsWith('$') && !deadRefs.includes(m[1])) deadRefs.push(m[1])
  }
  // STRUCT 已解释全部退化；只在结构正常时把断链单独记账，避免一次截断伪装成 N 个断链
  return { struct, deadRefs: struct.length ? [] : deadRefs }
}

function walkVue(dir, out = []) {
  if (!fs.existsSync(dir)) return out
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name)
    if (ent.isDirectory()) walkVue(p, out)
    else if (ent.name.endsWith('.vue')) out.push(p)
  }
  return out
}

export function scanSources(sources) {
  const findings = { struct: [], deadRef: [] }
  let checked = 0, skipped = 0
  for (const [rel, src] of sources) {
    const r = analyzeSfc(rel, src)
    if (r.skipped) { skipped++; continue }
    checked++
    if (r.struct.length) findings.struct.push([rel, r.struct])
    else if (r.deadRefs.length) findings.deadRef.push([rel, r.deadRefs])
  }
  return { checked, skipped, findings }
}

// —— 自检：门禁必须抓得住两类植入缺陷，且不误报合法写法 ——
function selftest() {
  const cases = [
    {
      name: '植入断链 ⇒ 必须点名',
      sources: [['a/Dead.vue', '<template><div>{{ ghostName }}</div></template>\n<script setup>const ok = 1</script>\n']],
      want: { dead: [['a/Dead.vue', ['ghostName']]], struct: 0, checked: 1, skipped: 0 }
    },
    {
      name: '合法的 $ 前缀实例上下文 ⇒ 不许报',
      sources: [['b/Ok.vue', '<template><div @click="$emit(\'go\')" :class="$attrs.class">{{ ok }}</div></template>\n<script setup>const ok = 1</script>\n']],
      want: { dead: [], struct: 0, checked: 1, skipped: 0 }
    },
    {
      name: '漏掉 </script> ⇒ 记 STRUCT，不许退化成一片断链',
      sources: [['c/Cut.vue', '<template><div>{{ alpha }} {{ beta }}</div></template>\n\n<script setup>\nconst alpha = 1\nconst beta = 2\n']],
      want: { dead: [], struct: 1, checked: 1, skipped: 0 }
    },
    {
      name: '没有 script setup 的 SFC ⇒ 计入 skipped，不许混进 checked',
      sources: [['d/Old.vue', '<template><div>{{ msg }}</div></template>\n<script>\nexport default { data: () => ({ msg: \'x\' }) }\n</script>\n']],
      want: { dead: [], struct: 0, checked: 0, skipped: 1 }
    }
  ]
  let pass = 0
  for (const c of cases) {
    const got = scanSources(c.sources)
    const bad = []
    if (got.checked !== c.want.checked) bad.push(`checked=${got.checked} 期望 ${c.want.checked}`)
    if (got.skipped !== c.want.skipped) bad.push(`skipped=${got.skipped} 期望 ${c.want.skipped}`)
    if (got.findings.struct.length !== c.want.struct) bad.push(`struct=${got.findings.struct.length} 期望 ${c.want.struct}`)
    const deadJson = JSON.stringify(got.findings.deadRef)
    if (deadJson !== JSON.stringify(c.want.dead.map(([f, n]) => [f, n]))) bad.push(`deadRef=${deadJson} 期望 ${JSON.stringify(c.want.dead)}`)
    if (bad.length) console.log(`FAIL ${c.name} :: ${bad.join('; ')}`)
    else { pass++; console.log(`PASS ${c.name}`) }
  }
  const ok = pass === cases.length && cases.length > 0
  console.log(`SELFTEST cases=${cases.length} passed=${pass} verdict=${ok ? 'PASS' : 'FAIL'}`)
  process.exit(ok ? 0 : 1)
}

function main() {
  if (process.argv.includes('--selftest')) selftest()
  const files = walkVue(path.join(ROOT, 'src'))
  const sources = files.map((abs) => [path.relative(ROOT, abs).replace(/\\/g, '/'), fs.readFileSync(abs, 'utf8')])
  const { checked, skipped, findings } = scanSources(sources)
  for (const [rel, names] of findings.deadRef) console.log(`DEADREF ${rel} :: ${names.join(', ')}`)
  for (const [rel, notes] of findings.struct) console.log(`STRUCT  ${rel} :: ${notes.join(' | ')}`)
  const bad = findings.deadRef.length + findings.struct.length
  console.log(`SFCSCAN files=${files.length} checked=${checked} skippedNonSetup=${skipped} deadRefFiles=${findings.deadRef.length} structFiles=${findings.struct.length} verdict=${bad === 0 ? 'PASS' : 'FAIL'}`)
  process.exit(bad === 0 ? 0 : 1)
}

main()
