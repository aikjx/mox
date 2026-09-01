#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
manage.py — 璇玑系统统一运维脚本【兼容别名】（薄转发）

历史文件名 manage.py 已归一化为 server-manage.py（权威入口）。
本文件仅为向后兼容保留，把所有参数原样转发给 scripts/server-manage.py：
  1. 防旧命令 / 旧脚本断链（start.sh、start-all.ps1、stop-all.ps1、deploy/start.ps1 等）；
  2. 新代码一律调用 scripts/server-manage.py（单一权威文件名）。

用法：
  python scripts/manage.py <action> [args...]     # 等价于 python scripts/server-manage.py ...
"""
import os
import subprocess
import sys

# 本文件位于 <repo>/scripts/，权威入口为同目录 server-manage.py
_HERE = os.path.dirname(os.path.abspath(__file__))
_AUTHORITATIVE = os.path.join(_HERE, "server-manage.py")


def main() -> int:
    if not os.path.exists(_AUTHORITATIVE):
        print(
            f"[ERROR] 权威入口不存在: {_AUTHORITATIVE}",
            file=sys.stderr,
        )
        print(
            "[ERROR] scripts/manage.py 为兼容别名，仅当 scripts/server-manage.py 存在时可用。",
            file=sys.stderr,
        )
        return 1
    # 原样转发参数（保持 cwd 不变，server-manage.py 自行按仓库根解析）
    cmd = [sys.executable, _AUTHORITATIVE] + sys.argv[1:]
    try:
        proc = subprocess.run(cmd)
    except KeyboardInterrupt:
        return 130
    return proc.returncode


if __name__ == "__main__":
    sys.exit(main())
