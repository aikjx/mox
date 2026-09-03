fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\config.rs'
with open(fpath, encoding='utf-8', errors='replace') as f:
    c = f.read()

old = '''                "/kg/v1".into(),
                "/ai/engine".into(),'''
new = '''                "/kg/v1".into(),
                "/ai/engine".into(),
                // 迁移期：/alliance/v1/* 暂为公开（专家联盟端点刚桥接真实实现，
                // 前端 dev 与 E2E 测试需直接访问；生产环境待 auth 对接 IAM JWT 后回收）。
                "/alliance/v1".into(),'''

if old in c:
    c = c.replace(old, new, 1)
    print('Added /alliance/v1 to public_paths')
else:
    print('Pattern not found')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(c)
print('Done')
