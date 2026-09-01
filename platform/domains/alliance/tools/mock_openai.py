# -*- coding: utf-8 -*-
"""Mock OpenAI 兼容 /v1/chat/completions 服务，用于验证 executor expert 模式的真实 LLM 调用链。
记录请求到 mock_reqs.log，返回固定最终答案（含结论评分/是否否决）。"""
import http.server, json, threading, os, datetime, sys

PORT = 8999
LOG = os.path.join(os.path.dirname(os.path.abspath(__file__)), "mock_reqs.log")
LOCK = threading.Lock()

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length else b""
        req = json.loads(body.decode("utf-8")) if body else {}
        with LOCK:
            with open(LOG, "a", encoding="utf-8") as f:
                f.write(json.dumps({
                    "ts": datetime.datetime.now().isoformat(),
                    "path": self.path,
                    "auth": self.headers.get("Authorization"),
                    "model": req.get("model"),
                    "n_messages": len(req.get("messages", [])),
                    "first_user": (req.get("messages") or [{}])[0].get("content", "")[:80],
                }, ensure_ascii=False) + "\n")
        # 返回 OpenAI chat.completions 格式：固定最终答案（无工具调用 → ReAct 一轮结束）
        content = "结论：6*7 等于 42，这是基本乘法运算结果。\n结论评分：0.9\n是否否决：否"
        resp = {
            "id": "chatcmpl-mock",
            "object": "chat.completion",
            "model": req.get("model", "test-model"),
            "choices": [{"index": 0, "message": {"role": "assistant", "content": content},
                         "finish_reason": "stop"}],
        }
        data = json.dumps(resp, ensure_ascii=False).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        # /v1/models 探活
        data = json.dumps({"object": "list", "data": [{"id": "test-model"}]}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

if __name__ == "__main__":
    if os.path.exists(LOG):
        os.remove(LOG)
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
    print("mock-openai listening on 127.0.0.1:%d, log=%s" % (PORT, LOG), flush=True)
    srv.serve_forever()
