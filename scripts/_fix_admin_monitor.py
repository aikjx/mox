import re

fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\admin\panels\AdminMonitor.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

fixes = []

# 1. Replace systemMetrics hardcoded data with zeros
old = '''// ===== 系统资源指标（演示占位：后端待提供 /actuator/metrics 详细指标聚合端点） =====
const systemMetrics = reactive({
  cpu: 35.2,
  memory: 58.4,
  memTotal: 32,
  memUsed: 18.7,
  disks: [
    { name: '系统盘 C:', usage: 62.5 },
    { name: '数据盘 D:', usage: 45.3 },
    { name: '备份盘 E:', usage: 78.1 }
  ],
  netUpload: 2.5, // MB/s
  netDownload: 18.3 // MB/s
})'''
new = '''// ===== 系统资源指标（由 /actuator/metrics 加载，初始为空） =====
const systemMetrics = reactive({
  cpu: 0,
  memory: 0,
  memTotal: 0,
  memUsed: 0,
  disks: [],
  netUpload: 0,
  netDownload: 0
})'''
if old in content:
    content = content.replace(old, new, 1)
    fixes.append('systemMetrics: zeros')

# 2. Replace qualityMetrics hardcoded data with zeros
old = '''// ===== 服务质量指标（演示占位：后端待提供 /monitor/quality 端点） =====
const qualityMetrics = reactive({
  qps: 128.5,
  qpsTrend: 5.2,
  errorRate: 0.12,
  errorCount: 46,
  avgLatency: 42,
  p50: 28,
  p95: 125,
  p99: 380,
  onlineUsers: 1247,
  peakUsers: 2156,
  activeConnections: 892
})'''
new = '''// ===== 服务质量指标（由 /monitor/quality 加载，初始为空） =====
const qualityMetrics = reactive({
  qps: 0,
  qpsTrend: 0,
  errorRate: 0,
  errorCount: 0,
  avgLatency: 0,
  p50: 0,
  p95: 0,
  p99: 0,
  onlineUsers: 0,
  peakUsers: 0,
  activeConnections: 0
})'''
if old in content:
    content = content.replace(old, new, 1)
    fixes.append('qualityMetrics: zeros')

# 3. Replace businessMetrics hardcoded data with zeros
old = '''// ===== 业务指标（演示占位：后端待提供 /monitor/business 聚合端点） =====
const businessMetrics = reactive({
  conversations: 3562,
  expertConsultations: 128,
  workflowRuns: 892,
  operatorCalls: 12450
})'''
new = '''// ===== 业务指标（由 /monitor/business 加载，初始为空） =====
const businessMetrics = reactive({
  conversations: 0,
  expertConsultations: 0,
  workflowRuns: 0,
  operatorCalls: 0
})'''
if old in content:
    content = content.replace(old, new, 1)
    fixes.append('businessMetrics: zeros')

# 4. Replace alertMetrics hardcoded data with zeros
old = '''// ===== 告警统计（演示占位：后端待提供 /monitor/alerts/summary 端点） =====
const alertMetrics = reactive({
  total: 23,
  p0: 2,
  p1: 8,
  p2: 13
})'''
new = '''// ===== 告警统计（由 /monitor/alerts/summary 加载，初始为空） =====
const alertMetrics = reactive({
  total: 0,
  p0: 0,
  p1: 0,
  p2: 0
})'''
if old in content:
    content = content.replace(old, new, 1)
    fixes.append('alertMetrics: zeros')

# 5. Replace generateTimeSeries to return empty array (no mock data)
old = '''// ===== 演示占位：Mock 数据生成（后端待提供时序指标查询端点 /monitor/timeseries） =====
function generateTimeSeries(points, base, variance, trend = 0) {
  const data = []
  let val = base
  for (let i = 0; i < points; i++) {
    val = base + (Math.random() - 0.5) * variance * 2 + trend * (i / points - 0.5)
    val = Math.max(0, val)
    data.push(parseFloat(val.toFixed(2)))
  }
  return data
}'''
new = '''// ===== 时序指标（后端待提供 /monitor/timeseries，当前返回空数组） =====
function generateTimeSeries(points, base, variance, trend = 0) {
  return [] // 后端端点就绪后替换为真实数据
}'''
if old in content:
    content = content.replace(old, new, 1)
    fixes.append('generateTimeSeries: empty array')

# 6. Remove updateMockMetrics function
pattern = r'// 演示占位：模拟指标波动（后端待提供实时指标推送）\nfunction updateMockMetrics\(\) \{.*?\n\}\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)
fixes.append('updateMockMetrics: removed')

# 7. Remove updateMockMetrics calls (setInterval and onMounted)
content = content.replace('refreshTimer = setInterval(updateMockMetrics, 3000)',
                          '// refreshTimer = setInterval(fetchAllMetrics, 30000) // 后端端点就绪后启用')
content = content.replace('updateMockMetrics()', '// updateMockMetrics() // 已移除 mock')

# 8. Replace 6 catch fallbacks "保留演示占位数据" with empty state
content = content.replace('} catch (e) { /* 保留演示占位数据 */ }',
                          '} catch (e) { console.error(\'[monitor] fetch failed:\', e) }')

# 9. Remove simulated service nodes
pattern = r'// 演示占位：模拟服务节点状态（后端待提供 /monitor/nodes 服务发现端点）\n.*?serviceNodes\.value = \[.*?\]\n'
content = re.sub(pattern, '// 服务节点状态由 /monitor/nodes 加载，初始为空\n', content, flags=re.DOTALL)
fixes.append('simulated service nodes: removed')

# 10. Remove business bar chart hardcoded data
old = '''    // 演示占位：业务量柱状图硬编码数据，后端待提供 /monitor/business/timeseries 端点
    const businessData = [3562, 4231, 3890, 5120, 4780, 5340, 4920]'''
new = '''    // 业务量柱状图由 /monitor/business/timeseries 加载，当前为空
    const businessData = []'''
if old in content:
    content = content.replace(old, new, 1)
    fixes.append('business bar chart: empty')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print('AdminMonitor.vue fixes:')
for f in fixes:
    print(f'  - {f}')
print('Done')
