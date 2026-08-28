import re
with open('index.html','r',encoding='utf-8') as f:
    content = f.read()

# 检查路由函数
for m in re.finditer(r'function\s+(\w+).*?\{', content):
    name = m.group(1)
    if any(k in name.lower() for k in ['route','render','navig','page','init']):
        print(f'函数: {name}')

print('\nhashchange监听:', 'hashchange' in content)
print('DOMContentLoaded:', 'DOMContentLoaded' in content)
print('window.onload:', 'window.onload' in content)
print('addEventListener:', content.count('addEventListener'))

# 检查页面容器
for mid in ['app','main','content','page','root']:
    if f'id="{mid}"' in content:
        print(f'容器: id="{mid}"')

# 检查导航链接
nav_links = re.findall(r'href="#/(\w+)"', content)
print(f'\n导航链接: {sorted(set(nav_links))}')

# 检查fetchData调用
print(f'\nfetchData调用: {content.count("fetchData(")}')
for m in re.finditer(r'fetchData\([\'"](\w+)[\'"]\)', content):
    print(f'  fetchData({m.group(1)})')

# 检查renderPage/route调用
print(f'\nrenderPage调用: {content.count("renderPage(")}')
print(f'route()调用: {content.count("route()")}')

# 检查可能的JS语法错误 - 大括号匹配
opens = content.count('{')
closes = content.count('}')
print(f'\n大括号: {{={opens} }}={closes} 差={opens-closes}')

# 检查script标签数量
print(f'script标签: {content.count("<script>")} 个')
