import * as echartsCore from 'echarts/core'

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
  GraphicComponent,
} from 'echarts/components'

import { CanvasRenderer } from 'echarts/renderers'
import { graphic as echartsGraphic } from 'echarts'

echartsCore.use([
  BarChart,
  LineChart,
  RadarChart,
  GraphChart,
  GaugeChart,
  TooltipComponent,
  LegendComponent,
  GridComponent,
  TitleComponent,
  GraphicComponent,
  CanvasRenderer,
])

export const init = echartsCore.init
export const graphic = echartsGraphic
export default echartsCore
