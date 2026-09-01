# -*- coding: utf-8 -*-
"""executor expert 模式端到端验证：
起 mock OpenAI(8999) + executor-svc(expert模式, MOX_LLM_*->mock) → POST /internal/executions
→ 节点经 ExpertNodeExecutor → LlmExpertConsultant → OpenAiChatClient 真实 HTTP 到 mock →
解析评分/结论 → 节点 completed。验证真实 LLM 调用链（auth/model/messages）。
"""
import json, os, subprocess, sys, time, uuid, urllib.request, signal

ROOT = r"D:\a10\aikjx\gitcode\infotopograph"
EXE = os.path.join(ROOT, "target", "debug", "mox-alliance-executor.exe")
TMP = os.path.dirname(os.path.abspath(__file__))
LLM_PORT = 8999
EXEC_PORT = 3200
TENANT = "11111111-1111-1111-1111-111111111111"
TASK_ID = str(uuid.uuid4())

def http(method, url, body=None, headers=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    for k, v in (headers or {}).items():
        req.add_header(k, v)
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return r.status, json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read().decode() or "{}")
    except Exception as e:
        return 0, {"error": str(e)}

def main():
    # 1. 起 mock OpenAI
    mock = subprocess.Popen([sys.executable, os.path.join(TMP, "mock_openai.py")],
                            cwd=TMP, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    time.sleep(1)
    # 2. 起 executor (expert 模式 + LLM env 指向 mock)
    env = dict(os.environ)
    env["MOX_ALLIANCE_EXECUTOR_MODE"] = "expert"
    env["MOX_ALLIANCE_SERVER_PORT"] = str(EXEC_PORT)
    env["MOX_LLM_ENABLED"] = "1"
    env["MOX_LLM_API_KEY"] = "test-key-123"
    env["MOX_LLM_BASE_URL"] = "http://127.0.0.1:%d/v1" % LLM_PORT
    env["MOX_LLM_MODEL"] = "test-model"
    env["MOX_LLM_TIMEOUT_MS"] = "15000"
    exe = subprocess.Popen([EXE], cwd=ROOT, env=env,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    time.sleep(2)

    def stop():
        mock.terminate(); exe.terminate()
        time.sleep(1)
        for p in (mock, exe):
            try: p.kill()
            except Exception: pass

    try:
        # 探活（最多 15 秒，debug 二进制首启较慢）
        st = 0
        for _ in range(15):
            st, _ = http("GET", "http://127.0.0.1:%d/health" % EXEC_PORT)
            if st == 200:
                break
            time.sleep(1)
        print("[1] executor /health =", st)
        if st != 200:
            out = exe.stdout.read().decode(errors="replace")[-3000:]
            print("executor 启动日志:\n", out)
            stop(); return 1

        now = "2026-09-01T08:00:00Z"
        task = {
            "task_id": TASK_ID, "tenant_id": TENANT, "user_id": TENANT,
            "title": "expert-e2e", "description": "验证生产专家模式真实 LLM 调用链",
            "task_type": "analysis", "status": "pending", "priority": "high",
            "progress": 0.0, "current_node_id": None,
            "mode": "parallel", "fusion_strategy": "weighted",
            "created_at": now, "started_at": None, "completed_at": None,
            "duration_ms": None, "tags": [], "fusion_result": None,
        }
        node = {
            "node_id": "n1", "task_id": TASK_ID, "expert_id": "code-expert-001",
            "module_id": "expert-code", "name": "代码专家推理", "description": "乘法计算",
            "status": "pending", "retry_count": 0, "dependencies": [],
            "input_refs": [], "output_ref": "out1",
            "started_at": None, "completed_at": None, "duration_ms": None,
            "error_message": None,
        }
        plan = {"task_id": TASK_ID, "mode": "parallel", "fusion_strategy": "weighted",
                "nodes": [node], "version": 1, "created_at": now}
        options = {"max_retries": 1, "node_timeout_ms": 20000, "fail_fast": False}
        body = {"task": task, "plan": plan, "options": options}

        st, r = http("POST", "http://127.0.0.1:%d/internal/executions" % EXEC_PORT, body)
        print("[2] submit /internal/executions =", st, json.dumps(r, ensure_ascii=False)[:120])
        if st != 200:
            out = exe.stdout.read().decode(errors="replace")[-4000:]
            print("executor 提交后日志:\n", out)
            stop(); return 1

        # 3. 轮询任务状态 → 节点应 completed（ExpertNodeExecutor 走真实 LLM 链）
        final = None
        for i in range(30):
            time.sleep(1)
            st, r = http("GET", "http://127.0.0.1:%d/tasks/%s/status" % (EXEC_PORT, TASK_ID), headers={"X-Tenant-Id": TENANT})
            if st == 200:
                final = r
                if r.get("status") in ("completed", "failed", "cancelled"):
                    break
        print("[3] 最终任务状态:", json.dumps(final, ensure_ascii=False)[:500])

        # 4. 节点详情
        st, nodes = http("GET", "http://127.0.0.1:%d/tasks/%s/nodes" % (EXEC_PORT, TASK_ID), headers={"X-Tenant-Id": TENANT})
        node_status = "?"
        output = None
        if st == 200 and nodes.get("nodes"):
            ns = nodes["nodes"][0]
            node_status = ns.get("status")
            output = ns.get("output") or ns.get("result")
        print("[4] 节点状态:", node_status, "| output:", json.dumps(output, ensure_ascii=False)[:300])

        # 5. 验证 mock 收到真实 HTTP LLM 请求（auth/model/messages）
        logf = os.path.join(TMP, "mock_reqs.log")
        reqs = []
        if os.path.exists(logf):
            with open(logf, encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if line:
                        try: reqs.append(json.loads(line))
                        except Exception: pass
        print("[5] mock 收到 LLM 请求数:", len(reqs))
        for q in reqs[:3]:
            print("    path=%s auth=%s model=%s messages=%d" % (q.get("path"), q.get("auth"), q.get("model"), q.get("n_messages")))

        ok_auth = any(q.get("auth") == "Bearer test-key-123" for q in reqs)
        ok_model = any(q.get("model") == "test-model" for q in reqs)
        ok_path = any("/chat/completions" in q.get("path", "") for q in reqs)
        print("[6] 判定: 真实HTTP=%s auth=%s model=%s path=%s" % (len(reqs) > 0, ok_auth, ok_model, ok_path))

        # 结论
        done = final and final.get("status") == "completed"
        passed = done and len(reqs) > 0 and ok_auth and ok_model and ok_path
        print("RESULT:", "PASS" if passed else "FAIL",
              "| 任务=%s 节点=%s LLM请求=%d" % (final and final.get("status"), node_status, len(reqs)))
        if final:
            print("  output:", json.dumps(output, ensure_ascii=False)[:400])
        stop()
        return 0 if passed else 1
    finally:
        stop()

if __name__ == "__main__":
    sys.exit(main())
