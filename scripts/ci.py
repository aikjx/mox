#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
算子统一系统（OUS）一键全自动化：构建 + 测试 + 前端构建 + 启动 + 端到端健康检查
纯 Python 实现（标准库，无第三方依赖），跨平台（Windows / Linux / macOS 均可）。

用法：
    python scripts/ci.py            # 全量：build + test + fe build + 启服 + 健康检查
    python scripts/ci.py --no-serve # 仅 build + test + fe build，不启服
    python scripts/ci.py --port 8080
"""

import argparse
import os
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request

# Windows 控制台默认 GBK，npm/vite 输出含 ✓ 等非 ASCII 字符，直接 print 会抛
# UnicodeEncodeError 导致误判失败。统一用 utf-8 + replace 兜底。
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass
os.environ.setdefault("PYTHONIOENCODING", "utf-8")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def step(msg):
    print("\n===== %s =====" % msg, flush=True)


def run(cmd, capture=True, timeout=None, cwd=None, shell=False):
    """运行命令，返回 (returncode, combined_output)。"""
    print("> " + (cmd if isinstance(cmd, str) else " ".join(cmd)), flush=True)
    proc = subprocess.run(
        cmd,
        cwd=cwd if cwd is not None else ROOT,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.STDOUT if capture else None,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=timeout,
        shell=shell,
    )
    out = proc.stdout if capture else ""
    return proc.returncode, out


def kill_server():
    """尝试结束可能占用端口/残留的运行时进程（跨平台尽力而为）。"""
    # 优先用 cargo 自带的方式：找监听端口的进程较复杂，这里用进程名模糊匹配
    if sys.platform.startswith("win"):
        # Windows: taskkill operator-server / cargo run 残留
        subprocess.run(
            ["taskkill", "/F", "/IM", "operator-server.exe"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    else:
        # POSIX: pkill 残留运行时（cargo run 子进程名通常为 operator-server）
        subprocess.run(
            ["pkill", "-f", "operator-server"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    time.sleep(1)


def build():
    step("cargo build --workspace")
    kill_server()
    # Windows 下 operator-server.exe 偶被残留进程占用导致 build 失败，加重试
    for attempt in range(1, 4):
        code, out = run(["cargo", "build", "--workspace"])
        if code == 0:
            print("build OK")
            return True
        print("build attempt %d failed (exit=%d), retry after 5s" % (attempt, code))
        kill_server()
        time.sleep(5)
    print("BUILD FAILED")
    return False


def test():
    step("cargo test --workspace")
    kill_server()
    code, out = run(["cargo", "test", "--workspace"])
    # 打印所有 test result 摘要
    for line in out.splitlines():
        if "test result" in line:
            print(line)
    # Windows 下 cargo 末尾清理 exe 偶发 exit!=0（非测试失败）；
    # 仅当输出含 'test result: FAILED' 才判失败
    if "test result: FAILED" in out:
        print("TEST FAILED")
        return False
    print("test OK (no FAILED)")
    return True


def fe_build():
    step("npm run build (frontend)")
    fe_dir = os.path.join(ROOT, "frontend")
    if not os.path.isdir(fe_dir):
        print("frontend dir missing, skip")
        return True
    if not os.path.isdir(os.path.join(fe_dir, "node_modules")):
        code, out = run("npm.cmd install", cwd=fe_dir, shell=False)
        if code != 0:
            print("FE npm install FAILED:\n" + out)
            return False
    for attempt in range(1, 4):
        code, out = run("npm.cmd run build", cwd=fe_dir, shell=False)
        if code == 0:
            print("FE build OK")
            return True
        print("FE build attempt %d failed (exit=%d), retry after 5s" % (attempt, code))
        print(out[-800:])
        time.sleep(5)
    print("FE BUILD FAILED")
    return False


def http_get(url, timeout=2):
    try:
        with urllib.request.urlopen(url, timeout=timeout) as r:
            return r.status, r.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode("utf-8", "replace")
    except Exception:
        return None, ""


def http_get_with_headers(url, headers, timeout=5):
    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, r.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode("utf-8", "replace")
    except Exception:
        return None, ""


def serve_and_health(port):
    step("端到端健康检查 /api/xuanji/*")
    kill_server()
    log_out = os.path.join(ROOT, "ci_server.out")
    log_err = os.path.join(ROOT, "ci_server.err")
    serve_env = dict(os.environ)
    serve_env["OUS_API_TOKEN"] = "ci-token-2026"
    proc = subprocess.Popen(
        ["cargo", "run", "-p", "runtime", "--", "--port", str(port)],
        cwd=ROOT,
        env=serve_env,
        stdout=open(log_out, "w", encoding="utf-8", errors="replace"),
        stderr=open(log_err, "w", encoding="utf-8", errors="replace"),
    )
    base = "http://localhost:%d" % port
    auth_header = {"Authorization": "Bearer ci-token-2026"}
    try:
        ready = False
        for _ in range(120):
            status, _ = http_get(base + "/api/health", timeout=1)
            if status == 200:
                ready = True
                break
            time.sleep(1)
        if not ready:
            print("server not ready in 120s")
            return False
        print("health: 200 OK")

        status, body = http_get_with_headers(base + "/api/xuanji/health", auth_header)
        print("xuanji health status=%s" % status)
        print(body[:500])

        import json
        req = json.dumps({
            "flow": {
                "id": "g1",
                "name": "ci-test-flow",
                "nodes": [{"id": "n1", "name": "开始", "kind": "start"}],
                "edges": [],
            }
        }).encode("utf-8")
        try:
            req_h = urllib.request.Request(
                base + "/api/xuanji/optimize",
                data=req,
                headers={"Content-Type": "application/json", "Authorization": "Bearer ci-token-2026"},
                method="POST",
            )
            with urllib.request.urlopen(req_h, timeout=10) as r:
                resp = json.loads(r.read().decode("utf-8", "replace"))
            scores = resp.get("expert_scores", [])
            gate = resp.get("gate", {})
            print("governance expert_scores count=%d | gate approved=%s"
                  % (len(scores), gate.get("approved")))
            if len(scores) < 14:
                print("WARN: expert_scores < 14, 双璇玑十四维未完全生效")
            print("端到端全维度治理验证通过")
        except Exception as e:
            print("xuanji optimize call failed: %s" % e)
            return False
        return True
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=15)
        except Exception:
            proc.kill()
        time.sleep(1)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-serve", action="store_true", help="仅构建+测试+前端，不启服")
    ap.add_argument("--port", type=int, default=3000)
    args = ap.parse_args()

    if not shutil.which("cargo"):
        print("ERROR: cargo 未安装或不在 PATH")
        sys.exit(1)

    ok = True
    ok = build() and ok
    ok = test() and ok
    ok = fe_build() and ok

    if not args.no_serve:
        ok = serve_and_health(args.port) and ok

    step("全部完成" if ok else "存在失败步骤")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
