(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();
  var bg3 = style.getPropertyValue('--bg3').trim();
  var warn = style.getPropertyValue('--warn').trim();
  var danger = style.getPropertyValue('--danger').trim();
  var success = style.getPropertyValue('--success').trim();

  // --- Chart 1: 顶层目录类别分布（饼图）---
  var chart1 = echarts.init(document.getElementById('chart-category'), null, { renderer: 'svg' });
  chart1.setOption({
    animation: false,
    tooltip: {
      trigger: 'item',
      appendToBody: true,
      formatter: '{b}: {c} 个 ({d}%)'
    },
    legend: {
      orient: 'vertical',
      right: '5%',
      top: 'center',
      textStyle: { color: ink, fontSize: 13 },
      itemWidth: 14,
      itemHeight: 14,
      itemGap: 12
    },
    color: [accent, accent2, warn, danger, success, muted],
    series: [
      {
        name: '目录类别',
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
          { value: 6, name: '核心源码 (platform/frontend-ui)' },
          { value: 7, name: 'HTML 原型项目' },
          { value: 4, name: '文档与规格' },
          { value: 1, name: '第三方代码 (ais/)' },
          { value: 5, name: '运行时/临时文件' },
          { value: 6, name: '配置/部署/工具' }
        ]
      }
    ]
  });
  window.addEventListener('resize', function() { chart1.resize(); });

  // --- Chart 2: 各阶段收益 vs 风险评估（雷达图）---
  var chart2 = echarts.init(document.getElementById('chart-phases'), null, { renderer: 'svg' });
  chart2.setOption({
    animation: false,
    tooltip: {
      trigger: 'item',
      appendToBody: true
    },
    legend: {
      data: ['阶段一：快速清理', '阶段二：结构归并', '阶段三：深度治理'],
      textStyle: { color: ink, fontSize: 13 },
      bottom: 0,
      itemWidth: 14,
      itemHeight: 14
    },
    radar: {
      indicator: [
        { name: '目录整洁度提升', max: 100 },
        { name: '实施风险（反向）', max: 100 },
        { name: '工作量（反向）', max: 100 },
        { name: '长期收益', max: 100 },
        { name: '可逆性', max: 100 }
      ],
      radius: '65%',
      center: ['50%', '45%'],
      axisName: {
        color: ink,
        fontSize: 12
      },
      splitArea: {
        areaStyle: {
          color: [bg2, bg3]
        }
      },
      axisLine: {
        lineStyle: { color: rule }
      },
      splitLine: {
        lineStyle: { color: rule }
      }
    },
    series: [
      {
        type: 'radar',
        data: [
          {
            value: [50, 90, 95, 30, 95],
            name: '阶段一：快速清理',
            lineStyle: { color: success, width: 2 },
            itemStyle: { color: success },
            areaStyle: { color: success + '20' }
          },
          {
            value: [80, 55, 50, 70, 70],
            name: '阶段二：结构归并',
            lineStyle: { color: accent, width: 2 },
            itemStyle: { color: accent },
            areaStyle: { color: accent + '20' }
          },
          {
            value: [95, 20, 15, 95, 30],
            name: '阶段三：深度治理',
            lineStyle: { color: warn, width: 2 },
            itemStyle: { color: warn },
            areaStyle: { color: warn + '20' }
          }
        ]
      }
    ]
  });
  window.addEventListener('resize', function() { chart2.resize(); });
})();
