import re
with open('index.html','r',encoding='utf-8') as f:
    c = f.read()

print('initPage()调用:', c.count('initPage()'))
print('navigate(getRoute()):', c.count('navigate(getRoute())'))
print()

pages = re.findall(r'id="page-(\w+)"', c)
print('页面div IDs:', pages)

page_classes = re.findall(r'class="[^"]*page[^"]*"', c)
print('含page类的元素数:', len(page_classes))

print()
print('page-graph存在:', 'id="page-graph"' in c)
print('graph在导航链接:', '#/graph' in c)

# 检查末尾
last800 = c[-800:]
print()
print('=== 文件末尾800字符 ===')
print(last800)
