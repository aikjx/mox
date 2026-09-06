(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim() || '#4f46e5';
  var accent2 = style.getPropertyValue('--accent2').trim() || '#06b6d4';
  var ink = style.getPropertyValue('--ink').trim() || '#0f172a';
  var muted = style.getPropertyValue('--muted').trim() || '#64748b';
  var rule = style.getPropertyValue('--rule').trim() || '#e2e8f0';
  var success = '#10b981';
  var warning = '#f59e0b';

  // ===== 图1：各模块功能完整度评分 =====
  var chart1 = echarts.init(document.getElementById('chart-modules'), null, { renderer: 'svg' });
  chart1.setOption({
    animation: false,
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
      appendToBody: true
    },
    grid: { left: 100, right: 40, top: 20, bottom: 40 },
    xAxis: {
      type: 'value',
      max: 100,
      axisLabel: { formatter: '{value}%', color: muted },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } }
    },
    yAxis: {
      type: 'category',
      data: ['系统管理', '算子中心', '算子商城', '工作流编排', '知识库', '专家联盟', '知识图谱', 'AI 对话'],
      axisLabel: { color: ink, fontSize: 13 },
      axisLine: { lineStyle: { color: rule } },
      axisTick: { show: false }
    },
    series: [{
      type: 'bar',
      data: [
        { value: 95, itemStyle: { color: success } },
        { value: 85, itemStyle: { color: success } },
        { value: 88, itemStyle: { color: success } },
        { value: 90, itemStyle: { color: success } },
        { value: 92, itemStyle: { color: success } },
        { value: 96, itemStyle: { color: accent } },
        { value: 93, itemStyle: { color: success } },
        { value: 97, itemStyle: { color: accent } },
      ],
      barWidth: 18,
      itemStyle: {
        borderRadius: [0, 4, 4, 0]
      },
      label: {
        show: true,
        position: 'right',
        formatter: '{c}%',
        color: ink,
        fontWeight: 600,
        fontSize: 12
      }
    }]
  });

  window.addEventListener('resize', function() { chart1.resize(); });
})();
