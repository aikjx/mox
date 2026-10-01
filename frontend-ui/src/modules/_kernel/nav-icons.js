// 字符串图标名的唯一登记口（外壳层）。
//
// 机制：vite.config.js 里的 unplugin-vue-components resolver 只在编译期改写**模板标签**
// （<el-icon> / <ElIconAim>），三类位点它一条都不改写：
//   1. 数据表里的 icon: 'Aim' —— 导航、快捷指令、通知类型、引导步骤、KPI 卡、门户特性位；
//   2. 内联三元 :is="open ? 'ArrowUp' : 'ArrowDown'"；
//   3. 裸写的图标标签 <Refresh /> / <MoreFilled :size="14" /> ——  resolver 只认 ElIconXxx 前缀，
//      裸名既没被自动导入、文件里也没有本地 import（浏览器实测：一路由表扫下来 20+ 条
//      "Failed to resolve component" 告警，图标位为空）。
// 这三类位点在没有运行期登记项时一律 "Failed to resolve component"、渲染成空白
// （TheSidebar.vue 的旧注释「main.js 已全局注册」是假的——那正是本文件补上的东西）。
//
// 本表是这三类名的封闭集（规模以 nav-icons.test.js 现场扫描为准，该测试同时钉住每通道下限）：
// main.js 用 registerNavIcons(app) 逐项注册；
// 需要"未登记就回落到文本"的位点（侧栏要兼容 emoji 旧配置）用 navIcon(name) 显式解析。
// 不做 `import *` 整包注册：该包导出 293 个图标，整包进 app.component 会把它们全打进首屏
// ⇒ 逐个点名，tree-shake 保住，同时让"用了没登记的名"成为可判定量：
// 同目录 nav-icons.test.js 全库扫描上述两条通道，新图标名必须先来回本表登记，否则测试红。
import {
  Aim,
  ArrowDown,
  ArrowUp,
  Avatar,
  Box,
  Bottom,
  ChatDotRound,
  ChatLineSquare,
  Check,
  CircleCheck,
  CircleCheckFilled,
  CircleCloseFilled,
  Clock,
  Close,
  Coin,
  Collection,
  CollectionTag,
  Connection,
  Cpu,
  DataAnalysis,
  DataBoard,
  Delete,
  EditPen,
  Document,
  Expand,
  Files,
  Fold,
  Folder,
  FolderAdd,
  FolderOpened,
  Grid,
  Key,
  List,
  Lock,
  MagicStick,
  Menu,
  Message,
  Monitor,
  MoreFilled,
  Odometer,
  OfficeBuilding,
  Operation,
  Plus,
  Promotion,
  Refresh,
  RefreshRight,
  Search,
  Select,
  SetUp,
  Setting,
  Share,
  Shop,
  SwitchButton,
  Tickets,
  Top,
  Tools,
  TrendCharts,
  Upload,
  User,
  UserFilled,
  VideoPlay,
  View,
  Warning
} from '@element-plus/icons-vue'

export const NAV_ICONS = Object.freeze({
  Aim,
  ArrowDown,
  ArrowUp,
  Avatar,
  Box,
  Bottom,
  ChatDotRound,
  ChatLineSquare,
  Check,
  CircleCheck,
  CircleCheckFilled,
  CircleCloseFilled,
  Clock,
  Close,
  Coin,
  Collection,
  CollectionTag,
  Connection,
  Cpu,
  DataAnalysis,
  DataBoard,
  Delete,
  EditPen,
  Document,
  Expand,
  Files,
  Fold,
  Folder,
  FolderAdd,
  FolderOpened,
  Grid,
  Key,
  List,
  Lock,
  MagicStick,
  Menu,
  Message,
  Monitor,
  MoreFilled,
  Odometer,
  OfficeBuilding,
  Operation,
  Plus,
  Promotion,
  Refresh,
  RefreshRight,
  Search,
  Select,
  SetUp,
  Setting,
  Share,
  Shop,
  SwitchButton,
  Tickets,
  Top,
  Tools,
  TrendCharts,
  Upload,
  User,
  UserFilled,
  VideoPlay,
  View,
  Warning
})

export function navIconNames() {
  return Object.keys(NAV_ICONS)
}

/**
 * 图标名 → 组件；未登记或不是 PascalCase（旧配置里的 emoji）返回 null，
 * 由调用方回退成文本：看得见一个原始串，远好于看见空白。
 */
export function navIcon(name) {
  if (typeof name !== 'string') return null
  return Object.prototype.hasOwnProperty.call(NAV_ICONS, name) ? NAV_ICONS[name] : null
}

/** 把封闭集注册为全局组件，返回注册数（测试用它钉"注册数 = 表长"）。 */
export function registerNavIcons(app) {
  for (const [name, comp] of Object.entries(NAV_ICONS)) app.component(name, comp)
  return Object.keys(NAV_ICONS).length
}
