# -*- coding: utf-8 -*-
import io, glob
d = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance"
files = glob.glob(d + r"\experts_*.rs")
total = 0
for p in files:
    s = io.open(p, encoding="utf-8").read()
    n = s.count("tenant.0.clone()")
    if n:
        s = s.replace("tenant.0.clone()", "tenant.clone()")
        io.open(p, "w", encoding="utf-8", newline="").write(s)
        print(p.split("\\")[-1], n)
        total += n
print("total", total)
