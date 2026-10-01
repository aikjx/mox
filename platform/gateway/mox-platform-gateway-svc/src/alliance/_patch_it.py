# -*- coding: utf-8 -*-
import io
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\tests\experts_db_persistence.rs"
s = io.open(p, encoding="utf-8").read()
reps = [
    ("experts_db::save_registry(&map)", 'experts_db::save_registry("default", &map)'),
    ("experts_db::load_registry()", 'experts_db::load_registry("default")'),
    ("experts_db::save_graph(&", 'experts_db::save_graph("default", &'),
    ("experts_db::load_graph()", 'experts_db::load_graph("default")'),
]
for old, new in reps:
    n = s.count(old); s = s.replace(old, new); print(n, old)
io.open(p, "w", encoding="utf-8", newline="").write(s)
