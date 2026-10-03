# -*- coding: utf-8 -*-
import re, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
path = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\_verification\fv_other_crates.log"
with open(path, encoding='utf-8', errors='replace') as f:
    lines = f.read().splitlines()
cur_crate = None
for ln in lines:
    s = ln.strip()
    m = re.match(r"==== CARGO TEST -p (\S+) ====", s)
    if m:
        cur_crate = m.group(1)
        print(f"\n##### {cur_crate} #####")
        continue
    m = re.match(r"Running (\S+ .*?)(?:\s*\(.*\))?$", s)
    if m and ("unittests" in s or "tests\\" in s or "deps\\" in s):
        print(f"  RUN: {s[:90]}")
    r = re.match(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed", s)
    if r:
        print(f"  -> {r.group(1)}: passed={r.group(2)} failed={r.group(3)}")
    e = re.match(r"---- EXIT\(([^)]+)\)=(\d+) ----", s)
    if e:
        print(f"  [exit {e.group(2)}]")
# any compile errors?
errs = [l for l in lines if re.search(r"error\[|^error: |error: could not compile", l)]
if errs:
    print("\n===== COMPILE ERRORS =====")
    for l in errs[:40]:
        print(l)
else:
    print("\n(no compile errors)")
