(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--success').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var accent3 = style.getPropertyValue('--accent3').trim();
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();
  var bg = style.getPropertyValue('--bg').trim();
  var warning = style.getPropertyValue('--warning').trim();
  var danger = style.getPropertyValue('--danger').trim();

  var palette = [accent, accent2, accent3, warning, danger, '#ff8787', '#69db7c', '#4dabf7', '#da77f2', '#ffd43b'];

  // --- Chart: Lines by Domain ---
  var chartLines = echarts.init(document.getElementById('chart-lines'), null, { renderer: 'svg' });
  chartLines.setOption({
    animation: false,
    tooltip: {
      appendToBody: true,
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
      formatter: function(params) {
        var p = params[0];
        return p.name + '<br/>代码行数: <strong>' + p.value.toLocaleString() + '</strong>';
      }
    },
    grid: { left: 60, right: 20, top: 20, bottom: 60 },
    xAxis: {
      type: 'category',
      data: ['Platform', 'AI', 'KG', 'Flow', 'Cloud', 'Data', 'Voice', 'Project', 'Market'],
      axisLabel: { color: muted, fontSize: 11, rotate: 30 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      axisLabel: { color: muted, fontSize: 11, formatter: function(v) { return (v/1000) + 'K'; } },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } },
      axisLine: { show: false }
    },
    series: [{
      type: 'bar',
      data: [
        { value: 43221, itemStyle: { color: accent } },
        { value: 42943, itemStyle: { color: accent2 } },
        { value: 19681, itemStyle: { color: accent3 } },
        { value: 14160, itemStyle: { color: warning } },
        { value: 11160, itemStyle: { color: accent } },
        { value: 10373, itemStyle: { color: accent2 } },
        { value: 9012, itemStyle: { color: accent3 } },
        { value: 2258, itemStyle: { color: warning } },
        { value: 618, itemStyle: { color: danger } }
      ],
      barWidth: '50%',
      itemStyle: { borderRadius: [4, 4, 0, 0] }
    }]
  });

  // --- Chart: Crates count + full ratio ---
  var chartCrates = echarts.init(document.getElementById('chart-crates'), null, { renderer: 'svg' });
  chartCrates.setOption({
    animation: false,
    tooltip: {
      appendToBody: true,
      trigger: 'axis',
      axisPointer: { type: 'shadow' }
    },
    legend: {
      data: ['Crate 总数', '完整实现'],
      textStyle: { color: muted, fontSize: 11 },
      top: 0
    },
    grid: { left: 50, right: 20, top: 40, bottom: 60 },
    xAxis: {
      type: 'category',
      data: ['Platform', 'Data', 'KG', 'AI', 'Voice', 'Flow', 'Cloud', 'Project', 'Market'],
      axisLabel: { color: muted, fontSize: 11, rotate: 30 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      axisLabel: { color: muted, fontSize: 11 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } },
      axisLine: { show: false }
    },
    series: [
      {
        name: 'Crate 总数',
        type: 'bar',
        data: [18, 10, 10, 7, 8, 7, 6, 2, 2],
        itemStyle: { color: accent2 + '66', borderRadius: [4, 4, 0, 0] },
        barGap: '-100%',
        barWidth: '40%'
      },
      {
        name: '完整实现',
        type: 'bar',
        data: [2, 2, 4, 4, 1, 3, 2, 1, 0],
        itemStyle: { color: accent, borderRadius: [4, 4, 0, 0] },
        barWidth: '40%'
      }
    ]
  });

  // --- Chart: Test coverage by domain ---
  var chartTestCov = echarts.init(document.getElementById('chart-test-coverage'), null, { renderer: 'svg' });
  var domains = ['cloud', 'kg', 'flow', 'platform', 'data', 'ai', 'voice', 'market', 'project'];
  var testCov = [83, 50, 57, 44, 30, 29, 0, 0, 0];
  var covData = testCov.map(function(v, i) {
    return {
      value: v,
      itemStyle: {
        color: v >= 60 ? accent : (v >= 30 ? warning : danger)
      }
    };
  });

  chartTestCov.setOption({
    animation: false,
    tooltip: {
      appendToBody: true,
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
      formatter: function(params) {
        var p = params[0];
        return p.name + '<br/>测试覆盖率: <strong>' + p.value + '%</strong>';
      }
    },
    grid: { left: 60, right: 30, top: 20, bottom: 40 },
    xAxis: {
      type: 'value',
      max: 100,
      axisLabel: { color: muted, fontSize: 11, formatter: '{value}%' },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } },
      axisLine: { show: false }
    },
    yAxis: {
      type: 'category',
      data: domains,
      axisLabel: { color: muted, fontSize: 11 },
      axisLine: { lineStyle: { color: rule } }
    },
    series: [{
      type: 'bar',
      data: covData,
      barWidth: '55%',
      itemStyle: { borderRadius: [0, 4, 4, 0] },
      label: {
        show: true,
        position: 'right',
        color: ink,
        fontSize: 11,
        fontWeight: 600,
        formatter: '{c}%'
      }
    }]
  });

  window.addEventListener('resize', function() {
    chartLines.resize();
    chartCrates.resize();
    chartTestCov.resize();
  });
})();
