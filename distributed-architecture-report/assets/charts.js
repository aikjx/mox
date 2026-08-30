// Distributed Architecture Report - Charts
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

  var textStyle = { color: ink, fontFamily: 'InstrumentSans, ArsenalSC, sans-serif' };
  var axisLineStyle = { lineStyle: { color: rule } };
  var splitLineStyle = { lineStyle: { color: rule, type: 'dashed' } };

  // --- Chart 1: EC Comparison ---
  var ecChart = echarts.init(document.getElementById('chart-ec-comparison'), null, { renderer: 'svg' });
  ecChart.setOption({
    animation: false,
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      appendToBody: true,
      backgroundColor: bg2,
      borderColor: rule,
      textStyle: { color: ink }
    },
    legend: {
      data: ['存储开销率', '可容忍故障数', '编码效率'],
      textStyle: { color: muted },
      top: 0
    },
    grid: { left: '5%', right: '5%', bottom: '10%', top: '15%' },
    xAxis: {
      type: 'category',
      data: ['4+2', '6+3', '8+4', '10+4', '12+4', '16+4'],
      axisLine: axisLineStyle,
      axisLabel: { color: muted, fontSize: 12 },
      name: 'EC 配置（数据分片+校验分片）',
      nameTextStyle: { color: muted, fontSize: 12 }
    },
    yAxis: [
      {
        type: 'value',
        name: '存储开销率 %',
        nameTextStyle: { color: muted, fontSize: 11 },
        axisLine: axisLineStyle,
        axisLabel: { color: muted, fontSize: 11 },
        splitLine: splitLineStyle
      },
      {
        type: 'value',
        name: '故障数 / 相对效率',
        nameTextStyle: { color: muted, fontSize: 11 },
        axisLine: axisLineStyle,
        axisLabel: { color: muted, fontSize: 11 },
        splitLine: { show: false }
      }
    ],
    series: [
      {
        name: '存储开销率',
        type: 'bar',
        data: [50, 50, 50, 40, 33.3, 25],
        itemStyle: { color: accent },
        barWidth: '20%',
        label: { show: true, position: 'top', color: ink, fontSize: 11, formatter: '{c}%' }
      },
      {
        name: '可容忍故障数',
        type: 'line',
        yAxisIndex: 1,
        data: [2, 3, 4, 4, 4, 4],
        itemStyle: { color: accent2 },
        lineStyle: { width: 2 },
        symbol: 'circle',
        symbolSize: 8,
        label: { show: true, color: ink, fontSize: 11 }
      },
      {
        name: '编码效率',
        type: 'line',
        yAxisIndex: 1,
        data: [1.0, 0.95, 0.9, 0.92, 0.93, 0.95],
        itemStyle: { color: accent3 },
        lineStyle: { width: 2, type: 'dashed' },
        symbol: 'diamond',
        symbolSize: 8,
        label: { show: false }
      }
    ]
  });

  // --- Chart 2: KG Capacity ---
  var kgChart = echarts.init(document.getElementById('chart-kg-capacity'), null, { renderer: 'svg' });
  kgChart.setOption({
    animation: false,
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      appendToBody: true,
      backgroundColor: bg2,
      borderColor: rule,
      textStyle: { color: ink },
      formatter: function(params) {
        var result = params[0].axisValue + '<br/>';
        params.forEach(function(p) {
          result += p.marker + p.seriesName + ': ' + p.data + ' TB<br/>';
        });
        return result;
      }
    },
    legend: {
      data: ['实体存储', '关系存储', '索引开销', '总容量'],
      textStyle: { color: muted },
      top: 0
    },
    grid: { left: '5%', right: '5%', bottom: '10%', top: '15%' },
    xAxis: {
      type: 'category',
      data: ['10亿', '50亿', '100亿', '500亿', '1000亿'],
      axisLine: axisLineStyle,
      axisLabel: { color: muted, fontSize: 12 },
      name: '实体数量',
      nameTextStyle: { color: muted, fontSize: 12 }
    },
    yAxis: {
      type: 'value',
      name: '存储容量 (TB)',
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLine: axisLineStyle,
      axisLabel: { color: muted, fontSize: 11 },
      splitLine: splitLineStyle
    },
    series: [
      {
        name: '实体存储',
        type: 'bar',
        stack: 'total',
        data: [0.3, 1.5, 3, 15, 30],
        itemStyle: { color: accent }
      },
      {
        name: '关系存储',
        type: 'bar',
        stack: 'total',
        data: [1.2, 6, 12, 60, 120],
        itemStyle: { color: accent2 }
      },
      {
        name: '索引开销',
        type: 'bar',
        stack: 'total',
        data: [0.5, 2.5, 5, 25, 50],
        itemStyle: { color: accent3 }
      },
      {
        name: '总容量',
        type: 'line',
        data: [2, 10, 20, 100, 200],
        itemStyle: { color: '#fbbf24' },
        lineStyle: { width: 2 },
        symbol: 'circle',
        symbolSize: 6
      }
    ]
  });

  // --- Chart 3: Test Coverage ---
  var testChart = echarts.init(document.getElementById('chart-test-coverage'), null, { renderer: 'svg' });
  testChart.setOption({
    animation: false,
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      appendToBody: true,
      axisPointer: { type: 'shadow' },
      backgroundColor: bg2,
      borderColor: rule,
      textStyle: { color: ink }
    },
    legend: {
      data: ['通过', '失败'],
      textStyle: { color: muted },
      top: 0
    },
    grid: { left: '5%', right: '5%', bottom: '10%', top: '15%' },
    xAxis: {
      type: 'category',
      data: ['KG-Algo\n(算法核心)', 'KG-Storage\n(存储服务)', 'Cloud-Master\n(云盘主服务)', 'Cloud-Volume\n(云盘卷服务)'],
      axisLine: axisLineStyle,
      axisLabel: { color: muted, fontSize: 11, interval: 0 }
    },
    yAxis: {
      type: 'value',
      name: '测试数量',
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLine: axisLineStyle,
      axisLabel: { color: muted, fontSize: 11 },
      splitLine: splitLineStyle
    },
    series: [
      {
        name: '通过',
        type: 'bar',
        stack: 'total',
        data: [77, 87, 41, 61],
        itemStyle: { color: accent3 },
        barWidth: '35%',
        label: { show: true, position: 'inside', color: '#0f172a', fontSize: 12, fontWeight: 'bold' }
      },
      {
        name: '失败',
        type: 'bar',
        stack: 'total',
        data: [0, 0, 0, 0],
        itemStyle: { color: '#f87171' },
        label: { show: true, position: 'inside', color: '#0f172a', fontSize: 11 }
      }
    ]
  });

  // Resize listeners
  window.addEventListener('resize', function() {
    ecChart.resize();
    kgChart.resize();
    testChart.resize();
  });
})();
