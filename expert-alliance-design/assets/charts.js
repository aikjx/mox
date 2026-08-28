(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var warning = style.getPropertyValue('--warning').trim();
  var danger = style.getPropertyValue('--danger').trim();
  var info = style.getPropertyValue('--info').trim();
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();
  var inkSecondary = style.getPropertyValue('--ink-secondary').trim();

  // Helper: build color palette
  var palette = [accent, accent2, warning, danger, info, accent + '99', accent2 + '99', warning + '99'];

  // ============================================================
  // Chart 1: 用户角色关系图 (Ring chart showing user distribution)
  // ============================================================
  var chartRolesEl = document.getElementById('chart-roles');
  if (chartRolesEl) {
    var chartRoles = echarts.init(chartRolesEl, null, { renderer: 'svg' });
    chartRoles.setOption({
      animation: false,
      tooltip: {
        trigger: 'item',
        appendToBody: true,
        formatter: '{b}: {c}% ({d}%)'
      },
      legend: {
        orient: 'vertical',
        right: '5%',
        top: 'center',
        itemWidth: 12,
        itemHeight: 12,
        textStyle: {
          color: inkSecondary,
          fontSize: 13
        },
        formatter: function(name) {
          return name;
        }
      },
      series: [{
        name: '用户角色分布',
        type: 'pie',
        radius: ['45%', '70%'],
        center: ['35%', '50%'],
        avoidLabelOverlap: false,
        itemStyle: {
          borderRadius: 4,
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
            fontWeight: 600,
            color: ink
          },
          itemStyle: {
            shadowBlur: 10,
            shadowOffsetX: 0,
            shadowColor: 'rgba(0, 0, 0, 0.2)'
          }
        },
        labelLine: {
          show: false
        },
        data: [
          { value: 70, name: '个人用户', itemStyle: { color: accent } },
          { value: 20, name: '企业用户', itemStyle: { color: accent2 } },
          { value: 5, name: '认证专家', itemStyle: { color: warning } },
          { value: 3, name: '行业顾问', itemStyle: { color: danger } },
          { value: 2, name: '机构入驻', itemStyle: { color: info } }
        ]
      }]
    });
    window.addEventListener('resize', function() { chartRoles.resize(); });
  }

  // ============================================================
  // Chart 2: 功能模块雷达图
  // ============================================================
  var chartModulesRadarEl = document.getElementById('chart-modules-radar');
  if (chartModulesRadarEl) {
    var chartModulesRadar = echarts.init(chartModulesRadarEl, null, { renderer: 'svg' });
    chartModulesRadar.setOption({
      animation: false,
      tooltip: {
        appendToBody: true
      },
      legend: {
        data: ['重要性', '复杂度'],
        top: 10,
        textStyle: {
          color: inkSecondary,
          fontSize: 13
        }
      },
      radar: {
        indicator: [
          { name: '专家发现匹配', max: 10 },
          { name: '专家主页服务', max: 10 },
          { name: '在线咨询系统', max: 10 },
          { name: '项目协作空间', max: 10 },
          { name: '知识内容社区', max: 10 },
          { name: '交易支付系统', max: 10 },
          { name: '评价信用体系', max: 10 },
          { name: '消息通知中心', max: 10 },
          { name: '数据分析看板', max: 10 },
          { name: '运营管理后台', max: 10 }
        ],
        center: ['50%', '55%'],
        radius: '65%',
        axisName: {
          color: inkSecondary,
          fontSize: 12
        },
        splitArea: {
          areaStyle: {
            color: ['rgba(64,158,255,0.02)', 'rgba(64,158,255,0.04)']
          }
        },
        axisLine: {
          lineStyle: {
            color: rule
          }
        },
        splitLine: {
          lineStyle: {
            color: rule
          }
        }
      },
      series: [{
        type: 'radar',
        data: [
          {
            value: [9.5, 9, 9.2, 7.5, 8, 9.8, 8.5, 7, 7.5, 8.5],
            name: '重要性',
            lineStyle: { color: accent, width: 2 },
            itemStyle: { color: accent },
            areaStyle: { color: accent + '20' }
          },
          {
            value: [8.5, 7, 9, 8, 7.5, 9.5, 7, 6, 7.5, 8],
            name: '复杂度',
            lineStyle: { color: accent2, width: 2 },
            itemStyle: { color: accent2 },
            areaStyle: { color: accent2 + '20' }
          }
        ]
      }]
    });
    window.addEventListener('resize', function() { chartModulesRadar.resize(); });
  }

  // ============================================================
  // Chart 3: 专家画像数据维度雷达图
  // ============================================================
  var chartExpertProfileEl = document.getElementById('chart-expert-profile');
  if (chartExpertProfileEl) {
    var chartExpertProfile = echarts.init(chartExpertProfileEl, null, { renderer: 'svg' });
    chartExpertProfile.setOption({
      animation: false,
      tooltip: {
        appendToBody: true
      },
      legend: {
        data: ['钻石专家', '金牌专家', '银牌专家'],
        top: 10,
        textStyle: {
          color: inkSecondary,
          fontSize: 13
        }
      },
      radar: {
        indicator: [
          { name: '专业资质', max: 100 },
          { name: '服务评分', max: 100 },
          { name: '响应速度', max: 100 },
          { name: '接单数量', max: 100 },
          { name: '内容产出', max: 100 },
          { name: '粉丝数量', max: 100 },
          { name: '复购率', max: 100 },
          { name: '投诉率(反向)', max: 100 }
        ],
        center: ['50%', '55%'],
        radius: '65%',
        axisName: {
          color: inkSecondary,
          fontSize: 12
        },
        splitArea: {
          areaStyle: {
            color: ['rgba(64,158,255,0.02)', 'rgba(64,158,255,0.04)']
          }
        },
        axisLine: {
          lineStyle: { color: rule }
        },
        splitLine: {
          lineStyle: { color: rule }
        }
      },
      series: [{
        type: 'radar',
        data: [
          {
            value: [98, 96, 92, 90, 88, 95, 85, 99],
            name: '钻石专家',
            lineStyle: { color: accent, width: 2 },
            itemStyle: { color: accent },
            areaStyle: { color: accent + '25' }
          },
          {
            value: [85, 88, 80, 75, 70, 78, 72, 92],
            name: '金牌专家',
            lineStyle: { color: warning, width: 2 },
            itemStyle: { color: warning },
            areaStyle: { color: warning + '20' }
          },
          {
            value: [70, 78, 65, 55, 50, 60, 60, 85],
            name: '银牌专家',
            lineStyle: { color: info, width: 2 },
            itemStyle: { color: info },
            areaStyle: { color: info + '15' }
          }
        ]
      }]
    });
    window.addEventListener('resize', function() { chartExpertProfile.resize(); });
  }

  // ============================================================
  // Chart 4: 收入结构占比饼图
  // ============================================================
  var chartRevenueEl = document.getElementById('chart-revenue');
  if (chartRevenueEl) {
    var chartRevenue = echarts.init(chartRevenueEl, null, { renderer: 'svg' });
    chartRevenue.setOption({
      animation: false,
      tooltip: {
        trigger: 'item',
        appendToBody: true,
        formatter: '{b}: {c}%'
      },
      legend: {
        orient: 'vertical',
        right: '5%',
        top: 'center',
        itemWidth: 12,
        itemHeight: 12,
        textStyle: {
          color: inkSecondary,
          fontSize: 13
        }
      },
      series: [{
        name: '收入结构',
        type: 'pie',
        radius: ['40%', '72%'],
        center: ['35%', '50%'],
        roseType: 'radius',
        itemStyle: {
          borderRadius: 6,
          borderColor: bg2,
          borderWidth: 2
        },
        label: {
          show: true,
          position: 'outside',
          formatter: '{b}\n{d}%',
          fontSize: 12,
          color: inkSecondary
        },
        labelLine: {
          show: true,
          length: 10,
          length2: 10,
          lineStyle: { color: rule }
        },
        data: [
          { value: 40, name: '服务交易抽佣', itemStyle: { color: accent } },
          { value: 20, name: '知识内容分成', itemStyle: { color: accent2 } },
          { value: 18, name: '会员订阅收入', itemStyle: { color: warning } },
          { value: 12, name: '营销推广收入', itemStyle: { color: danger } },
          { value: 10, name: '企业定制服务', itemStyle: { color: info } }
        ]
      }]
    });
    window.addEventListener('resize', function() { chartRevenue.resize(); });
  }
})();
