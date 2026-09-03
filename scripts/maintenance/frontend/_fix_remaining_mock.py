import re

fixes = []

# ========== Fix 1: Dashboard.vue ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\project\Dashboard.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove generateMockLogs call
content = content.replace(
    'logs.value = logsArr.length > 0 ? logsArr : generateMockLogs()',
    'logs.value = logsArr'
)

# Remove generateMockLogs function
pattern = r'// 演示占位：API 返回空时的兜底执行日志\nfunction generateMockLogs\(\) \{.*?\n\}\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)

# Remove hardcoded fallback values in stats
content = content.replace('stats.value[1].value = (st && st.graph && st.graph.nodes) ?? 23',
                          'stats.value[1].value = (st && st.graph && st.graph.nodes) ?? 0')
content = content.replace('stats.value[2].value = (st && st.executions_count) ?? 15',
                          'stats.value[2].value = (st && st.executions_count) ?? 0')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('Dashboard.vue: removed generateMockLogs + hardcoded fallbacks')

# ========== Fix 2: ExpertPlazaView.vue ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\expert\ExpertPlazaView.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove mockExperts array
pattern = r'// 演示占位：专家列表（getExperts API 失败时的降级展示）\nconst mockExperts = \[.*?\n\]\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)

# Fix catch in loadExperts
content = content.replace(
    '''  } catch (e) {
    console.warn('[ExpertPlaza] API 加载失败，使用 Mock 数据:', e.message)
    experts.value = processExperts(mockExperts)
  } finally {''',
    '''  } catch (e) {
    console.error('[ExpertPlaza] API 加载失败:', e)
    experts.value = []
  } finally {'''
)

# Fix catch "保留演示占位"
content = content.replace(
    '} catch (e) { /* 保留演示占位 */ }',
    '} catch (e) { console.error(\'[ExpertPlaza] load stats failed:\', e) }'
)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('ExpertPlazaView.vue: removed mockExperts + catch fallbacks')

# ========== Fix 3: KnowledgeBasePanel.vue (its own getMockDocuments) ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\project\panels\KnowledgeBasePanel.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove getMockDocuments call
content = content.replace(
    'documents.value = getMockDocuments()',
    'documents.value = []'
)

# Remove getMockDocuments function
pattern = r'function getMockDocuments\(\) \{.*?\n\}\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('KnowledgeBasePanel.vue: removed own getMockDocuments')

# ========== Fix 4: AdminMonitor.vue (remaining mock) ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\admin\panels\AdminMonitor.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove business bar chart hardcoded data
content = content.replace(
    '''    // 演示占位：业务量柱状图硬编码数据，后端待提供 /monitor/business/timeseries 端点
    const businessData = [3562, 4231, 3890, 5120, 4780, 5340, 4920]''',
    '''    // 业务量柱状图由 /monitor/business/timeseries 加载，当前为空
    const businessData = []'''
)

# Remove generateMockMonitorLogs call
content = content.replace(
    'const safeLogs = logs.length > 0 ? logs : generateMockMonitorLogs()',
    'const safeLogs = logs'
)

# Remove generateMockMonitorLogs function
pattern = r'function generateMockMonitorLogs\(\) \{.*?\n\}\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
fixes.append('AdminMonitor.vue: removed business hardcode + generateMockMonitorLogs')

# ========== Fix 5: ResourcesView.vue (hardcoded health) ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\project\ResourcesView.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Find and replace hardcoded health values - look for the comment
pattern = r'// 演示占位：插件/算子/图谱/总线健康度为硬编码值，后端待提供 /ai/resources/health 详细指标\n(.*?)\n'
match = re.search(pattern, content, flags=re.DOTALL)
if match:
    # Replace the hardcoded values with zeros/empty
    old_block = match.group(0)
    new_block = '// 健康度由 /ai/resources/health 加载，初始为 0\n'
    content = content.replace(old_block, new_block)
    fixes.append('ResourcesView.vue: removed hardcoded health values')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print('Batch fixes:')
for f in fixes:
    print(f'  - {f}')
print('Done')
