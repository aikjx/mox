// admin 面板冒烟测试共享替身：el-table 用 provide-rows 驱动行渲染，
// 其余 Element Plus 标签按未知元素渲染为原生节点，不拉整 EP 进测试环境。
// 与 AdminUser.smoke.test.js 同一范式，抽出复用。
import { h, provide, inject } from 'vue'

export const ROWS = Symbol('rows')

// el-tree：暴露面板会调用的 setCurrentKey/filter 等方法为 no-op，避免 treeRef.value 为空崩
export const ElTree = {
  name: 'el-tree',
  template: '<div class="el-tree-stub"></div>',
  methods: { setCurrentKey() {}, filter() {}, getNode() { return null }, setCheckedKeys() {} },
}

export const ElTable = {
  name: 'el-table',
  props: ['data', 'vLoading'],
  setup(props, { slots }) {
    provide(ROWS, () => props.data || [])
    return () => h('div', { class: 'el-table-stub', 'data-rows': String((props.data || []).length) }, slots.default?.())
  }
}

export const ElTableColumn = {
  name: 'el-table-column',
  props: ['label', 'prop'],
  setup(props, { slots }) {
    const rows = inject(ROWS, () => [])
    return () => h('div', { class: 'el-col' }, rows().map((row, i) =>
      h('div', { class: 'el-cell', 'data-i': String(i) },
        slots.default ? slots.default({ row, $index: i }) : (row[props.prop] ?? ''))
    ))
  }
}

export const ElButton = {
  name: 'el-button',
  props: ['type', 'icon', 'loading', 'disabled', 'link', 'size'],
  setup(props, { slots }) {
    return () => h('button', { class: 'el-btn', disabled: props.disabled ? '' : null }, slots.default?.())
  }
}

export const tableStubs = {
  'el-table': ElTable,
  'el-table-column': ElTableColumn,
  'el-button': ElButton,
  'el-tree': ElTree,
  'router-link': { props: ['to'], template: '<a class="stub-router-link"><slot/></a>' },
  'router-view': { template: '<div class="stub-router-view"></div>' },
}

export function setupGlobals() {
  globalThis.ElMessage = globalThis.ElMessage || { error: () => {}, success: () => {}, warning: () => {} }
  globalThis.ElMessageBox = globalThis.ElMessageBox || { confirm: async () => {}, alert: async () => {} }
}
