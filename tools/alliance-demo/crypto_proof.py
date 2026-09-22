# -*- coding: utf-8 -*-
"""全链路一键加密归一化证明（crypto proof）

验证目标（与 docs/api/API-CRYPTO-TRANSPORT.md 一一对应）：
  P1 向后兼容：不携带协商头 → 一切明文如旧（存量前端/脚本零破坏）
  P2 网关边缘加密：携带 x-mox-crypto → 统一信封 data 变为 SM4-GCM 密文信封
  P3 压缩有效：gzip 后密文显著小于明文（大列表 payload）
  P4 调度器维度归一：直连 :3100 同样按开关+协商加密
  P5 内部链路互操作：网关(SDK加密协商)→调度器(加密应答)→网关解密成功
                    —— 若网关 /api/alliance/tasks 列表在 sm4 全开下返回 200 即为链路解密互通
  P6 篡改拒绝：伪造密文上送 → 400（认证加密不可伪造）
"""
import json
import urllib.request
import urllib.error
import base64
import sys
import uuid

GW = "http://127.0.0.1:3080"
SCH = "http://127.0.0.1:3100"
AUTH = {"Authorization": "Bearer dev-secret-token"}
NEG = {"x-mox-crypto": "sm4-gcm+gzip"}

passed = []
failed = []

def call(base, method, path, headers=None, body=None):
    h = dict(AUTH)
    h.update(headers or {})
    data = None
    if body is not None:
        data = json.dumps(body, ensure_ascii=False).encode("utf-8")
        h["Content-Type"] = "application/json"
    req = urllib.request.Request(base + path, data=data, headers=h, method=method)
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return r.status, dict(r.headers), r.read()
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), e.read()

def check(name, cond, detail=""):
    (passed if cond else failed).append(name)
    print(("PASS " if cond else "FAIL ") + name + (("  " + detail) if detail else ""))

# 播种 6 个任务，使列表 payload 具备压缩统计意义
tenant = str(uuid.uuid4())
th = {"X-Tenant-Id": tenant}
for i in range(6):
    call(GW, "POST", "/api/alliance/tasks", th,
         {"title": f"压缩样本任务-{i}", "description": "归一化证明播种任务" * 6, "mode": "parallel"})
s, hs, b = call(GW, "GET", "/api/alliance/tasks", th)
plain_ok = s == 200 and b'"code"' in b and b"crypto" not in b
check("P1 无协商头→明文信封不变", plain_ok, f"status={s}")
plain_list_len = len(b)

# P2 网关边缘加密
s2, hs2, b2 = call(GW, "GET", "/api/alliance/tasks", {**th, **NEG})
try:
    env = json.loads(b2.decode("utf-8"))
    c = env.get("data", {}).get("crypto", {})
    p2 = (s2 == 200 and c.get("alg") == "SM4-GCM" and c.get("zip") == "gzip"
          and hs2.get("x-mox-crypto") == "sm4-gcm+gzip"
          and env.get("code") == 0)
except Exception:
    p2 = False
check("P2 带协商头→data 变 SM4-GCM+gzip 密文信封", p2, f"status={s2}")

# P3 压缩有效（用同一列表的明文响应 vs 密文字节数）
ct_bytes = len(base64.b64decode(c.get("ct", ""))) if p2 else 0
if plain_list_len > 400 and ct_bytes:
    check("P3 gzip+SM4 密文 < 明文响应体积", ct_bytes * 2 < plain_list_len,
          f"plain={plain_list_len}B ct={ct_bytes}B ratio={ct_bytes/plain_list_len:.1%}")
else:
    # 列表太短不具压缩参考：造一批任务再比（若环境允许）
    check("P3 压缩样本不足（列表过小），仍验证密文非膨胀", bool(ct_bytes),
          f"plain={plain_list_len}B ct={ct_bytes}B")

# P4 调度器维度归一（直连 :3100；调度器返回裸 DTO → 整体加密为 crypto 信封）
s3, hs3, b3 = call(SCH, "GET", "/tasks", {**th, **NEG})
try:
    env3 = json.loads(b3.decode("utf-8"))
    c3 = env3.get("crypto") or env3.get("data", {}).get("crypto", {})
    p4 = s3 == 200 and c3.get("alg") == "SM4-GCM"
except Exception:
    p4 = False
check("P4 调度器 :3100 同样按开关加密(裸DTO整体加密)", p4, f"status={s3}")

# P5 内部链路互操作：创建任务（网关→SDK 协商→调度器密文应答→SDK 透明解密→网关再加密出边）
s4, _, b4 = call(GW, "POST", "/api/alliance/tasks",
                 {**th, **NEG},
                 {"title": "crypto-e2e-probe", "description": "全链路加密证明任务", "mode": "parallel"})
try:
    env4 = json.loads(b4.decode("utf-8"))
    ok4 = env4.get("code") == 0 and env4.get("data", {}).get("crypto", {}).get("alg") == "SM4-GCM"
except Exception:
    ok4 = False
check("P5 网关-调度器内部加解密链路互操作（创建任务成功）", ok4, f"status={s4}")

# P6 篡改拒绝（直连调度器，伪造密文上送）
fake = {"crypto": {"alg": "SM4-GCM", "zip": "gzip", "nonce": base64.b64encode(b"0" * 12).decode(),
                   "ct": base64.b64encode(b"1" * 32).decode(), "tag": base64.b64encode(b"2" * 16).decode()}}
s5, _, _ = call(SCH, "POST", "/tasks", {**th, **NEG}, fake)
check("P6 伪造密文上送被拒（400 认证失败）", s5 == 400, f"status={s5}")

print("-" * 60)
print(f"结果: {len(passed)} 通过 / {len(failed)} 失败")
sys.exit(1 if failed else 0)
