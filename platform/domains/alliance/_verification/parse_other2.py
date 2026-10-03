# -*- coding: utf-8 -*-
import re, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
path = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\_verification\fv_other_crates2.log"
with open(path, encoding='utf-8', errors='replace') as f:
    lines = f.read().splitlines()
cur = None
tot = {}
for ln in lines:
    s = ln.strip()
    m = re.match(r"==== CARGO TEST -p (\S+) ====", s)
    if m:
        cur = m.group(1); tot[cur] = [0,0]; print(f"\n##### {cur} #####"); continue
    r = re.match(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed", s)
    if r and cur:
        p, fl = int(r.group(2)), int(r.group(3))
        tot[cur][0]+=p; tot[cur][1]+=fl
        print(f"  {r.group(1)}: passed={p} failed={fl}")
    e = re.match(r"---- EXIT\(([^)]+)\)=(\d+) ----", s)
    if e: print(f"  [exit {e.group(2)}]")
errs = [l for l in lines if re.search(r"error\[|^error: |could not compile", l)]
print("\n===== TOTALS =====")
for c,(p,fl) in tot.items(): print(f"  {c}: passed={p} failed={fl}")
if errs:
    print("\n===== COMPILE/TEST ERRORS =====")
    for l in errs[:50]: print(l)
