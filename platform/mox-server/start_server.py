# -*- coding: utf-8 -*-
"""启动 mox-server（subprocess 方式，输出落日志，避免管道阻塞）。"""
import os
import socket
import subprocess
import sys
import time

BASE = os.path.dirname(os.path.abspath(__file__))
PORT = sys.argv[1] if len(sys.argv) > 1 else "8600"
LOG = os.path.join(BASE, "server.log")


def port_open(port, host="127.0.0.1"):
    s = socket.socket()
    try:
        s.settimeout(1)
        s.connect((host, int(port)))
        return True
    except Exception:  # noqa: BLE001
        return False
    finally:
        s.close()


if __name__ == "__main__":
    # 若端口已被占用则直接返回（视为已在运行）
    if port_open(PORT):
        print(f"ALREADY RUNNING on {PORT}")
        sys.exit(0)
    with open(LOG, "a", encoding="utf-8") as f:
        p = subprocess.Popen([sys.executable, "-u", "run.py", PORT],
                             cwd=BASE, stdout=f, stderr=f)
        # 等待最多 12 秒
        ok = False
        for _ in range(24):
            time.sleep(0.5)
            if port_open(PORT):
                ok = True
                break
            if p.poll() is not None:
                break
        print(f"PID={p.pid} ready={ok}")
        if not ok:
            with open(LOG, "r", encoding="utf-8") as f2:
                tail = f2.read()[-2000:]
            print("LOG_TAIL:", tail[-1500:])
        sys.exit(0 if ok else 1)
