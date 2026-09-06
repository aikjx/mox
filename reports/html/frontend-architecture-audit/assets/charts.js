(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();

  // --- Chart 1: 优化前后代码量对比 ---
  var chart1 = echarts.init(document.getElementById('chart-code-compare'), null, { renderer: 'svg' });
  chart1.setOption({
    animation: false,
    tooltip: { trigger: 'axis', appendToBody: true },
    legend: { data: ['优化前', '优化后'], top: 0, textStyle: { color: muted } },
    grid: { left: 60, right: 20, top: 40, bottom: 30 },
    xAxis: {
      type: 'category',
      data: ['视图层', '组件层', 'API层', '常量/类型', '状态管理'],
      axisLabel: { color: muted, fontSize: 12 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      name: '代码行数',
      nameTextStyle: { color: muted },
      axisLabel: { color: muted },
      splitLine: { lineStyle: { color: rule } }
    },
    series: [
      {
        name: '优化前',
        type: 'bar',
        data: [25050, 11000, 499, 212, 0],
        itemStyle: { color: muted + '66' },
        barWidth: '30%'
      },
      {
        name: '优化后',
        type: 'bar',
        data: [19050, 14643, 633, 201, 166],
        itemStyle: { color: accent },
        barWidth: '30%'
      }
    ]
  });
  window.addEventListener('resize', function() { chart1.resize(); });

  // --- Chart 2: 问题严重度分布 ---
  var chart2 = echarts.init(document.getElementById('chart-issues'), null, { renderer: 'svg' });
  chart2.setOption({
    animation: false,
    tooltip: { trigger: 'item', appendToBody: true },
    legend: { bottom: 0, textStyle: { color: muted } },
    series: [{
      type: 'pie',
      radius: ['40%', '70%'],
      center: ['50%', '45%'],
      avoidLabelOverlap: false,
      itemStyle: { borderRadius: 6, borderColor: bg2, borderWidth: 2 },
      label: { show: false },
      emphasis: { label: { show: true, fontSize: 14, fontWeight: 'bold' } },
      data: [
        { value: 1, name: 'P0 致命 - 重复代码', itemStyle: { color: '#ef4444' } },
        { value: 3, name: 'P1 严重 - 架构臃肿', itemStyle: { color: '#f59e0b' } },
        { value: 2, name: 'P2 中等 - 可维护性', itemStyle: { color: accent } },
        { value: 1, name: 'P3 轻微 - 规范性', itemStyle: { color: accent2 } }
      ]
    }]
  });
  window.addEventListener('resize', function() { chart2.resize(); });

  // --- Chart 3: 目录结构饼图 ---
  var chart3 = echarts.init(document.getElementById('chart-structure'), null, { renderer: 'svg' });
  chart3.setOption({
    animation: false,
    tooltip: { trigger: 'item', appendToBody: true, formatter: '{b}: {c} 个文件 ({d}%)' },
    legend: { bottom: 0, textStyle: { color: muted }, type: 'scroll' },
    series: [{
      type: 'pie',
      radius: '65%',
      center: ['50%', '42%'],
      itemStyle: { borderRadius: 4 },
      label: { fontSize: 11, color: ink },
      data: [
        { value: 35, name: 'views 视图层', itemStyle: { color: accent } },
        { value: 26, name: 'components 组件', itemStyle: { color: accent2 } },
        { value: 15, name: 'api 接口层', itemStyle: { color: '#10b981' } },
        { value: 5, name: 'constants 常量', itemStyle: { color: '#f59e0b' } },
        { value: 3, name: 'stores 状态', itemStyle: { color: '#8b5cf6' } },
        { value: 3, name: 'composables', itemStyle: { color: '#ec4899' } },
        { value: 6, name: 'styles 样式', itemStyle: { color: '#06b6d4' } }
      ]
    }]
  });
  window.addEventListener('resize', function() { chart3.resize(); });

  // --- Chart 4: 优化成果雷达图 ---
  var chart4 = echarts.init(document.getElementById('chart-radar'), null, { renderer: 'svg' });
  chart4.setOption({
    animation: false,
    tooltip: { appendToBody: true },
    radar: {
      indicator: [
        { name: '代码复用率', max: 100 },
        { name: '模块清晰度', max: 100 },
        { name: '可维护性', max: 100 },
        { name: '可扩展性', max: 100 },
        { name: '开发效率', max: 100 },
        { name: '代码规范性', max: 100 }
      ],
      axisName: { color: muted, fontSize: 12 },
      splitLine: { lineStyle: { color: rule } },
      splitArea: { areaStyle: { color: [bg2, 'transparent'] } },
      axisLine: { lineStyle: { color: rule } }
    },
    series: [{
      type: 'radar',
      data: [
        {
          value: [55, 45, 50, 55, 60, 50],
          name: '优化前',
          lineStyle: { color: muted },
          areaStyle: { color: muted + '33' },
          itemStyle: { color: muted }
        },
        {
          value: [85, 88, 82, 85, 78, 90],
          name: '优化后',
          lineStyle: { color: accent, width: 2 },
          areaStyle: { color: accent + '33' },
          itemStyle: { color: accent }
        }
      ]
    }],
    legend: { data: ['优化前', '优化后'], bottom: 0, textStyle: { color: muted } }
  });
  window.addEventListener('resize', function() { chart4.resize(); });
})();
