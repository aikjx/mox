fpath = r'D:\a10\aikjx\gitcode\infotopograph\Cargo.toml'
with open(fpath, 'rb') as f:
    data = f.read()

# Decode with errors='replace' to handle any bad bytes
text = data.decode('utf-8', errors='replace')

old = '''# HTTP client for AI APIs and browser automation
reqwest = { version = "0.12", features = ["json", "rustls-tls", "stream"], default-features = false }
base64 = "0.22"'''

new = '''# HTTP client for AI APIs and browser automation
reqwest = { version = "0.12", features = ["json", "rustls-tls", "stream"], default-features = false }
base64 = "0.22"

# Redis client (cloud-filer-svc real Redis backend)
redis = { version = "0.26", features = ["tokio-comp"] }'''

n = text.count(old)
print(f'匹配: {n}')
if n > 0:
    text = text.replace(old, new, 1)
    # Write back as UTF-8 without BOM
    with open(fpath, 'w', encoding='utf-8', newline='') as f:
        f.write(text)
    print('已添加 redis 依赖')
else:
    print('未匹配，检查内容')
    # Find the reqwest line
    for i, line in enumerate(text.split('\n')):
        if 'reqwest' in line:
            print(f'行 {i+1}: {line[:80]}')
