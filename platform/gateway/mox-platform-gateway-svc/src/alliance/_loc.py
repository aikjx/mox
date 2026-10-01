# -*- coding: utf-8 -*-
import io, re
lines = io.open(r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\_cargo_err.txt", encoding="utf-8").read().splitlines()
cur = None
for i, l in enumerate(lines):
    m = re.match(r"error\[(E\d+)\]", l)
    if m:
        # find next --> line
        for j in range(i, min(i+6, len(lines))):
            mm = re.search(r"-->\s*(\S+):(\d+):(\d+)", lines[j])
            if mm:
                f = mm.group(1).split("\\")[-1].split("/")[-1]
                print(m.group(1), f, mm.group(2))
                break
