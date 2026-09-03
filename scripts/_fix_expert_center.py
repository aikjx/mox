import re

fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\expert\ExpertCenterView.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

fixes = []

# 1. Remove buildMockGraph function
pattern = r'// 演示占位：公司官网需求图谱预设数据（API 不可用时降级）\nfunction buildMockGraph\(\) \{.*?\n\}\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)
fixes.append('buildMockGraph: removed')

# 2. Fix catch "降级到 mock"
content = content.replace(
    '} catch (e) { /* 降级到 mock */ }',
    '} catch (e) { console.error(\'[expert-center] load graph failed:\', e); graphData.nodes = []; graphData.edges = [] }'
)
fixes.append('catch 降级到 mock: empty state')

# 3. Fix catch "保留演示占位"
content = content.replace(
    '} catch (e) { /* 保留演示占位 */ }',
    '} catch (e) { console.error(\'[expert-center] load phase failed:\', e) }'
)
fixes.append('catch 保留演示占位: error log')

# 4. Fix randomizeGraph function (calls buildMockGraph)
old = '''function randomizeGraph() {
  const mock = buildMockGraph()
  graphData.nodes.splice(0, graphData.nodes.length, ...mock.nodes)
  graphData.edges.splice(0, graphData.edges.length, ...mock.edges)
  nextTick(() => drawGraph())
}'''
new = '''function randomizeGraph() {
  // 后端需求图谱 API 就绪后重新加载，当前清空
  graphData.nodes.splice(0, graphData.nodes.length)
  graphData.edges.splice(0, graphData.edges.length)
  nextTick(() => drawGraph())
}'''
if old in content:
    content = content.replace(old, new, 1)
    fixes.append('randomizeGraph: empty instead of mock')

# 5. Fix phaseProgress initial hardcoded value
content = content.replace(
    "// 演示占位：阶段进度初始值（后端待提供: GET /api/projects/:id/phase-progress）\nconst phaseProgress = reactive({ requirement: 8, architecture: 0, develop: 0, release: 0 })",
    "// 阶段进度由 GET /api/projects/:id/phase-progress 加载，初始为 0\nconst phaseProgress = reactive({ requirement: 0, architecture: 0, develop: 0, release: 0 })"
)
fixes.append('phaseProgress initial: zeros')

# 6. Remove watch(localPhase) simulated progress update
old = '''// 演示占位：页面阶段变化时模拟进度更新（后端待提供: 阶段进度实时同步）
watch(localPhase, (k) => {
  phaseProgress[k] = Math.max(phaseProgress[k] || 0, 10 + Math.round(Math.random() * 8))
})'''
new = '''// 阶段进度由后端实时同步，当前不做前端模拟'''
if old in content:
    content = content.replace(old, new, 1)
    fixes.append('watch(localPhase) simulated: removed')

# 7. Fix onMounted fallback to mock
old = '''  // 优先加载后端需求图谱，失败降级为 Mock 图谱
  const loaded = await loadRequirementsGraph()
  if (!loaded) {
    const mock = buildMockGraph()
    graphData.nodes.push(...mock.nodes)
    graphData.edges.push(...mock.edges)
  }'''
new = '''  // 加载后端需求图谱，失败显示空状态
  const loaded = await loadRequirementsGraph()
  if (!loaded) {
    console.warn('[expert-center] requirements graph not loaded, showing empty canvas')
  }'''
if old in content:
    content = content.replace(old, new, 1)
    fixes.append('onMounted fallback: empty state')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print('ExpertCenterView.vue fixes:')
for f in fixes:
    print(f'  - {f}')
print('Done')
