// 按需引入 ECharts（大幅减小打包体积，避免全量 ~1MB 打入每个图表视图）
//
// 仅注册实际使用的图表类型与组件。新增图表类型时，在此处补充对应 import 与 use()。
import * as echarts from 'echarts/core'

import {
  BarChart,
  LineChart,
  RadarChart,
  GraphChart,
  GaugeChart,
} from 'echarts/charts'

import {
  TooltipComponent,
  LegendComponent,
  GridComponent,
  TitleComponent,
} from 'echarts/components'

import { CanvasRenderer } from 'echarts/renderers'

echarts.use([
  BarChart,
  LineChart,
  RadarChart,
  GraphChart,
  GaugeChart,
  TooltipComponent,
  LegendComponent,
  GridComponent,
  TitleComponent,
  CanvasRenderer,
])

export default echarts
