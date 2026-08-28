import re
with open('index.html','r',encoding='utf-8') as f:
    c = f.read()

# 检查所有page div
for m in re.finditer(r'<div[^>]*id="page-(\w+)"[^>]*>', c):
    print(f'page-{m.group(1)}: {m.group(0)[:120]}')

print()
# 检查导航链接
nav_links = re.findall(r'<a[^>]*href="#/(\w+)"[^>]*>', c)
print(f'导航链接hash: {sorted(set(nav_links))}')

# 检查是否有data-route
data_routes = re.findall(r'data-route="(\w+)"', c)
print(f'data-route属性: {sorted(set(data_routes))}')

# 检查page类
page_class_count = len(re.findall(r'class="[^"]*\bpage\b[^"]*"', c))
print(f'含page类的元素: {page_class_count}')

# 检查是否有CSS .page { display: none }
if re.search(r'\.page\s*\{[^}]*display\s*:\s*none', c):
    print('CSS: .page { display: none } ✓')
else:
    print('CSS: 缺少 .page { display: none } ✗')
