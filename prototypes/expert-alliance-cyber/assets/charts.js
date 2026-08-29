(function() {
  // Get CSS variables for theme consistency
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim() || '#00D4FF';
  var accent2 = style.getPropertyValue('--accent2').trim() || '#B14AFF';
  var accent3 = style.getPropertyValue('--accent3').trim() || '#00FFC8';
  var warning = style.getPropertyValue('--warning').trim() || '#FFAA00';
  var danger = style.getPropertyValue('--danger').trim() || '#FF2D55';
  var ink = style.getPropertyValue('--ink').trim() || '#E8F4FF';
  var muted = style.getPropertyValue('--ink-secondary').trim() || '#7A9BB8';
  var rule = style.getPropertyValue('--rule').trim() || '#1A3050';
  var bg2 = style.getPropertyValue('--bg2').trim() || '#0B1220';
  var inkSecondary = style.getPropertyValue('--ink-secondary').trim() || '#7A9BB8';

  var palette = [accent, accent2, accent3, warning, danger, accent + '99', accent2 + '99'];

  // Common chart text styles for dark theme
  var textStyle = {
    color: inkSecondary,
    fontFamily: "'WorkSans', 'PingFang SC', sans-serif",
    fontSize: 12
  };

  var titleStyle = {
    color: ink,
    fontFamily: "'Tektur', 'WorkSans', sans-serif",
    fontWeight: 500,
    fontSize: 14,
    letterSpacing: 1
  };

  // ========== Chart 1: User Roles Distribution ==========
  var chartRolesEl = document.getElementById('chart-roles');
  if (chartRolesEl) {
    var chartRoles = echarts.init(chartRolesEl, null, { renderer: 'svg' });
    chartRoles.setOption({
      animation: false,
      tooltip: {
        trigger: 'item',
        appendToBody: true,
        formatter: '{b}: {c}% ({d}%)',
        backgroundColor: 'rgba(15, 26, 46, 0.95)',
        borderColor: accent,
        borderWidth: 1,
        textStyle: { color: ink, fontSize: 12 }
      },
      legend: {
        orient: 'vertical',
        right: 20,
        top: 'center',
        textStyle: textStyle,
        itemWidth: 12,
        itemHeight: 12,
        itemGap: 16
      },
      series: [{
        type: 'pie',
        radius: ['45%', '70%'],
        center: ['35%', '50%'],
        avoidLabelOverlap: true,
        itemStyle: {
          borderColor: bg2,
          borderWidth: 3
        },
        label: {
          show: false
        },
        emphasis: {
          label: {
            show: true,
            color: ink,
            fontSize: 13,
            fontWeight: 600
          },
          itemStyle: {
            shadowBlur: 20,
            shadowColor: 'rgba(0, 212, 255, 0.5)'
          }
        },
        labelLine: { show: false },
        data: [
          { value: 40, name: '专家顾问', itemStyle: { color: accent } },
          { value: 35, name: '企业需求方', itemStyle: { color: accent2 } },
          { value: 10, name: '内容消费者', itemStyle: { color: accent3 } },
          { value: 8, name: '机构合伙人', itemStyle: { color: warning } },
          { value: 5, name: '平台运营', itemStyle: { color: danger } },
          { value: 2, name: '其他角色', itemStyle: { color: muted } }
        ]
      }]
    });
    window.addEventListener('resize', function() { chartRoles.resize(); });
  }

  // ========== Chart 2: Features Radar ==========
  var chartFeaturesEl = document.getElementById('chart-features');
  if (chartFeaturesEl) {
    var chartFeatures = echarts.init(chartFeaturesEl, null, { renderer: 'svg' });
    chartFeatures.setOption({
      animation: false,
      tooltip: {
        trigger: 'item',
        backgroundColor: 'rgba(15, 26, 46, 0.95)',
        borderColor: accent,
        borderWidth: 1,
        textStyle: { color: ink, fontSize: 12 }
      },
      legend: {
        data: ['重要性', '成熟度'],
        bottom: 10,
        textStyle: textStyle,
        itemWidth: 16,
        itemHeight: 8
      },
      radar: {
        indicator: [
          { name: '智能匹配', max: 100 },
          { name: '专家档案', max: 100 },
          { name: '在线协作', max: 100 },
          { name: '订单支付', max: 100 },
          { name: '信用评价', max: 100 },
          { name: '内容社区', max: 100 },
          { name: '数据智能', max: 100 },
          { name: '运营后台', max: 100 }
        ],
        center: ['50%', '45%'],
        radius: '65%',
        axisName: {
          color: inkSecondary,
          fontSize: 12,
          fontFamily: "'WorkSans', sans-serif"
        },
        splitLine: {
          lineStyle: { color: rule }
        },
        splitArea: {
          areaStyle: {
            color: ['rgba(0, 212, 255, 0.02)', 'rgba(0, 212, 255, 0.04)']
          }
        },
        axisLine: {
          lineStyle: { color: rule }
        }
      },
      series: [{
        type: 'radar',
        data: [
          {
            value: [95, 90, 80, 85, 88, 70, 75, 82],
            name: '重要性',
            lineStyle: { color: accent, width: 2 },
            areaStyle: {
              color: {
                type: 'radial',
                x: 0.5, y: 0.5, r: 0.5,
                colorStops: [
                  { offset: 0, color: 'rgba(0, 212, 255, 0.4)' },
                  { offset: 1, color: 'rgba(0, 212, 255, 0.05)' }
                ]
              }
            },
            itemStyle: { color: accent }
          },
          {
            value: [70, 85, 60, 80, 75, 50, 55, 65],
            name: '成熟度',
            lineStyle: { color: accent2, width: 2, type: 'dashed' },
            areaStyle: {
              color: {
                type: 'radial',
                x: 0.5, y: 0.5, r: 0.5,
                colorStops: [
                  { offset: 0, color: 'rgba(177, 74, 255, 0.3)' },
                  { offset: 1, color: 'rgba(177, 74, 255, 0.03)' }
                ]
              }
            },
            itemStyle: { color: accent2 }
          }
        ]
      }]
    });
    window.addEventListener('resize', function() { chartFeatures.resize(); });
  }

  // ========== Chart 3: Expert Profile Radar ==========
  var chartExpertEl = document.getElementById('chart-expert');
  if (chartExpertEl) {
    var chartExpert = echarts.init(chartExpertEl, null, { renderer: 'svg' });
    chartExpert.setOption({
      animation: false,
      tooltip: {
        trigger: 'item',
        backgroundColor: 'rgba(15, 26, 46, 0.95)',
        borderColor: accent,
        borderWidth: 1,
        textStyle: { color: ink, fontSize: 12 }
      },
      legend: {
        data: ['专家A', '专家B', '行业平均'],
        bottom: 10,
        textStyle: textStyle,
        itemWidth: 16,
        itemHeight: 8
      },
      radar: {
        indicator: [
          { name: '专业能力', max: 100 },
          { name: '响应速度', max: 100 },
          { name: '交付质量', max: 100 },
          { name: '沟通能力', max: 100 },
          { name: '准时率', max: 100 },
          { name: '性价比', max: 100 }
        ],
        center: ['50%', '48%'],
        radius: '65%',
        axisName: {
          color: inkSecondary,
          fontSize: 12,
          fontFamily: "'WorkSans', sans-serif"
        },
        splitLine: {
          lineStyle: { color: rule }
        },
        splitArea: {
          areaStyle: {
            color: ['rgba(0, 212, 255, 0.02)', 'rgba(0, 212, 255, 0.04)']
          }
        },
        axisLine: {
          lineStyle: { color: rule }
        }
      },
      series: [{
        type: 'radar',
        data: [
          {
            value: [95, 88, 92, 85, 96, 78],
            name: '专家A',
            lineStyle: { color: accent, width: 2 },
            areaStyle: {
              color: 'rgba(0, 212, 255, 0.25)'
            },
            itemStyle: { color: accent }
          },
          {
            value: [82, 95, 88, 92, 85, 90],
            name: '专家B',
            lineStyle: { color: accent2, width: 2 },
            areaStyle: {
              color: 'rgba(177, 74, 255, 0.2)'
            },
            itemStyle: { color: accent2 }
          },
          {
            value: [75, 70, 72, 78, 73, 76],
            name: '行业平均',
            lineStyle: { color: muted, width: 1, type: 'dashed' },
            areaStyle: { color: 'transparent' },
            itemStyle: { color: muted }
          }
        ]
      }]
    });
    window.addEventListener('resize', function() { chartExpert.resize(); });
  }

  // ========== Chart 4: Expert Radar (chapter 9) ==========
  var chartExpertRadarEl = document.getElementById('chart-expert-radar');
  if (chartExpertRadarEl) {
    var chartExpertRadar = echarts.init(chartExpertRadarEl, null, { renderer: 'svg' });
    chartExpertRadar.setOption({
      animation: false,
      tooltip: {
        trigger: 'item',
        backgroundColor: 'rgba(15, 26, 46, 0.95)',
        borderColor: accent,
        borderWidth: 1,
        textStyle: { color: ink, fontSize: 12 }
      },
      legend: {
        data: ['钻石级专家', '黄金级专家', '平台均值'],
        bottom: 10,
        textStyle: textStyle,
        itemWidth: 16,
        itemHeight: 8
      },
      radar: {
        indicator: [
          { name: '技能深度', max: 100 },
          { name: '项目经验', max: 100 },
          { name: '教育背景', max: 100 },
          { name: '客户评价', max: 100 },
          { name: '响应效率', max: 100 },
          { name: '影响力指数', max: 100 }
        ],
        center: ['50%', '48%'],
        radius: '65%',
        axisName: {
          color: inkSecondary,
          fontSize: 12,
          fontFamily: "'WorkSans', sans-serif"
        },
        splitLine: {
          lineStyle: { color: rule }
        },
        splitArea: {
          areaStyle: {
            color: ['rgba(0, 212, 255, 0.02)', 'rgba(0, 212, 255, 0.04)']
          }
        },
        axisLine: {
          lineStyle: { color: rule }
        }
      },
      series: [{
        type: 'radar',
        data: [
          {
            value: [96, 92, 88, 95, 90, 85],
            name: '钻石级专家',
            lineStyle: { color: accent, width: 2 },
            areaStyle: {
              color: 'rgba(0, 212, 255, 0.25)'
            },
            itemStyle: { color: accent }
          },
          {
            value: [80, 75, 72, 82, 78, 65],
            name: '黄金级专家',
            lineStyle: { color: warning, width: 2 },
            areaStyle: {
              color: 'rgba(255, 170, 0, 0.2)'
            },
            itemStyle: { color: warning }
          },
          {
            value: [65, 58, 60, 70, 62, 45],
            name: '平台均值',
            lineStyle: { color: muted, width: 1, type: 'dashed' },
            areaStyle: { color: 'transparent' },
            itemStyle: { color: muted }
          }
        ]
      }]
    });
    window.addEventListener('resize', function() { chartExpertRadar.resize(); });
  }

  // ========== Chart 5: Revenue Structure ==========
  var chartRevenueEl = document.getElementById('chart-revenue');
  if (chartRevenueEl) {
    var chartRevenue = echarts.init(chartRevenueEl, null, { renderer: 'svg' });
    chartRevenue.setOption({
      animation: false,
      tooltip: {
        trigger: 'item',
        appendToBody: true,
        formatter: '{b}: {c}% ({d}%)',
        backgroundColor: 'rgba(15, 26, 46, 0.95)',
        borderColor: accent,
        borderWidth: 1,
        textStyle: { color: ink, fontSize: 12 }
      },
      legend: {
        orient: 'vertical',
        right: 30,
        top: 'center',
        textStyle: textStyle,
        itemWidth: 12,
        itemHeight: 12,
        itemGap: 18
      },
      series: [{
        type: 'pie',
        radius: ['50%', '75%'],
        center: ['35%', '50%'],
        roseType: 'radius',
        itemStyle: {
          borderColor: bg2,
          borderWidth: 3,
          borderRadius: 2
        },
        label: {
          show: false
        },
        emphasis: {
          label: {
            show: true,
            color: ink,
            fontSize: 13,
            fontWeight: 600
          },
          itemStyle: {
            shadowBlur: 20,
            shadowColor: 'rgba(0, 212, 255, 0.4)'
          }
        },
        labelLine: { show: false },
        data: [
          { value: 60, name: '交易佣金', itemStyle: { color: accent } },
          { value: 20, name: '会员订阅', itemStyle: { color: accent2 } },
          { value: 12, name: '知识付费', itemStyle: { color: accent3 } },
          { value: 8, name: '增值服务', itemStyle: { color: warning } }
        ]
      }]
    });
    window.addEventListener('resize', function() { chartRevenue.resize(); });
  }
})();
