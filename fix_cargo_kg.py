path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-service-svc\Cargo.toml'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

content = content.replace(
    'mox-framework = { workspace = true, optional = true }\n',
    'mox-framework = { workspace = true, optional = true }\nmox-api-protocol = { workspace = true, optional = true }\n'
)

old_feat = 'http-adapter = ["dep:chrono", "dep:axum", "dep:mox-framework", "dep:serde_qs"]'
new_feat = 'http-adapter = ["dep:chrono", "dep:axum", "dep:mox-framework", "dep:serde_qs", "dep:mox-api-protocol"]'
content = content.replace(old_feat, new_feat)

with open(path, 'w', encoding='utf-8-sig', newline='') as f:
    f.write(content)
print('Cargo.toml updated')
