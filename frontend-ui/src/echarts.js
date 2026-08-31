import * as echartsCore from 'echarts/core'

import {
  BarChart,
  LineChart,
  RadarChart,
  GraphChart,
  GaugeChart,
  PieChart,
} from 'echarts/charts'

import {
  TooltipComponent,
  LegendComponent,
  GridComponent,
  TitleComponent,
  GraphicComponent,
  DataZoomComponent,
} from 'echarts/components'

import { CanvasRenderer } from 'echarts/renderers'
import { graphic as echartsGraphic } from 'echarts'

echartsCore.use([
  BarChart,
  LineChart,
  RadarChart,
  GraphChart,
  GaugeChart,
  PieChart,
  TooltipComponent,
  LegendComponent,
  GridComponent,
  TitleComponent,
  GraphicComponent,
  DataZoomComponent,
  CanvasRenderer,
])

export const init = echartsCore.init
export const graphic = echartsGraphic
export default echartsCore
