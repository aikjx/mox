import re
with open('index.html','r',encoding='utf-8') as f:
    c = f.read()

# initPage引用的元素ID
ids_needed = ['homeProducts','homeNews','homeCases','allProducts','allNews','allCases','teamList','chipLines','navLinks','navToggle']
print('=== initPage/导航引用的元素 ===')
for eid in ids_needed:
    exists = f'id="{eid}"' in c
    print(f'  {eid}: {"✓" if exists else "✗ 缺失"}')

# 检查所有getElementById调用
print('\n=== 所有getElementById调用 ===')
for m in re.finditer(r"getElementById\(['\"](\w+)['\"]\)", c):
    eid = m.group(1)
    exists = f'id="{eid}"' in c
    if not exists:
        print(f'  ✗ 引用但缺失: {eid}')

# 检查querySelector引用的类
print('\n=== querySelectorAll引用的类 ===')
for m in re.finditer(r"querySelectorAll\(['\"]([^'\"]+)['\"]\)", c):
    sel = m.group(1)
    print(f'  {sel}')

# 检查form的onsubmit
print('\n=== 表单事件 ===')
print('submitContact引用:', c.count('submitContact'))
print('onsubmit属性:', 'onsubmit' in c)

# 检查按钮点击
print('\n=== 按钮/交互元素 ===')
for m in re.finditer(r'onclick="([^"]+)"', c):
    print(f'  onclick: {m.group(1)}')
