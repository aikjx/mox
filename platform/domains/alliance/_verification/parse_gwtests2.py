# -*- coding: utf-8 -*-
import re, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
path = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\_verification\fv_gw_tests2.log"
with open(path, encoding='utf-8', errors='replace') as f:
    lines = f.read().splitlines()
cur=None; tot=0; totf=0
for ln in lines:
    s=ln.strip()
    if s.startswith("Running "): cur=s
    m=re.match(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed",s)
    if m:
        p,fl=int(m.group(2)),int(m.group(3)); tot+=p; totf+=fl
        print(f"{'OK ' if m.group(1)=='ok' else 'FLD'} p={p:3d} f={fl:2d}  {cur[:70] if cur else ''}")
print("="*50)
print(f"TOTAL passed={tot} failed={totf}")
# new tests detail
for ln in lines:
    if 'webhook_persistence' in ln or 'a3_plan_quota' in ln or 'plan_quota' in ln:
        print("NEW:", ln.strip())
errs=[l for l in lines if re.search(r"error\[|^error: |could not compile",l)]
if errs: print("COMPILE ERRORS:", errs[:20])
