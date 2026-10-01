// 全量 SFC 编译门禁：src 下每个 .vue 必须能被 vue/compiler-sfc 编译通过。
//
// 为什么需要它：2026-09-27 联盟工作台主入口（views/workspace/ExpertWorkspaceView.vue）
// 因 `<script setup>` 里重复声明标识符而根本无法编译，却因构建按入口分块、其他页面照常
// 跑通而长期没人发现——单页测试要挂载它才报，vite dev 只在打开那一页时报。
// 本门禁把「整棵视图树能不能编译」变成一条与用例无关的断言。
//
// 判据分三层，缺一层就是空转：
//  1) 扫描集规模下限（没有分母的"零失败"可以靠什么都没扫得到）
//  2) 正对照：故意坏掉的 SFC 必须被判坏（证明检测器通电且真走编译）
//  3) 反对照：合法 SFC 不得被判坏（证明零失败不是全量遮蔽）
import { readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'
import { describe, it, expect } from 'vitest'
import { compileScript, compileTemplate, parse } from 'vue/compiler-sfc'

// 锚在 src 根：从 frontend-ui、从 src、或从仓库根调用都扫同一棵树。
// 不用 import.meta.url —— vitest 转换后它不是 file: 协议，fileURLToPath 会直接抛错。
function hasViews(dir) {
  try {
    return readdirSync(dir, { withFileTypes: true }).some((e) => e.name === 'views' && e.isDirectory())
  } catch (e) {
    return false
  }
}

function findSrcDir(start) {
  let dir = start
  for (let i = 0; i < 6; i++) {
    if (hasViews(dir)) return dir
    if (hasViews(path.join(dir, 'src'))) return path.join(dir, 'src')
    dir = path.dirname(dir)
  }
  throw new Error(`未找到 src 根（自 ${start} 向上）`)
}

const SRC = findSrcDir(process.cwd())

function walkVue(dir, out = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) walkVue(full, out)
    else if (/\.vue$/.test(entry.name)) out.push(full)
  }
  return out
}

/** 纯函数：编译单个 SFC 源，返回错误列表（真语料与变异体共用同一条通道） */
export function compileSfc(source, filename = 'Probe.vue') {
  const errors = []
  let descriptor
  try {
    const parsed = parse(source, { filename })
    descriptor = parsed.descriptor
    for (const err of parsed.errors || []) errors.push(String(err.message || err))
    if (!descriptor) return errors
  } catch (e) {
    return [`parse 抛错：${e.message}`]
  }

  const id = 'probe'
  if (descriptor.script || descriptor.scriptSetup) {
    try {
      compileScript(descriptor, { id })
    } catch (e) {
      errors.push(`script 编译失败：${e.message}`)
    }
  }
  if (descriptor.template) {
    const result = compileTemplate({
      source: descriptor.template.content,
      filename,
      id,
      compilerOptions: { prefixIdentifiers: true }
    })
    for (const err of result.errors || []) errors.push(`template 编译失败：${String(err.message || err)}`)
  }
  return errors
}

describe('全量 SFC 必须可编译（视图树不允许静默坏掉）', () => {
  const files = walkVue(SRC)
  // 下限为 2026-09-27 实测 127 个 .vue 留出的余量（只许变多，不许变少）
  const SCAN_FLOOR = 120

  it('扫描集非空：零失败必须来自"逐个编译且都过"，不是来自"什么都没扫"', () => {
    expect(files.length, `被扫描 .vue 数 ${files.length} 低于下限 ${SCAN_FLOOR}`).toBeGreaterThanOrEqual(SCAN_FLOOR)
  })

  it('src 下每个 .vue 的 script/template 都能编译', () => {
    const failures = []
    for (const file of files) {
      const rel = path.relative(SRC, file).split(path.sep).join('/')
      const errors = compileSfc(readFileSync(file, 'utf8'), path.basename(file))
      if (errors.length) failures.push(`${rel}: ${errors[0]}`)
    }
    expect(failures, `${failures.length} 个 SFC 编译失败\n${failures.join('\n')}`).toEqual([])
  })

  it('正对照：重复声明标识符的 <script setup> 必须被判坏（本轮 P0 缺陷的形状）', () => {
    const broken = [
      '<template><div>{{ x }}</div></template>',
      "<script setup>",
      "import { collabMode } from '@/modules/expert-alliance/contract'",
      "const collabMode = ref('smart')",
      '</script>'
    ].join('\n')
    const errors = compileSfc(broken)
    expect(errors.length, '重复标识符未被判坏 ⇒ 门禁不编译 script，整条断言是空转').toBeGreaterThan(0)
    expect(errors.join(' ')).toContain('already been declared')
  })

  it('正对照：坏模板（标签未闭合）必须走 template 通道被判坏', () => {
    const errors = compileSfc('<template><div :a="1"\n<script setup>\nconst x = 1\n</script>')
    expect(errors.length, '未闭合模板未被判坏 ⇒ template 通道没接电').toBeGreaterThan(0)
  })

  it('正对照：未闭合的 <script setup> 会被判坏（孤儿 ProjectPicker 的旧病）', () => {
    const errors = compileSfc('<template><p>ok</p></template>\n<script setup>\nconst a = 1\n')
    expect(errors.length, '未闭合 script 未被判坏').toBeGreaterThan(0)
  })

  it('反对照：合法 SFC 不得被判坏（否则"零失败"是遮蔽出来的）', () => {
    const good = [
      '<template><div class="x">{{ label }}<span v-if="n">{{ n }}</span></div></template>',
      "<script setup>",
      "import { computed, ref } from 'vue'",
      "const n = ref(2)",
      "const label = computed(() => '甲')",
      '</script>',
      '<style scoped>.x { color: red }</style>'
    ].join('\n')
    expect(compileSfc(good)).toEqual([])
  })

  it('无 script 块的纯模板组件也走编译，不因缺 script 而被跳过', () => {
    const templateOnly = '<template><b>{{ v }}</b></template>'
    expect(compileSfc(templateOnly)).toEqual([])
    expect(compileSfc('<template><b></b><template>').length).toBeGreaterThan(0)
  })
})
