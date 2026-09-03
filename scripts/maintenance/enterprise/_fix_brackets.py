import re

fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\platform\svc\mox-platform-enterprise-svc\tests\smoke_enterprise.rs'
with open(fpath, encoding='utf-8-sig', errors='ignore') as f:
    content = f.read()

original = content

# 修复括号不匹配：d.get("KEY").and_then(|v| v.as_str()), → d.get("KEY")).and_then(|v| v.as_str()),
# 模式：and_then(|d| d.get("KEY").and_then(|v| v.METHOD()),
# 替换：and_then(|d| d.get("KEY")).and_then(|v| v.METHOD()),
content = re.sub(
    r'and_then\(\|d\| d\.get\(([^)]+)\)\.and_then\(\|v\| v\.(\w+)\(\)\),',
    r'and_then(|d| d.get(\1)).and_then(|v| v.\2()),',
    content
)

# 检查是否还有括号不平衡的行
lines = content.split('\n')
fixed_count = 0
for i, line in enumerate(lines):
    if 'and_then(|d| d.get(' in line:
        opens = line.count('(')
        closes = line.count(')')
        if opens != closes:
            print(f'  仍不平衡 行{i+1}: open={opens} close={closes}: {line.strip()[:100]}')
        else:
            fixed_count += 1

print(f'已修复括号，{fixed_count} 行平衡')

# 检查其他修复状态
checks = {
    'sample_data Option': 'fn sample_data() -> Option<serde_json::Value>' in content,
    'version Some': 'assert_eq!(rec.version, Some(1)' in content or '.version, Some(' in content,
    'entity_code as_deref': '.entity_code.as_deref()' in content,
    'old_version unwrap': 'old_version.unwrap_or(0)' in content,
    'list_pipelines 注释': 'list_pipelines() 方法已移除' in content,
    'metrics.snapshot 注释': 'metrics.snapshot() 方法在 orchestrator' in content,
    'create_sync data 包装': 'Some(serde_json::to_value(&data).unwrap())' in content,
}
print('\n修复状态检查:')
for k, v in checks.items():
    print(f'  {k}: {"✓" if v else "✗"}')

if content != original:
    with open(fpath, 'w', encoding='utf-8', newline='') as f:
        f.write(content)
    print('\n已保存修复')
else:
    print('\n未发生变化')
