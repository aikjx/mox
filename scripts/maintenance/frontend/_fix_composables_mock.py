import re

# ========== Fix 1: useKnowledgeBase.js ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\composables\useKnowledgeBase.js'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove getMockDocuments function
pattern = r'  // ========== Mock Data ==========\n\n  function getMockDocuments\(\) \{.*?\n  \}\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)

# Fix fetchDocuments catch
content = content.replace(
    '''    } catch (e) {
      documents.value = getMockDocuments()
      ElMessage.warning('使用本地缓存数据')
    } finally {''',
    '''    } catch (e) {
      documents.value = []
      console.error('[kb] fetchDocuments failed:', e)
    } finally {'''
)

# Fix fetchCategories catch
content = content.replace(
    '''    } catch { /* keep mock data */ }''',
    '''    } catch (e) { categories.value = []; console.error('[kb] fetchCategories failed:', e) }'''
)

# Fix fetchTags catch (second occurrence)
content = content.replace(
    '''    } catch { /* keep mock data */ }''',
    '''    } catch (e) { tags.value = []; console.error('[kb] fetchTags failed:', e) }'''
)

# Fix fetchStats catch
content = content.replace(
    '''    } catch {
      stats.value = {
        total: documents.value.length,
        categories: categories.value.length,
        versions: documents.value.reduce((sum, d) => sum + (d.version_count || 1), 0),
        analyzed: documents.value.filter(d => d.ai_analyzed).length
      }
    }''',
    '''    } catch (e) {
      stats.value = {}
      console.error('[kb] fetchStats failed:', e)
    }'''
)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
print('useKnowledgeBase.js: removed getMockDocuments + 4 catch fallbacks')

# ========== Fix 2: useGraphCanvas.js ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\composables\workspace\useGraphCanvas.js'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove useMockGraph function
pattern = r'  function useMockGraph\(\) \{.*?\n  \}\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)

# Replace calls to useMockGraph() with empty graph + error log
content = content.replace('} else useMockGraph()', '} else { nodes.value = []; edges.value = []; console.warn("[graph] API returned empty, showing blank canvas") }')
content = content.replace('} catch (e) { console.warn(\'[workspace] 加载图谱失败:\', e); useMockGraph() }',
                          '} catch (e) { console.warn(\'[workspace] 加载图谱失败:\', e); nodes.value = []; edges.value = [] }')
content = content.replace('if (layout === \'force\') useMockGraph()',
                          'if (layout === \'force\') { /* force layout uses real graph data */ }')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
print('useGraphCanvas.js: removed useMockGraph + 3 call sites')

# ========== Fix 3: useTaskOrchestration.js ==========
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\composables\workspace\useTaskOrchestration.js'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove generateMockSubtasks function
pattern = r'  function generateMockSubtasks\(taskDesc\) \{.*?\n  \}\n'
content = re.sub(pattern, '', content, flags=re.DOTALL)

# Replace call to generateMockSubtasks
content = content.replace(
    'const subtasks = generateMockSubtasks(taskDesc)',
    'const subtasks = [] // 后端待提供子任务生成 API，当前为空'
)

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
print('useTaskOrchestration.js: removed generateMockSubtasks')

print('Done')
