# -*- coding: utf-8 -*-
import re, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
path = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\_verification\fv_gw_tests.log"
with open(path, encoding='utf-8', errors='replace') as f:
    lines = f.read().splitlines()
cur = None
for ln in lines:
    s = ln.strip()
    if s.startswith("Running "):
        cur = s
    m = re.match(r"test result: ok\. (\d+) passed; (\d+) failed", s)
    if m:
        print(f"pass={int(m.group(1)):4d}  {cur}")
        cur = None
    if re.match(r"test result: FAILED", s):
        print(f"FAILED  {cur} :: {s}")
        cur = None
