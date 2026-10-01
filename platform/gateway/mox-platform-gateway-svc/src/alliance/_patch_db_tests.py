# -*- coding: utf-8 -*-
"""一次性：给 experts_db.rs 测试里剩余的 conn 调用补 "default" 租户参数。"""
import io

p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_db.rs"
with io.open(p, "r", encoding="utf-8") as f:
    s = f.read()

repls = [
    ('upsert_graph_edge_conn(&conn, 1, &e)',
     'upsert_graph_edge_conn(&conn, "default", 1, &e)'),
    ('set_graph_meta_conn(&conn, 3,',
     'set_graph_meta_conn(&conn, "default", 3,'),
    ('set_graph_meta_conn(&conn, 4,',
     'set_graph_meta_conn(&conn, "default", 4,'),
]
for old, new in repls:
    n = s.count(old)
    s = s.replace(old, new)
    print("replaced", n, "->", old[:40])

with io.open(p, "w", encoding="utf-8", newline="") as f:
    f.write(s)
print("OK")
