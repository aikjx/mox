(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim() || '#4f46e5';
  var accent2 = style.getPropertyValue('--accent2').trim() || '#06b6d4';
  var ink = style.getPropertyValue('--ink').trim() || '#0f172a';
  var muted = style.getPropertyValue('--muted').trim() || '#64748b';
  var rule = style.getPropertyValue('--rule').trim() || '#e2e8f0';
  var bg2 = style.getPropertyValue('--bg2').trim() || '#f8fafc';
  var success = '#10b981';
  var warning = '#f59e0b';
  var danger = '#ef4444';

  // --- Chart 1: 架构成熟度雷达图 ---
  var chart1 = echarts.init(document.getElementById('chart-maturity'), null, { renderer: 'svg' });
  chart1.setOption({
    animation: false,
    tooltip: { appendToBody: true },
    radar: {
      indicator: [
        { name: '架构设计', max: 100 },
        { name: '代码质量', max: 100 },
        { name: '测试覆盖', max: 100 },
        { name: '可观测性', max: 100 },
        { name: '安全合规', max: 100 },
        { name: '部署运维', max: 100 },
        { name: '文档体系', max: 100 },
        { name: '性能优化', max: 100 }
      ],
      radius: '65%',
      axisName: { color: muted, fontSize: 12 },
      splitLine: { lineStyle: { color: rule } },
      splitArea: { areaStyle: { color: ['transparent', bg2] } },
      axisLine: { lineStyle: { color: rule } }
    },
    series: [{
      type: 'radar',
      data: [{
        value: [78, 65, 42, 35, 55, 48, 82, 60],
        name: '当前成熟度',
        areaStyle: { color: accent + '22' },
        lineStyle: { color: accent, width: 2 },
        itemStyle: { color: accent }
      }, {
        value: [92, 88, 85, 80, 90, 85, 90, 88],
        name: '企业级目标',
        areaStyle: { color: success + '15' },
        lineStyle: { color: success, width: 2, type: 'dashed' },
        itemStyle: { color: success }
      }]
    }],
    legend: {
      data: ['当前成熟度', '企业级目标'],
      bottom: 0,
      textStyle: { color: muted, fontSize: 12 }
    }
  });
  window.addEventListener('resize', function() { chart1.resize(); });

  // --- Chart 2: 业务域完成度 ---
  var chart2 = echarts.init(document.getElementById('chart-domain'), null, { renderer: 'svg' });
  chart2.setOption({
    animation: false,
    tooltip: { appendToBody: true, formatter: '{b}: {c}%' },
    grid: { left: 100, right: 40, top: 20, bottom: 20 },
    xAxis: {
      type: 'value',
      max: 100,
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: muted, formatter: '{value}%' },
      splitLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'category',
      data: ['专家联盟', '算子引擎', '工作流引擎', 'AI 对话', '知识图谱', '项目管理', '资源中心', '系统管理'],
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: ink, fontSize: 13 }
    },
    series: [{
      type: 'bar',
      data: [
        { value: 72, itemStyle: { color: accent2 } },
        { value: 68, itemStyle: { color: accent2 } },
        { value: 65, itemStyle: { color: accent2 } },
        { value: 78, itemStyle: { color: accent } },
        { value: 85, itemStyle: { color: accent } },
        { value: 70, itemStyle: { color: accent2 } },
        { value: 60, itemStyle: { color: accent2 } },
        { value: 55, itemStyle: { color: accent2 } }
      ],
      barWidth: 20,
      label: {
        show: true,
        position: 'right',
        formatter: '{c}%',
        color: muted,
        fontSize: 12
      }
    }]
  });
  window.addEventListener('resize', function() { chart2.resize(); });

  // --- Chart 3: 优化优先级矩阵 ---
  var chart3 = echarts.init(document.getElementById('chart-priority'), null, { renderer: 'svg' });
  chart3.setOption({
    animation: false,
    tooltip: {
      appendToBody: true,
      formatter: function(params) {
        return params.data.name + '<br/>影响: ' + params.data.value[0] + '<br/>难度: ' + params.data.value[1];
      }
    },
    grid: { left: 60, right: 40, top: 40, bottom: 50 },
    xAxis: {
      type: 'value',
      name: '实施难度 →',
      nameTextStyle: { color: muted },
      min: 0,
      max: 100,
      axisLine: { lineStyle: { color: rule } },
      axisLabel: { color: muted },
      splitLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      name: '业务影响 ↑',
      nameTextStyle: { color: muted },
      min: 0,
      max: 100,
      axisLine: { lineStyle: { color: rule } },
      axisLabel: { color: muted },
      splitLine: { lineStyle: { color: rule } }
    },
    series: [{
      type: 'scatter',
      symbolSize: function(data) { return data[2] * 2; },
      data: [
        // [难度, 影响, 大小, 名称]
        { value: [30, 95, 50], name: 'API 契约统一', itemStyle: { color: success } },
        { value: [40, 90, 45], name: '统一错误码体系', itemStyle: { color: success } },
        { value: [50, 85, 42], name: '可观测性体系', itemStyle: { color: success } },
        { value: [60, 92, 48], name: 'CI/CD 流水线', itemStyle: { color: success } },
        { value: [45, 80, 40], name: 'RBAC 权限体系', itemStyle: { color: accent } },
        { value: [55, 75, 38], name: '微服务拆分', itemStyle: { color: accent } },
        { value: [70, 88, 45], name: '多租户架构', itemStyle: { color: accent } },
        { value: [65, 70, 35], name: '服务网格', itemStyle: { color: warning } },
        { value: [80, 85, 42], name: '分布式事务', itemStyle: { color: warning } },
        { value: [85, 75, 35], name: '多活容灾', itemStyle: { color: warning } },
        { value: [35, 60, 30], name: '前端性能优化', itemStyle: { color: accent2 } },
        { value: [25, 55, 28], name: '代码规范统一', itemStyle: { color: accent2 } }
      ],
      label: {
        show: true,
        formatter: function(params) { return params.data.name; },
        position: 'top',
        color: ink,
        fontSize: 11
      }
    }],
    markLine: {
      silent: true,
      symbol: 'none',
      lineStyle: { color: rule, type: 'dashed' },
      data: [
        { xAxis: 50 },
        { yAxis: 70 }
      ]
    }
  });
  window.addEventListener('resize', function() { chart3.resize(); });

  // --- Chart 4: 技术栈分布 ---
  var chart4 = echarts.init(document.getElementById('chart-techstack'), null, { renderer: 'svg' });
  chart4.setOption({
    animation: false,
    tooltip: { appendToBody: true, trigger: 'item', formatter: '{b}: {c} ({d}%)' },
    legend: {
      orient: 'vertical',
      right: 20,
      top: 'center',
      textStyle: { color: muted, fontSize: 12 }
    },
    series: [{
      type: 'pie',
      radius: ['45%', '70%'],
      center: ['35%', '50%'],
      avoidLabelOverlap: true,
      itemStyle: { borderRadius: 6, borderColor: '#fff', borderWidth: 2 },
      label: { show: false },
      data: [
        { value: 35, name: 'Rust 后端', itemStyle: { color: accent } },
        { value: 25, name: 'Vue 3 前端', itemStyle: { color: accent2 } },
        { value: 15, name: 'Node.js 服务', itemStyle: { color: success } },
        { value: 10, name: 'RocksDB 存储', itemStyle: { color: warning } },
        { value: 8, name: 'Python 脚本', itemStyle: { color: '#8b5cf6' } },
        { value: 7, name: '其他', itemStyle: { color: muted } }
      ]
    }]
  });
  window.addEventListener('resize', function() { chart4.resize(); });

  // --- Chart 5: 优化路线图甘特 ---
  var chart5 = echarts.init(document.getElementById('chart-roadmap'), null, { renderer: 'svg' });
  var phases = ['P0 基础加固', 'P1 架构优化', 'P2 企业能力', 'P3 规模创新'];
  var tasks = [
    { name: 'API 契约与错误码统一', phase: 0, start: 0, end: 20 },
    { name: '代码规范与 Lint 体系', phase: 0, start: 5, end: 25 },
    { name: 'CI/CD 流水线建设', phase: 0, start: 10, end: 35 },
    { name: '测试覆盖提升至 70%', phase: 0, start: 15, end: 45 },
    { name: '可观测性体系（日志/指标/追踪）', phase: 1, start: 30, end: 60 },
    { name: '领域模型归一化', phase: 1, start: 35, end: 65 },
    { name: '网关与服务治理', phase: 1, start: 45, end: 75 },
    { name: 'RBAC 权限与审计', phase: 2, start: 60, end: 85 },
    { name: '多租户架构', phase: 2, start: 70, end: 95 },
    { name: '高可用与容灾', phase: 2, start: 80, end: 110 },
    { name: '微服务拆分演进', phase: 3, start: 90, end: 120 },
    { name: 'AI Agent 编排升级', phase: 3, start: 100, end: 130 }
  ];
  var phaseColors = [accent, accent2, success, warning];
  var ganttData = tasks.map(function(t, i) {
    return {
      value: [i, t.start, t.end, t.phase],
      itemStyle: { color: phaseColors[t.phase] },
      name: t.name
    };
  });

  chart5.setOption({
    animation: false,
    tooltip: {
      appendToBody: true,
      formatter: function(params) {
        return params.data.name + '<br/>阶段: ' + phases[params.data.value[3]] + '<br/>工期: ' + (params.data.value[2] - params.data.value[1]) + ' 天';
      }
    },
    grid: { left: 180, right: 40, top: 40, bottom: 40 },
    xAxis: {
      type: 'value',
      name: '工期（天）',
      nameTextStyle: { color: muted },
      max: 140,
      axisLine: { lineStyle: { color: rule } },
      axisLabel: { color: muted },
      splitLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'category',
      data: tasks.map(function(t) { return t.name; }),
      inverse: true,
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: ink, fontSize: 12 }
    },
    series: [{
      type: 'custom',
      renderItem: function(params, api) {
        var categoryIndex = api.value(0);
        var start = api.coord([api.value(1), categoryIndex]);
        var end = api.coord([api.value(2), categoryIndex]);
        var height = api.size([0, 1])[1] * 0.6;
        return {
          type: 'rect',
          shape: {
            x: start[0],
            y: start[1] - height / 2,
            width: end[0] - start[0],
            height: height
          },
          style: {
            fill: phaseColors[api.value(3)],
            opacity: 0.85
          }
        };
      },
      data: ganttData,
      markLine: {
        silent: true,
        symbol: 'none',
        lineStyle: { color: danger, type: 'dashed', width: 2 },
        data: [{ xAxis: 45, label: { formatter: 'Q1 交付', position: 'end', color: danger } }],
        label: { color: danger, fontSize: 11 }
      }
    }]
  });
  window.addEventListener('resize', function() { chart5.resize(); });

})();
