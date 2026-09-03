path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kb-svc\src\handlers.rs'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

# Global replace: all remaining '-> Response {' are handler return types
count = content.count('-> Response {')
content = content.replace('-> Response {', '-> ApiResponse<Value> {')

with open(path, 'w', encoding='utf-8-sig', newline='') as f:
    f.write(content)

print(f'Replaced {count} handler return types')
print(f'Remaining -> Response: {content.count("-> Response")}')
