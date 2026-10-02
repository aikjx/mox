# -*- coding: utf-8 -*-
import io
REPORT = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\_verification\backend-fix-report.md"
with io.open(REPORT, "r", encoding="utf-8") as f:
    s = f.read()
old = "- gateway alliance 用例：见 p0c_gateway_test.log（本轮补跑）"
new = ("- gateway alliance 用例：`cargo test -p mox-platform-gateway-svc alliance` "
       "→ **64 passed, 0 failed**（0.20s；另有 1 个既有 warning `unused import: rand::RngCore` "
       "位于 system/mfa.rs:26，与本次三 svc 改动无关，未触碰）")
if old in s:
    s = s.replace(old, new)
    with io.open(REPORT, "w", encoding="utf-8", newline="") as f:
        f.write(s)
    print("updated gateway line")
else:
    print("anchor not found")
