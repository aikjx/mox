// 图标登记口收口（§5.25/§5.26）留下的样式欠账：回落用的类名必须在同一个 SFC 里存在。
// 起因：AdminRole.vue 与 ProjectsView.vue 的回落分支分别写了 class="muted" / class="cat-icon-fallback"，
// 而两个 SFC 的 <style scoped> 里都没有这条规则 ⇒ 回落文字按默认色/字号渲染（画得出来但没样式）。
import { describe, it, expect } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')

// [文件, 模板里用到的静态类名, 该类的定义处]
const PAIRS = [
  ['views/admin/panels/AdminRole.vue', 'muted', 'same-file'],
  ['views/project/ProjectsView.vue', 'cat-icon-fallback', 'same-file'],
  ['views/admin/panels/AdminMenu.vue', 'muted', 'same-file'],
  ['views/admin/panels/AdminMenu.vue', 'icon-option', 'same-file'],
]

const read = (rel) => fs.readFileSync(path.join(SRC, rel), 'utf8')
const styleBlock = (t) => (t.includes('<style') ? t.slice(t.lastIndexOf('<style')) : '')
const used = (t, cls) => new RegExp(`(?<![:\\w-])class="[^"{}]*\\b${cls}\\b`).test(t)
const defined = (t, cls) => new RegExp(`\\.(${cls})(?![\\w-])\\s*[,{]`).test(styleBlock(t))

describe('回落分支的类名有定义', () => {
  it.each(PAIRS)('%s 用到 .%s 且同文件 <style> 里有规则', (rel, cls) => {
    const t = read(rel)
    expect(used(t, cls), `${rel} should still use .${cls}`).toBe(true)
    expect(defined(t, cls), `${rel} has no style rule for .${cls}`).toBe(true)
  })

  it('驱动器不许把规则塞进注释里（去掉注释后仍判得出定义存在）', () => {
    for (const [rel, cls] of PAIRS) {
      const t = read(rel)
      const noComment = styleBlock(t).replace(/\/\*[\s\S]*?\*\//g, '')
      expect(new RegExp(`\\.${cls}(?![\\w-])\\s*[,{]`).test(noComment), rel + ': ' + cls)
        .toBe(true)
    }
  })

  it('两处回落分支都还挂着 navIcon 解析（样式补上不等于口径回退）', () => {
    for (const rel of ['views/admin/panels/AdminRole.vue', 'views/project/ProjectsView.vue']) {
      expect(read(rel)).toContain("nav-icons'")
    }
  })
})
