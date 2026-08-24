# -*- coding: utf-8 -*-
"""临时运行器：执行打包版 --selftest-full 并汇总结果。"""
import json
import subprocess
import sys
import time

exe = r"d:\a10\aikjx\gitcode\infotopograph\melody2score\dist\Melody2Score\Melody2Score.exe"
out = r"d:\a10\aikjx\gitcode\infotopograph\melody2score\dist\selftest_full_frozen.json"

t0 = time.time()
p = subprocess.run([exe, "--selftest-full", out], capture_output=True, timeout=600)
print("exit=%s, elapsed=%.1fs" % (p.returncode, time.time() - t0))

r = json.load(open(out, encoding="utf-8"))
print("pass:", r["pass"], "| mode:", r["mode"], "| frozen:", r["frozen"])
print("basic: notes=%s bpm=%s backend=%s jianpu=%s... elapsed=%ss"
      % (r["note_count"], r["bpm"], r["backend"], r["jianpu_head"][:12], r["elapsed_sec"]))
for k, v in r.get("full_chain", {}).items():
    tag = "PASS" if v["pass"] else "FAIL"
    if v.get("skip"):
        tag += "/SKIP"
    print("  [%s] %s: %s" % (tag, k, v["detail"]))
rg = r.get("regression", {})
print("回归: %s/%s 精确, avg_tol=%s, avg_pc=%s, pass=%s"
      % (rg.get("exact_hits"), rg.get("n"), rg.get("avg_tol"),
         rg.get("avg_pc"), rg.get("pass")))
for s in rg.get("samples", []):
    if "skip" in s:
        print("  [SKIP] %s/%s: %s" % (s["title"], s["timbre"], s["skip"]))
    else:
        print("  [%s] %s/%s: tol=%s pc=%s notes=%s bpm=%s"
              % ("EXACT" if s["exact"] else "DIFF", s["title"], s["timbre"],
                 s["tol"], s["pc"], s["notes"], s["bpm"]))
sys.exit(0 if r["pass"] else 1)
