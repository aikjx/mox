# -*- coding: utf-8 -*-
"""T3 MCP Server 真实验证客户端。
以 Content-Length 帧 over stdio 与 mox-alliance-mcp-server 通信，
走完整 initialize -> notifications/initialized -> tools/list -> tools/call 三工具，
捕获网关真实响应作为证据。零 mock。
"""
import json, os, subprocess, sys, time

ROOT = r"D:\a10\aikjx\gitcode\infotopograph"
MCP_BIN = os.path.join(ROOT, "target", "debug", "mox-alliance-mcp-server.exe")
GATEWAY_URL = os.environ.get("MOX_MCP_GATEWAY_URL", "http://127.0.0.1:3080")


def send_frame(proc, obj):
    body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
    proc.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
    proc.stdin.flush()


def recv_frame(proc):
    # 读帧头直到空行
    headers = {}
    while True:
        line = proc.stdout.readline()
        if not line:
            raise RuntimeError("stdout EOF while reading headers")
        line = line.rstrip(b"\r\n")
        if line == b"":
            break
        k, _, v = line.partition(b":")
        headers[k.strip().decode().lower()] = v.strip().decode()
    n = int(headers["content-length"])
    body = proc.stdout.read(n)
    return json.loads(body.decode("utf-8"))


def main():
    env = dict(os.environ)
    env["MOX_MCP_GATEWAY_URL"] = GATEWAY_URL
    # 代码内正式开发凭证（debug 构建 dev_mode）：由 MCP server 注入 Bearer，零硬编码。
    env["MOX_INTERNAL_TOKEN"] = os.environ.get("MOX_INTERNAL_TOKEN", "dev-secret-token")
    print("== spawn mcp server:", MCP_BIN, flush=True)
    proc = subprocess.Popen(
        [MCP_BIN], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, env=env,
    )

    log = []
    try:
        # 1) initialize
        send_frame(proc, {"jsonrpc": "2.0", "id": 1, "method": "initialize",
                          "params": {"protocolVersion": "2024-11-05",
                                     "capabilities": {}, "clientInfo": {"name": "verify-client", "version": "1.0"}}})
        r1 = recv_frame(proc)
        log.append(("initialize", r1))
        print("[initialize] serverInfo =", r1["result"]["serverInfo"],
              "| protocol =", r1["result"]["protocolVersion"], flush=True)

        # 2) notifications/initialized（无响应）
        send_frame(proc, {"jsonrpc": "2.0", "method": "notifications/initialized"})
        time.sleep(0.2)

        # 3) tools/list
        send_frame(proc, {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
        r2 = recv_frame(proc)
        names = [t["name"] for t in r2["result"]["tools"]]
        log.append(("tools/list", r2))
        print("[tools/list] tools =", names, flush=True)

        # 4) tools/call 三个真实工具
        calls = [
            ("expert_search", {"query": "分布式架构与微服务", "domains": ["architecture"], "limit": 3}),
            ("optimal_team", {"required_domains": ["architecture", "ai"], "max_members": 3, "min_rating": 4.0}),
            ("graph_expand", {"seeds": ["exp-architecture-001"], "max_depth": 2, "top_k": 5}),
        ]
        for i, (tool, args) in enumerate(calls, start=3):
            send_frame(proc, {"jsonrpc": "2.0", "id": i, "method": "tools/call",
                              "params": {"name": tool, "arguments": args}})
            r = recv_frame(proc)
            log.append((tool, r))
            text = r["result"]["content"][0]["text"]
            payload = json.loads(text)
            print("\n===== tools/call %s (isError=%s) =====" % (tool, r["result"].get("isError")), flush=True)
            print("upstream:", payload.get("upstream"), "| gateway_status:", payload.get("gateway_status"), flush=True)
            print(json.dumps(payload.get("result"), ensure_ascii=False, indent=2)[:1800], flush=True)

        # 落盘证据
        with open(os.path.join(ROOT, "platform", "domains", "alliance", "mcp", "verify_evidence.json"),
                  "w", encoding="utf-8") as f:
            json.dump([{"step": s, "response": r} for s, r in log], f, ensure_ascii=False, indent=2)
        print("\n== evidence saved ==", flush=True)
    finally:
        proc.stdin.close()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()
        err = proc.stderr.read().decode("utf-8", "replace")
        print("\n---- MCP server stderr ----", flush=True)
        print(err[-2000:], flush=True)


if __name__ == "__main__":
    main()
