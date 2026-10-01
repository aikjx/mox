# -*- coding: utf-8 -*-
import io, re
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\_cargo_err.txt"
s = io.open(p, encoding="utf-8", errors="replace").read()
# find lines like " --> path:line:col"
locs = re.findall(r"--> ([^\n]+?):(\d+):(\d+)", s)
from collections import Counter
c = Counter()
for f, l, col in locs:
    c[f.split("src\\")[-1]] += 1
for f, n in c.most_common():
    print(n, f)
print("---- error codes ----")
codes = re.findall(r"error\[E\d+\]", s)
print(Counter(codes))
