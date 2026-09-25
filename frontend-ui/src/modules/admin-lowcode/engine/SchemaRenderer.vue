<script>
/**
 * 递归渲染器：用 render 函数 + h() 输出 vnode，不使用运行时模板字符串编译。
 * node = schema.list.columns[i]（带 widget/prop/map/tagTypeOf/labelOf）。
 */
import { renderCell } from './widgetRegistry.js'

export default {
  name: 'SchemaRenderer',
  props: {
    node: { type: Object, required: true },
    row: { type: Object, default: () => ({}) },
  },
  setup(props) {
    return () => renderCell(props.node, props.row)
  },
}
</script>
