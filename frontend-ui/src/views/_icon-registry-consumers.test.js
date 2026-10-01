// 图标登记口的"外部字符串消费者"不变量：
// DB／服务端回包里的 icon 名不许直接进 <component :is>，必须先过 navIcon()（名单外→null→文本回落）。
// 见证通道是编译产物本身：SFC 编译出的 render 函数里，`<script setup>` 的绑定以 `navIcon` 身份出现，
// 所以"这一位点确实走了解析器"是可判定的；把解析器撤掉（退回裸 :is）这条引用就消失，测试立刻红。
// 之所以用编译产物而不是挂载：这两处站点分别在角色权限弹窗与新建项目对话框的 v-if 子树里，
// 打开弹窗需要各自的替身链路，而本文件只钉"挂在通道上"这一件事（真机渲染另见 §5.26 的欠账）。
import { describe, it, expect } from 'vitest'
import AdminRole from '@/views/admin/panels/AdminRole.vue'
import ProjectsView from '@/views/project/ProjectsView.vue'
import AdminMenu from '@/views/admin/panels/AdminMenu.vue'

const renderSrc = (comp) => String(comp.render || '')

const sites = [
  ['AdminRole 菜单权限树节点（data.icon ← getMenuTree 的 DB 列）', AdminRole, 1],
  ['ProjectsView 分类选择器（c.icon ← getProjectTypes 的服务端字段）', ProjectsView, 1],
  ['AdminMenu 两个渲染位点（row.icon ← 同一张菜单表）', AdminMenu, 2],
]

describe('DB 驱动的图标名只能经登记口进模板', () => {
  it('编译产物里每个位点都引用 navIcon，且没有把外部 icon 名直接交给 :is', () => {
    for (const [label, comp, minRefs] of sites) {
      const src = renderSrc(comp)
      expect(src.length, `${label}: 拿不到 render 函数`).toBeGreaterThan(0)
      const refs = (src.match(/navIcon/g) || []).length
      expect(refs, `${label}: navIcon 引用数`).toBeGreaterThanOrEqual(minRefs)
      // 编译形态实测（D:\tmp\render-adminmenu.txt，由 AdminMenu.render 原样落盘）：
      //   合规 → resolveDynamicComponent($setup.navIcon(row.icon))
      //   违规 → resolveDynamicComponent(row.icon)（M4 变异体就是这一形，用它当这道的正对照）
      expect(src, `${label}: 仍把外部 icon 名直接当标签`).not.toMatch(/resolveDynamicComponent\(\s*(?!navIcon\()[\w$?.]*\.icon\b/)
    }
  })
})
