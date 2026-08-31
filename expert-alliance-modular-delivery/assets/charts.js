(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var accent3 = style.getPropertyValue('--accent3').trim();
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();
  var bg = style.getPropertyValue('--bg').trim();
  var warning = style.getPropertyValue('--warning').trim();
  var danger = style.getPropertyValue('--danger').trim();

  // === Chart: Graph Engines ===
  var chartEngines = echarts.init(document.getElementById('chart-graph-engines'), null, { renderer: 'svg' });
  chartEngines.setOption({
    animation: false,
    tooltip: {
      trigger: 'item',
      appendToBody: true,
      formatter: '{b}: {c} ({d}%)'
    },
    legend: {
      orient: 'vertical',
      right: '5%',
      top: 'center',
      textStyle: { color: muted },
      itemWidth: 12,
      itemHeight: 12
    },
    series: [{
      name: '图引擎类型',
      type: 'pie',
      radius: ['45%', '70%'],
      center: ['35%', '50%'],
      avoidLabelOverlap: false,
      itemStyle: {
        borderRadius: 6,
        borderColor: bg2,
        borderWidth: 2
      },
      label: {
        show: false
      },
      emphasis: {
        label: {
          show: true,
          fontSize: 14,
          fontWeight: 'bold',
          color: ink
        }
      },
      labelLine: {
        show: false
      },
      data: [
        { value: 45, name: 'RelGraph', itemStyle: { color: accent } },
        { value: 25, name: 'Neo4j', itemStyle: { color: accent2 } },
        { value: 20, name: 'InMemory', itemStyle: { color: accent3 } },
        { value: 10, name: 'Custom', itemStyle: { color: warning } }
      ]
    }]
  });

  // === Chart: Weights Comparison ===
  var chartWeights = echarts.init(document.getElementById('chart-weights'), null, { renderer: 'svg' });
  chartWeights.setOption({
    animation: false,
    tooltip: {
      trigger: 'axis',
      appendToBody: true,
      axisPointer: { type: 'shadow' },
      formatter: function(params) {
        var result = params[0].axisValue + '<br/>';
        params.forEach(function(p) {
          result += p.marker + p.seriesName + ': ' + (p.value * 100).toFixed(0) + '%<br/>';
        });
        return result;
      }
    },
    legend: {
      data: ['默认权重', '自定义权重(能力优先)'],
      textStyle: { color: muted },
      top: 0
    },
    grid: {
      left: '3%',
      right: '4%',
      bottom: '3%',
      top: '15%',
      containLabel: true
    },
    xAxis: {
      type: 'category',
      data: ['领域匹配', '能力匹配', '专家评分', '历史性能', '健康状态'],
      axisLine: { lineStyle: { color: rule } },
      axisLabel: { color: muted, fontSize: 12 }
    },
    yAxis: {
      type: 'value',
      max: 0.5,
      axisLabel: {
        color: muted,
        formatter: function(v) { return (v * 100).toFixed(0) + '%'; }
      },
      splitLine: { lineStyle: { color: rule } }
    },
    series: [
      {
        name: '默认权重',
        type: 'bar',
        barWidth: '30%',
        data: [0.35, 0.30, 0.20, 0.10, 0.05],
        itemStyle: {
          color: accent,
          borderRadius: [4, 4, 0, 0]
        }
      },
      {
        name: '自定义权重(能力优先)',
        type: 'bar',
        barWidth: '30%',
        data: [0.15, 0.50, 0.15, 0.15, 0.05],
        itemStyle: {
          color: accent2,
          borderRadius: [4, 4, 0, 0]
        }
      }
    ]
  });

  // === Chart: Fusion Strategies ===
  var chartFusion = echarts.init(document.getElementById('chart-fusion'), null, { renderer: 'svg' });

  var strategies = [
    '加权投票',
    '加权融合',
    '置信度加权',
    '择优融合',
    '拼接融合',
    '辩论融合',
    '迭代精炼',
    'Map-Reduce',
    '堆叠(元学习)'
  ];

  var scenarios = ['分类决策', '数值计算', '文本生成', '分析评估', '创意生成'];

  var heatData = [
    // [x, y, value] - 适用程度 0-5
    [0, 0, 5], [1, 0, 3], [2, 0, 4], [3, 0, 5], [4, 0, 2], // 加权投票
    [0, 1, 4], [1, 1, 5], [2, 1, 3], [3, 1, 4], [4, 1, 3], // 加权融合
    [0, 2, 4], [1, 2, 5], [2, 2, 4], [3, 2, 5], [4, 2, 3], // 置信度加权
    [0, 3, 3], [1, 3, 4], [2, 3, 5], [3, 3, 3], [4, 3, 5], // 择优融合
    [0, 4, 2], [1, 4, 2], [2, 4, 5], [3, 4, 4], [4, 4, 5], // 拼接融合
    [0, 5, 5], [1, 5, 2], [2, 5, 3], [3, 5, 5], [4, 5, 4], // 辩论融合
    [0, 6, 3], [1, 6, 4], [2, 6, 5], [3, 6, 4], [4, 6, 5], // 迭代精炼
    [0, 7, 3], [1, 7, 5], [2, 7, 3], [3, 7, 3], [4, 7, 2], // Map-Reduce
    [0, 8, 4], [1, 8, 5], [2, 8, 4], [3, 8, 4], [4, 8, 3], // 堆叠
  ];

  chartFusion.setOption({
    animation: false,
    tooltip: {
      position: 'top',
      appendToBody: true,
      formatter: function(p) {
        return strategies[p.data[1]] + '<br/>' + scenarios[p.data[0]] + ': ' + p.data[2] + '/5';
      }
    },
    grid: {
      left: '15%',
      right: '10%',
      top: '5%',
      bottom: '15%'
    },
    xAxis: {
      type: 'category',
      data: scenarios,
      splitArea: { show: false },
      axisLine: { lineStyle: { color: rule } },
      axisLabel: { color: muted, fontSize: 11, rotate: 0 }
    },
    yAxis: {
      type: 'category',
      data: strategies,
      splitArea: { show: false },
      axisLine: { lineStyle: { color: rule } },
      axisLabel: { color: muted, fontSize: 11 }
    },
    visualMap: {
      min: 0,
      max: 5,
      calculable: false,
      orient: 'horizontal',
      left: 'center',
      bottom: '0%',
      inRange: {
        color: [bg2, accent2, accent]
      },
      textStyle: { color: muted }
    },
    series: [{
      name: '适用程度',
      type: 'heatmap',
      data: heatData,
      label: {
        show: true,
        color: ink,
        fontSize: 11,
        formatter: function(p) {
          var stars = '';
          for (var i = 0; i < p.data[2]; i++) stars += '★';
          return stars;
        }
      },
      emphasis: {
        itemStyle: {
          shadowBlur: 10,
          shadowColor: 'rgba(0, 0, 0, 0.5)'
        }
      }
    }]
  });

  // Resize handlers
  window.addEventListener('resize', function() {
    chartEngines.resize();
    chartWeights.resize();
    chartFusion.resize();
  });
})();
