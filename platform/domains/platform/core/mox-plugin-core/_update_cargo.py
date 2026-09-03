path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\platform\core\mox-plugin-core\Cargo.toml'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

# The file has CRLF, handle both
old = '# VSIX解压（ZIP格式）\r\nzip = { version = "0.6", default-features = false, features = ["deflate"] }\r\n'
new = '# VSIX解压（ZIP格式）\r\nzip = { version = "0.6", default-features = false, features = ["deflate"] }\r\n# deno_core JS 运行时（阶段2：VSCode 扩展执行环境）\r\ndeno_core = "0.290"\r\n'

if old in content:
    content = content.replace(old, new)
    with open(path, 'w', encoding='utf-8-sig', newline='') as f:
        f.write(content)
    print('Cargo.toml updated successfully')
else:
    print('ERROR: old string not found, trying without CR')
    old2 = '# VSIX解压（ZIP格式）\nzip = { version = "0.6", default-features = false, features = ["deflate"] }\n'
    if old2 in content:
        content = content.replace(old2, new.replace('\r\n', '\n'))
        with open(path, 'w', encoding='utf-8-sig', newline='') as f:
            f.write(content)
        print('Cargo.toml updated (LF mode)')
    else:
        print('Still not found')
