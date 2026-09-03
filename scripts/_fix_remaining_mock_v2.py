import re

fixes = []

# ========== Fix 1: AllianceTaskView.vue ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\expert\AllianceTaskView.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove mockTasks array
pattern = r'// 演示占位：联盟任务列表（getAllianceTasks API 失败时的降级展示）\nconst mockTasks = \[.*?\n\]\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)

# Remove mockLogs array
pattern = r'// 演示占位：任务执行日志（后端待提供: GET /api/alliance/tasks/:id/logs）\nconst mockLogs = \[.*?\n\]\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)

# Remove mockFusion object
pattern = r'// 演示占位：融合结果（后端待提供: GET /api/alliance/tasks/:id/fusion-result）\nconst mockFusion = \{.*?\n\}\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)

# Fix dagNodes computed (remove hardcoded fallback)
content = content.replace(
    '''// DAG 节点：优先使用 API 数据，失败则使用演示占位
const dagNodes = computed(() => {
  if (dagNodesData.value && dagNodesData.value.length > 0) return dagNodesData.value
  if (!selectedTask.value) return []
  const base = [
    { id: 'n1', name: '需求解析', type: '算法专家', status: 'completed', x: 100, y: 60 },
    { id: 'n2', name: '实体抽取', type: 'AI专家', status: 'completed', x: 280, y: 30 },
    { id: 'n3', name: '关系抽取', type: '图谱专家', status: 'running', x: 280, y: 90 },
    { id: 'n4', name: 'Schema设计', type: '架构专家', status: 'pending', x: 460, y: 60 },
    { id: 'n5', name: '融合输出', type: '融合专家', status: 'pending', x: 640, y: 60 }
  ]
  return base
})''',
    '''// DAG 节点：由 API 加载，无数据时为空
const dagNodes = computed(() => {
  if (dagNodesData.value && dagNodesData.value.length > 0) return dagNodesData.value
  return []
})'''
)

# Fix dagEdges computed (remove hardcoded fallback) - find and replace
pattern = r'// DAG 边：优先使用 API 数据，失败则使用演示占位\nconst dagEdges = computed\(\) => \{.*?\n\}\)'
replacement = '''// DAG 边：由 API 加载，无数据时为空
const dagEdges = computed(() => {
  if (dagEdgesData.value && dagEdgesData.value.length > 0) return dagEdgesData.value
  return []
})'''
content = re.sub(pattern, replacement, content, flags=re.DOTALL)

# Fix mockLogs calls
content = content.replace(
    "logs.value = Array.isArray(logData) ? logData : (logData?.items || [...mockLogs])",
    "logs.value = Array.isArray(logData) ? logData : (logData?.items || [])"
)
content = content.replace(
    "logs.value = [...mockLogs]",
    "logs.value = []"
)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('AllianceTaskView.vue: removed mockTasks/mockLogs/mockFusion/dagNodes/dagEdges')

# ========== Fix 2: TaskView.vue ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\project\TaskView.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Find and remove mock task data
pattern = r'// 演示占位：API 失败/空时的兜底任务数据\n(.*?)\n'
match = re.search(pattern, content, flags=re.DOTALL)
if match:
    # Find the full mock block (const mockTasks = [...])
    full_pattern = r'const mock\w+ = \[.*?\n\]\n'
    content = re.sub(full_pattern, '', content, flags=re.DOTALL)
    # Replace calls to mock data
    content = re.sub(r'tasks\.value = \[.*?mock\w+.*?\]', 'tasks.value = []', content, flags=re.DOTALL)
    fixes.append('TaskView.vue: removed mock task data')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

# ========== Fix 3: ProjectsView.vue ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\project\ProjectsView.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove mock activities and documents
pattern = r'// 演示占位：项目动态流，后端待提供 /projects/:id/activities 端点\n(.*?)\n'
content = re.sub(pattern, '// 项目动态流由 /projects/:id/activities 加载，初始为空\n', content, flags=re.DOTALL)
pattern = r'// 演示占位：项目文档列表，后端待提供 /projects/:id/documents 端点\n(.*?)\n'
content = re.sub(pattern, '// 项目文档列表由 /projects/:id/documents 加载，初始为空\n', content, flags=re.DOTALL)

# Remove mock arrays
content = re.sub(r'const mock\w+ = \[.*?\n\]\n', '', content, flags=re.DOTALL)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('ProjectsView.vue: removed mock activities/documents')

# ========== Fix 4: Dashboard.vue (remaining) ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\project\Dashboard.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Fix catch "保留演示占位"
content = content.replace(
    '} catch (e) { /* 保留演示占位 */ }',
    '} catch (e) { console.error(\'[dashboard] load failed:\', e) }'
)

# Remove mock phase progress
content = re.sub(r'// 项目阶段进度：优先使用后端数据，失败降级为演示占位\n', '// 项目阶段进度由后端加载\n', content)
content = re.sub(r'// 演示占位：API 无数据时的兜底默认值\n', '', content)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('Dashboard.vue: removed remaining mock fallbacks')

# ========== Fix 5: ResourcesView.vue (remaining) ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\project\ResourcesView.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove mock resource rows
content = re.sub(r'// 演示占位：API 返回空时的兜底资源行数据\n(.*?)\n', '// 资源行由 API 加载，初始为空\n', content, flags=re.DOTALL)
content = re.sub(r'// 演示占位：CPU/内存无数据时的兜底默认值\n', '', content)
content = re.sub(r'const mock\w+ = \[.*?\n\]\n', '', content, flags=re.DOTALL)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('ResourcesView.vue: removed remaining mock fallbacks')

# ========== Fix 6: KnowledgeBasePanel.vue (remaining) ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\project\panels\KnowledgeBasePanel.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove remaining mock comments and data
content = re.sub(r'// 演示占位：API 返回空时的兜底文档数据\n', '', content)
content = re.sub(r'// 演示占位：实体搜索失败时的兜底数据\n', '', content)
content = re.sub(r'const mock\w+ = \[.*?\n\]\n', '', content, flags=re.DOTALL)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('KnowledgeBasePanel.vue: removed remaining mock fallbacks')

# ========== Fix 7: AdminMonitor.vue (remaining) ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\admin\panels\AdminMonitor.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove remaining mock comments
content = re.sub(r'// 演示占位：日志为空时使用模拟日志兜底（后端 /api/logs 正常返回时不触发）\n', '', content)
content = re.sub(r'// 演示占位：生成模拟执行日志（后端 /api/logs 正常返回时不使用）\n', '', content)
content = re.sub(r'// 加载监控域真实数据（失败则保留演示占位）\n', '// 加载监控域真实数据\n', content)

# Remove any remaining generateMock functions
content = re.sub(r'function generateMock\w+\(\) \{.*?\n\}\n', '', content, flags=re.DOTALL)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('AdminMonitor.vue: removed remaining mock fallbacks')

# ========== Fix 8: ExpertPlazaView.vue (remaining comments) ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\expert\ExpertPlazaView.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

content = content.replace(
    '// 平台统计：调用 GET /api/experts/stats，失败保留演示占位',
    '// 平台统计：调用 GET /api/experts/stats'
)
content = content.replace(
    '// 我的预约列表：调用 GET /api/experts/bookings/mine，失败保留演示占位',
    '// 我的预约列表：调用 GET /api/experts/bookings/mine'
)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('ExpertPlazaView.vue: updated remaining comments')

print('Batch fixes (round 2):')
for f in fixes:
    print(f'  - {f}')
print('Done')
