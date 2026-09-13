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

import { CanvasRenderer, SVGRenderer } from 'echarts/renderers'
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
  SVGRenderer,
])

export const init = echartsCore.init
export const getInstanceByDom = echartsCore.getInstanceByDom
export const graphic = echartsGraphic
export default echartsCore
