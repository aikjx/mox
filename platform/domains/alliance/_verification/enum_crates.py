# -*- coding: utf-8 -*-
"""Enumerate alliance crates: list dirs, check Cargo.toml, list lib.rs modules."""
import os, json, io, sys

BASE = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance"
out = []

def read(path):
    try:
        with io.open(path, "r", encoding="utf-8") as f:
            return f.read()
    except Exception as e:
        return "<ERR %s>" % e

# Walk top-level
for top in sorted(os.listdir(BASE)):
    top_path = os.path.join(BASE, top)
    if not os.path.isdir(top_path):
        continue
    out.append("=== TOP: %s ===" % top)
    # direct Cargo.toml?
    direct_toml = os.path.join(top_path, "Cargo.toml")
    if os.path.exists(direct_toml):
        out.append("  [crate-at-top] Cargo.toml: %s" % direct_toml)
    for sub in sorted(os.listdir(top_path)):
        sub_path = os.path.join(top_path, sub)
        if not os.path.isdir(sub_path):
            continue
        toml = os.path.join(sub_path, "Cargo.toml")
        has_toml = os.path.exists(toml)
        lib_rs = os.path.join(sub_path, "src", "lib.rs")
        has_lib = os.path.exists(lib_rs)
        main_rs = os.path.join(sub_path, "src", "main.rs")
        has_main = os.path.exists(main_rs)
        out.append("  - %s | Cargo.toml=%s lib.rs=%s main.rs=%s" % (sub, has_toml, has_lib, has_main))
        if has_toml:
            content = read(toml)
            name = ""
            for line in content.splitlines():
                ls = line.strip()
                if ls.startswith("name"):
                    name = ls
                    break
            out.append("      crate-name: %s" % name)
        if has_lib:
            lc = read(lib_rs)
            mods = []
            for line in lc.splitlines():
                ls = line.strip()
                if ls.startswith("pub mod ") or ls.startswith("mod "):
                    mods.append(ls)
            out.append("      lib.rs modules (%d):" % len(mods))
            for m in mods:
                out.append("        " + m)
        if has_main:
            mc = read(main_rs)
            # find bind / listen
            for i, line in enumerate(mc.splitlines(), 1):
                if any(k in line for k in ["bind", "listen", "3080", "3100", "3200", "3400", "SocketAddr", "TcpListener"]):
                    out.append("      main.rs:%d  %s" % (i, line.strip()))

with io.open(os.path.join(os.path.dirname(__file__), "enum_crates.txt"), "w", encoding="utf-8") as f:
    f.write("\n".join(out))
print("done, lines:", len(out))
