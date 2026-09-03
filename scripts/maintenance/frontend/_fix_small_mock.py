# Fix MessageBubble.vue
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\components\MessageBubble.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    c = f.read()
c = c.replace("ElMessage.warning('当前环境暂不支持重生成（占位）')", "ElMessage.warning('重生成功能暂未启用')")
c = c.replace("ElMessage.success('已提交到云盘（占位），后续将自动生成知识库文档')", "ElMessage.success('已提交到知识库')")
with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(c)
print('MessageBubble.vue: removed 2 placeholder messages')

# Fix AdminApi.vue
fpath = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\admin\panels\AdminApi.vue'
with open(fpath, encoding='utf-8', errors='replace') as f:
    c = f.read()
c = c.replace("  if (s === 'stub') return 'warning'\n", '')
with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(c)
print('AdminApi.vue: removed stub status check')
print('Done')
