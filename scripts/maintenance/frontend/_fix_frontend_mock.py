import re

# Fix 1: RegisterExpertDialog.vue - remove mockExpert in catch block
fpath1 = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\components\expert\RegisterExpertDialog.vue'
with open(fpath1, encoding='utf-8', errors='replace') as f:
    content = f.read()

old = '''    // 优雅降级：模拟成功（演示用）
    ElMessage.warning('注册服务暂不可用，已生成本地模拟数据')
    const mockExpert = {
      id: 'exp_' + Date.now().toString(36),
      name: formData.name.trim(),
      type: formData.type,
      avatar: formData.avatar,
      description: formData.description,
      capabilities: [...formData.domains, ...formData.skills],
      experienceLevel: formData.experienceLevel,
      status: 'active',
      metrics: { total_consults: 0, success_rate: 0.95 }
    }
    emit('registered', mockExpert)
    handleClose()'''

new = '''    // 注册失败：只显示错误，不生成假数据
    ElMessage.error(submitError.value)'''

if old in content:
    content = content.replace(old, new, 1)
    print('RegisterExpertDialog: removed mockExpert')
else:
    print('RegisterExpertDialog: pattern not found, trying regex...')
    # Try to find and remove the mockExpert block
    pattern = r'    // 优雅降级：模拟成功（演示用）.*?handleClose\(\)'
    content = re.sub(pattern, '    ElMessage.error(submitError.value)', content, flags=re.DOTALL)
    print('RegisterExpertDialog: removed via regex')

with open(fpath1, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

# Fix 2: KnowledgeBasePanel.vue - remove demo placeholder initial data
fpath2 = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\project\panels\KnowledgeBasePanel.vue'
with open(fpath2, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Remove categories demo placeholder
old_cats = '''// 演示占位：分类树初始数据，实际由 kbGetCategories() 加载覆盖
const categories = ref(['''
new_cats = '''// 分类树：由 kbGetCategories() 加载，初始为空
const categories = ref([])'''

if old_cats in content:
    # Find the end of the array (the closing ])
    start = content.find(old_cats)
    end = content.find('])', start) + 2
    content = content[:start] + new_cats + content[end:]
    print('KnowledgeBasePanel: removed categories demo placeholder')
else:
    print('KnowledgeBasePanel: categories pattern not found')

# Remove tags demo placeholder
old_tags = '''// 演示占位：标签初始数据，实际由 kbGetTags() 加载覆盖
const tags = ref(['''
new_tags = '''// 标签：由 kbGetTags() 加载，初始为空
const tags = ref([])'''

if old_tags in content:
    start = content.find(old_tags)
    end = content.find('])', start) + 2
    content = content[:start] + new_tags + content[end:]
    print('KnowledgeBasePanel: removed tags demo placeholder')
else:
    print('KnowledgeBasePanel: tags pattern not found')

# Remove stats demo placeholder
old_stats = '''// 演示占位：统计初始数据，实际由 kbGetStats() 加载覆盖
const stats = ref({'''
new_stats = '''// 统计：由 kbGetStats() 加载，初始为空
const stats = ref({})'''

if old_stats in content:
    start = content.find(old_stats)
    # Find matching closing })
    depth = 0
    i = start + len(old_stats) - 1
    while i < len(content):
        if content[i] == '{':
            depth += 1
        elif content[i] == '}':
            depth -= 1
            if depth == 0:
                end = i + 1
                break
        i += 1
    content = content[:start] + new_stats + content[end:]
    print('KnowledgeBasePanel: removed stats demo placeholder')
else:
    print('KnowledgeBasePanel: stats pattern not found')

with open(fpath2, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print('Done')
