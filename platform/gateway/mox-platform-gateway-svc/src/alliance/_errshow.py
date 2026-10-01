# -*- coding: utf-8 -*-
import io, sys
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\_cargo_err.txt"
s = io.open(p, encoding="utf-8", errors="replace").read()
want = sys.argv[1]
# print blocks containing "want" file path
import re
blocks = s.split("error[")
out = []
for b in blocks:
    if want in b:
        out.append("error[" + b[:600])
print("\n----\n".join(out[:12]))
