// ============================================================
// 专家联盟最优架构分析报告 · 图表配置
// ============================================================

(function () {
  const colors = {
    primary: '#6366f1',
    secondary: '#06b6d4',
    success: '#10b981',
    warning: '#f59e0b',
    danger: '#ef4444',
    muted: '#64748b',
    ink: '#0f172a',
    bg: '#f8fafc',
    rule: 'rgba(99, 102, 241, 0.15)'
  };

  // ----------------------------------------------------------
  // Chart 1: 三种架构方案多维度性能雷达对比
  // ----------------------------------------------------------
  function initRadarChart() {
    const dom = document.getElementById('chart-radar');
    if (!dom || !window.echarts) return;

    const chart = echarts.init(dom);

    const option = {
      backgroundColor: 'transparent',
      tooltip: {
        trigger: 'item',
        backgroundColor: 'rgba(255,255,255,0.95)',
        borderColor: colors.rule,
        borderWidth: 1,
        textStyle: { color: colors.ink, fontSize: 13 }
      },
      legend: {
        data: ['大融合单体', '完全模块化', '混合架构（推荐）'],
        bottom: 0,
        textStyle: { color: colors.ink, fontSize: 13 },
        itemWidth: 14,
        itemHeight: 14
      },
      radar: {
        indicator: [
          { name: '开发效率', max: 100 },
          { name: '运行性能', max: 100 },
          { name: '可维护性', max: 100 },
          { name: '可扩展性', max: 100 },
          { name: '故障隔离', max: 100 },
          { name: '用户体验', max: 100 },
          { name: '团队协作', max: 100 }
        ],
        center: ['50%', '45%'],
        radius: '62%',
        axisName: {
          color: colors.ink,
          fontSize: 13,
          fontWeight: 500
        },
        splitArea: {
          areaStyle: {
            color: ['rgba(99,102,241,0.02)', 'rgba(99,102,241,0.04)', 'rgba(99,102,241,0.06)', 'rgba(99,102,241,0.08)', 'rgba(99,102,241,0.10)']
          }
        },
        axisLine: {
          lineStyle: { color: colors.rule }
        },
        splitLine: {
          lineStyle: { color: colors.rule }
        }
      },
      series: [{
        type: 'radar',
        data: [
          {
            value: [85, 92, 45, 35, 25, 88, 40],
            name: '大融合单体',
            lineStyle: { color: colors.danger, width: 2 },
            areaStyle: { color: 'rgba(239, 68, 68, 0.12)' },
            itemStyle: { color: colors.danger }
          },
          {
            value: [55, 72, 88, 92, 90, 55, 90],
            name: '完全模块化',
            lineStyle: { color: colors.warning, width: 2 },
            areaStyle: { color: 'rgba(245, 158, 11, 0.12)' },
            itemStyle: { color: colors.warning }
          },
          {
            value: [78, 85, 90, 88, 85, 95, 92],
            name: '混合架构（推荐）',
            lineStyle: { color: colors.primary, width: 3 },
            areaStyle: { color: 'rgba(99, 102, 241, 0.20)' },
            itemStyle: { color: colors.primary }
          }
        ]
      }]
    };

    chart.setOption(option);
    window.addEventListener('resize', () => chart.resize());
  }

  // ----------------------------------------------------------
  // Chart 2: 关键性能指标目标
  // ----------------------------------------------------------
  function initMetricsChart() {
    const dom = document.getElementById('chart-metrics');
    if (!dom || !window.echarts) return;

    const chart = echarts.init(dom);

    const option = {
      backgroundColor: 'transparent',
      tooltip: {
        trigger: 'axis',
        axisPointer: { type: 'shadow' },
        backgroundColor: 'rgba(255,255,255,0.95)',
        borderColor: colors.rule,
        borderWidth: 1,
        textStyle: { color: colors.ink, fontSize: 13 }
      },
      legend: {
        data: ['当前基线', '目标值'],
        bottom: 0,
        textStyle: { color: colors.ink, fontSize: 13 },
        itemWidth: 14,
        itemHeight: 14
      },
      grid: {
        left: '3%',
        right: '4%',
        bottom: '12%',
        top: '5%',
        containLabel: true
      },
      xAxis: {
        type: 'category',
        data: [
          '图谱查询\n(P95)',
          '文档搜索\n(P95)',
          '专家匹配\n(P95)',
          '页面加载\n(首屏)',
          'API 可用性',
          '并发用户数'
        ],
        axisLabel: {
          color: colors.ink,
          fontSize: 12,
          interval: 0,
          lineHeight: 16
        },
        axisLine: { lineStyle: { color: colors.rule } },
        axisTick: { show: false }
      },
      yAxis: [
        {
          type: 'value',
          name: '延迟 (ms)',
          position: 'left',
          nameTextStyle: { color: colors.muted, fontSize: 12 },
          axisLabel: { color: colors.muted, fontSize: 12 },
          axisLine: { show: false },
          splitLine: { lineStyle: { color: colors.rule, type: 'dashed' } }
        },
        {
          type: 'value',
          name: '百分比 / 数量',
          position: 'right',
          nameTextStyle: { color: colors.muted, fontSize: 12 },
          axisLabel: { color: colors.muted, fontSize: 12 },
          axisLine: { show: false },
          splitLine: { show: false }
        }
      ],
      series: [
        {
          name: '当前基线',
          type: 'bar',
          barWidth: '30%',
          data: [
            { value: 450, itemStyle: { color: colors.warning } },
            { value: 800, itemStyle: { color: colors.warning } },
            { value: 1200, itemStyle: { color: colors.warning } },
            { value: 3500, itemStyle: { color: colors.warning } },
            { value: 95, itemStyle: { color: colors.warning } },
            { value: 200, itemStyle: { color: colors.warning } }
          ]
        },
        {
          name: '目标值',
          type: 'bar',
          barWidth: '30%',
          data: [
            { value: 150, itemStyle: { color: colors.success } },
            { value: 300, itemStyle: { color: colors.success } },
            { value: 500, itemStyle: { color: colors.success } },
            { value: 1500, itemStyle: { color: colors.success } },
            { value: 99.9, itemStyle: { color: colors.success } },
            { value: 1000, itemStyle: { color: colors.success } }
          ]
        }
      ]
    };

    chart.setOption(option);
    window.addEventListener('resize', () => chart.resize());
  }

  // ----------------------------------------------------------
  // Initialize all charts when DOM is ready
  // ----------------------------------------------------------
  function initAll() {
    initRadarChart();
    initMetricsChart();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initAll);
  } else {
    initAll();
  }
})();
