(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var success = '#10b981';
  var warning = '#f59e0b';
  var danger = '#ef4444';
  var info = '#06b6d4';
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();

  // --- Chart 1: 模块测试通过率 ---
  var chart1 = echarts.init(document.getElementById('chart-pass-rate'), null, { renderer: 'svg' });
  chart1.setOption({
    animation: false,
    tooltip: { trigger: 'axis', appendToBody: true, axisPointer: { type: 'shadow' } },
    legend: { data: ['通过', '部分通过', '未通过'], textStyle: { color: muted, fontSize: 12 }, bottom: 0 },
    grid: { left: '3%', right: '4%', bottom: '15%', top: '10%', containLabel: true },
    xAxis: {
      type: 'category',
      data: ['SQL控制台', '数据管理', '知识图谱', '知识库云盘', '专家联盟', '监控告警', '权限管理', '审计日志', '系统设置', '数据看板'],
      axisLabel: { color: muted, fontSize: 11, rotate: 20 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      max: 100,
      axisLabel: { color: muted, fontSize: 11, formatter: '{value}%' },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } }
    },
    series: [
      { name: '通过', type: 'bar', stack: 'total', data: [14, 12, 15, 14, 13, 9, 13, 12, 7, 8], itemStyle: { color: success }, barWidth: '50%' },
      { name: '部分通过', type: 'bar', stack: 'total', data: [4, 3, 6, 5, 8, 5, 4, 3, 4, 3], itemStyle: { color: warning } },
      { name: '未通过', type: 'bar', stack: 'total', data: [2, 2, 4, 3, 7, 5, 2, 3, 3, 2], itemStyle: { color: danger } }
    ]
  });
  window.addEventListener('resize', function() { chart1.resize(); });

  // --- Chart 2: 测试用例分布饼图 ---
  var chart2 = echarts.init(document.getElementById('chart-case-dist'), null, { renderer: 'svg' });
  chart2.setOption({
    animation: false,
    tooltip: { trigger: 'item', appendToBody: true, formatter: '{b}: {c} 个 ({d}%)' },
    legend: { orient: 'vertical', right: '5%', top: 'center', textStyle: { color: muted, fontSize: 12 } },
    series: [{
      type: 'pie',
      radius: ['45%', '70%'],
      center: ['35%', '50%'],
      avoidLabelOverlap: false,
      itemStyle: { borderRadius: 6, borderColor: bg2, borderWidth: 2 },
      label: { show: false },
      data: [
        { value: 20, name: 'SQL控制台', itemStyle: { color: accent } },
        { value: 17, name: '数据管理', itemStyle: { color: info } },
        { value: 25, name: '知识图谱', itemStyle: { color: success } },
        { value: 22, name: '知识库云盘', itemStyle: { color: warning } },
        { value: 28, name: '专家联盟', itemStyle: { color: '#8b5cf6' } },
        { value: 19, name: '监控告警', itemStyle: { color: danger } },
        { value: 19, name: '权限管理', itemStyle: { color: '#ec4899' } },
        { value: 18, name: '审计日志', itemStyle: { color: '#14b8a6' } },
        { value: 14, name: '系统设置', itemStyle: { color: '#f97316' } },
        { value: 13, name: '数据看板', itemStyle: { color: '#84cc16' } }
      ]
    }]
  });
  window.addEventListener('resize', function() { chart2.resize(); });

  // --- Chart 3: 缺陷严重程度趋势 ---
  var chart3 = echarts.init(document.getElementById('chart-bug-trend'), null, { renderer: 'svg' });
  chart3.setOption({
    animation: false,
    tooltip: { trigger: 'axis', appendToBody: true },
    legend: { data: ['严重', '一般', '轻微'], textStyle: { color: muted, fontSize: 12 }, top: 0 },
    grid: { left: '3%', right: '4%', bottom: '3%', top: '18%', containLabel: true },
    xAxis: {
      type: 'category',
      data: ['D1', 'D2', 'D3', 'D4', 'D5', 'D6', 'D7'],
      axisLabel: { color: muted, fontSize: 11 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      axisLabel: { color: muted, fontSize: 11 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } }
    },
    series: [
      { name: '严重', type: 'line', data: [8, 5, 3, 2, 1, 0, 0], itemStyle: { color: danger }, lineStyle: { width: 2 }, symbol: 'circle', symbolSize: 6 },
      { name: '一般', type: 'line', data: [15, 12, 10, 7, 5, 3, 2], itemStyle: { color: warning }, lineStyle: { width: 2 }, symbol: 'circle', symbolSize: 6 },
      { name: '轻微', type: 'line', data: [22, 18, 14, 10, 8, 5, 3], itemStyle: { color: info }, lineStyle: { width: 2 }, symbol: 'circle', symbolSize: 6 }
    ]
  });
  window.addEventListener('resize', function() { chart3.resize(); });

  // --- Chart 4: 性能指标雷达图 ---
  var chart4 = echarts.init(document.getElementById('chart-perf-radar'), null, { renderer: 'svg' });
  chart4.setOption({
    animation: false,
    tooltip: { appendToBody: true },
    radar: {
      indicator: [
        { name: '响应速度', max: 100 },
        { name: '并发能力', max: 100 },
        { name: '稳定性', max: 100 },
        { name: '资源占用', max: 100 },
        { name: '可扩展性', max: 100 },
        { name: '容错能力', max: 100 }
      ],
      axisName: { color: muted, fontSize: 11 },
      splitLine: { lineStyle: { color: rule } },
      splitArea: { areaStyle: { color: [bg2, 'transparent'] } },
      axisLine: { lineStyle: { color: rule } }
    },
    series: [{
      type: 'radar',
      data: [
        { value: [88, 82, 95, 78, 90, 85], name: '当前版本', itemStyle: { color: accent }, areaStyle: { opacity: 0.2 }, lineStyle: { width: 2 } },
        { value: [70, 65, 80, 72, 75, 70], name: '基线版本', itemStyle: { color: muted }, areaStyle: { opacity: 0.1 }, lineStyle: { width: 1, type: 'dashed' } }
      ]
    }]
  });
  window.addEventListener('resize', function() { chart4.resize(); });

  // --- Chart 5: 缺陷模块分布 ---
  var chart5 = echarts.init(document.getElementById('chart-bug-module'), null, { renderer: 'svg' });
  chart5.setOption({
    animation: false,
    tooltip: { trigger: 'axis', appendToBody: true, axisPointer: { type: 'shadow' } },
    grid: { left: '3%', right: '8%', bottom: '3%', top: '5%', containLabel: true },
    xAxis: {
      type: 'value',
      axisLabel: { color: muted, fontSize: 11 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } }
    },
    yAxis: {
      type: 'category',
      data: ['系统设置', '数据看板', '审计日志', '权限管理', '监控告警', '专家联盟', '知识库云盘', '知识图谱', '数据管理', 'SQL控制台'],
      axisLabel: { color: muted, fontSize: 11 },
      axisLine: { lineStyle: { color: rule } }
    },
    series: [{
      type: 'bar',
      data: [7, 6, 7, 7, 10, 15, 10, 13, 8, 7],
      itemStyle: {
        color: function(params) {
          var colors = [info, info, warning, warning, warning, danger, warning, danger, warning, info];
          return colors[params.dataIndex];
        },
        borderRadius: [0, 4, 4, 0]
      },
      barWidth: '55%',
      label: { show: true, position: 'right', color: muted, fontSize: 11 }
    }]
  });
  window.addEventListener('resize', function() { chart5.resize(); });

  // --- Chart 6: 测试执行进度 ---
  var chart6 = echarts.init(document.getElementById('chart-progress'), null, { renderer: 'svg' });
  chart6.setOption({
    animation: false,
    tooltip: { appendToBody: true, formatter: '{b}: {c}%' },
    series: [{
      type: 'gauge',
      startAngle: 200,
      endAngle: -20,
      min: 0,
      max: 100,
      splitNumber: 10,
      itemStyle: { color: accent },
      progress: { show: true, width: 20 },
      pointer: { show: false },
      axisLine: { lineStyle: { width: 20, color: [[1, bg2]] } },
      axisTick: { show: false },
      splitLine: { show: false },
      axisLabel: { show: false },
      title: { show: false },
      detail: {
        valueAnimation: false,
        fontSize: 32,
        fontWeight: 'bold',
        color: ink,
        offsetCenter: [0, '0%'],
        formatter: '{value}%'
      },
      data: [{ value: 86, name: '测试完成率' }]
    }, {
      type: 'gauge',
      startAngle: 200,
      endAngle: -20,
      min: 0,
      max: 100,
      itemStyle: { color: success },
      progress: { show: true, width: 8 },
      pointer: { show: false },
      axisLine: { show: false },
      axisTick: { show: false },
      splitLine: { show: false },
      axisLabel: { show: false },
      detail: { show: false },
      data: [{ value: 75, name: '功能覆盖率' }]
    }]
  });
  window.addEventListener('resize', function() { chart6.resize(); });

})();
