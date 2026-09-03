#!/usr/bin/env python3
"""Update API-SPECIFICATION.md: message -> msg for ApiResponse fields."""
import re

fpath = r'D:\a10\aikjx\gitcode\infotopograph\docs\api\API-SPECIFICATION.md'
with open(fpath, encoding='utf-8-sig', errors='replace', newline='') as f:
    c = f.read()

original = c

# 1. JSON 示例: "message": "ok", -> "msg": "ok",
c = c.replace('"message": "ok",', '"msg": "ok",')
# 2. JSON 示例: "message": "节点不存在" -> "msg": "节点不存在"
c = c.replace('"message": "节点不存在"', '"msg": "节点不存在"')
# 3. JSON 示例: "message": "缺少认证 Token" -> "msg": "缺少认证 Token"
c = c.replace('"message": "缺少认证 Token"', '"msg": "缺少认证 Token"')
# 4. JSON 示例: "message": "无权限执行此操作" -> "msg": "无权限执行此操作"
c = c.replace('"message": "无权限执行此操作"', '"msg": "无权限执行此操作"')

# 5. 字段表格: | `message` | `string` | ...
c = c.replace('| `message` | `string` |', '| `msg` | `string` |')

# 6. 说明文字: - `message` 固定为 -> - `msg` 固定为
c = c.replace('- `message` 固定为', '- `msg` 固定为')
c = c.replace('- `message` 为面向用户', '- `msg` 为面向用户')

# 7. 底部汇总格式示例: { "code": N, "message": ...
# 用正则替换所有 { "code": <number>, "message": 模式
c = re.sub(r'(\{ "code": \d+, "message":)', lambda m: m.group(1).replace('"message":', '"msg":'), c)

# 8. 代码映射表中的 ApiResponse::error(code, message) - 这是函数参数名，保持不变
# 不修改

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(c)

changed = (original != c)
print(f'API-SPECIFICATION.md updated: {changed}')
# Count remaining message references
remaining = len(re.findall(r'message', c))
print(f'Remaining "message" references: {remaining}')
for i, line in enumerate(c.split('\n'), 1):
    if 'message' in line:
        print(f'  L{i}: {line.strip()[:80]}')
