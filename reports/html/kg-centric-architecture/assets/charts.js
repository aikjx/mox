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

  // --- Chart: Self-Built Modules by Domain ---
  var chartSelfbuilt = echarts.init(document.getElementById('chart-selfbuilt'), null, { renderer: 'svg' });
  chartSelfbuilt.setOption({
    animation: false,
    tooltip: {
      appendToBody: true,
      trigger: 'item',
      formatter: function(params) {
        return params.name + '<br/>自研模块: <strong>' + params.value + '</strong> 个';
      }
    },
    legend: {
      orient: 'vertical',
      right: '5%',
      top: 'center',
      textStyle: { color: muted, fontSize: 12 },
      itemGap: 12
    },
    series: [{
      name: '自研模块分布',
      type: 'pie',
      radius: ['40%', '70%'],
      center: ['35%', '50%'],
      avoidLabelOverlap: true,
      itemStyle: {
        borderRadius: 6,
        borderColor: bg,
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
        },
        itemStyle: {
          shadowBlur: 10,
          shadowOffsetX: 0,
          shadowColor: 'rgba(100, 255, 218, 0.3)'
        }
      },
      labelLine: {
        show: false
      },
      data: [
        { value: 6, name: 'KG 知识图谱', itemStyle: { color: accent } },
        { value: 6, name: 'AI 智能', itemStyle: { color: accent2 } },
        { value: 6, name: 'Data 数据', itemStyle: { color: accent3 } },
        { value: 5, name: 'Cloud 云存储', itemStyle: { color: '#ffd43b' } },
        { value: 6, name: 'Flow 流程', itemStyle: { color: '#69db7c' } },
        { value: 8, name: 'Platform 平台', itemStyle: { color: '#ff8787' } },
        { value: 5, name: 'Voice 语音', itemStyle: { color: '#da77f2' } },
        { value: 2, name: 'Project 项目', itemStyle: { color: '#4dabf7' } }
      ]
    }]
  });

  window.addEventListener('resize', function() {
    chartSelfbuilt.resize();
  });
})();
