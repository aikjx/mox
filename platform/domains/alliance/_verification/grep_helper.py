# -*- coding: utf-8 -*-
"""Grep helper: search regex patterns across alliance + gateway trees, print file:line."""
import os, re, io, sys

ROOTS = [
    r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance",
    r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src",
]

def iter_files():
    for root in ROOTS:
        for dp, dn, fn in os.walk(root):
            # skip target / .git / _verification
            dn[:] = [d for d in dn if d not in ("target", ".git", "node_modules", "_verification")]
            for f in fn:
                if f.endswith((".rs", ".toml")):
                    yield os.path.join(dp, f)

def grep(pattern, flags=0, label=None):
    rx = re.compile(pattern, flags)
    hits = []
    for path in iter_files():
        try:
            with io.open(path, "r", encoding="utf-8") as f:
                for i, line in enumerate(f, 1):
                    if rx.search(line):
                        hits.append((path, i, line.rstrip()))
        except Exception:
            pass
    return hits

if __name__ == "__main__":
    # patterns to search, from CLI args: --pat "regex" --label "name"
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--pat", required=True)
    ap.add_argument("--label", default="")
    ap.add_argument("-i", action="store_true")
    args = ap.parse_args()
    flags = re.IGNORECASE if args.i else 0
    hits = grep(args.pat, flags)
    out = ["### PATTERN: %s  (label=%s)  hits=%d" % (args.pat, args.label, len(hits))]
    for p, i, l in hits[:200]:
        # shorten path
        short = p.replace("D:\\a10\\aikjx\\gitcode\\infotopograph\\platform\\", "")
        out.append("%s:%d: %s" % (short, i, l.strip()[:200]))
    print("\n".join(out))
