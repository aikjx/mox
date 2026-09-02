# -*- coding: utf-8 -*-
import re

def routes(path, prefixes=()):
    s = open(path, encoding="utf-8-sig").read()
    out = []
    for m in re.finditer(r'\.route\(\s*"([^"]+)"\s*,\s*(\w+)\(([^)]*)\)', s):
        p, handler, args = m.group(1), m.group(2), m.group(3)
        # 方法推断
        methods = []
        for mm in re.finditer(r'(\w+)\(', handler):
            methods.append(mm.group(1))
        out.append((p, methods[0] if methods else "?", handler))
    return out

print("=== expert-svc routes (server.rs) ===")
r = routes(r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\ai\svc\mox-ai-expert-svc\src\server.rs")
for p, m, h in r:
    print(f"  {m.upper():6} {p}  [{h}]")

print("\n=== gateway routes (mod.rs, only /graph & /experts & /expert) ===")
r2 = routes(r"D:\a10\aikjx\gitcode\infotopograph\platform\legacy\backend-rust\src\api\mod.rs")
for p, m, h in r2:
    if p.startswith(("/graph", "/experts", "/expert", "/api")):
        print(f"  {m.upper():6} {p}  [{h}]")
