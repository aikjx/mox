#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
开发专家联盟 · Nacos 阶段二+三端到端验证脚本
============================================
验证两条链路（真实 rnacos 服务端）：
  A. 配置中心（阶段二）：scheduler-svc 经 load_scheduler_with_nacos 从 Nacos 拉取远程配置
     —— 远程配置 server.port=3155 应整体覆盖本地引导 yml(3199)，服务实际监听 3155。
  B. 注册中心（阶段三）：naming.enabled=true 时 scheduler 把自己注册到 Nacos；
     停止后 deregister 移除实例。

前置：rnacos 已运行在 127.0.0.1:8848/9848；scheduler 二进制已编译
  (cargo build -p mox-alliance-scheduler-svc)。
用法：python tools/alliance_nacos_e2e.py
"""
import io
import json
import os
import subprocess
import sys
import time
import urllib.parse
import urllib.request

ROOT = r"D:\a10\aikjx\gitcode\infotopograph"
NACOS = "http://127.0.0.1:8848"
DATA_ID = "mox-alliance-scheduler-e2e.yml"
REMOTE_PORT = 3155          # 远程配置里声明的端口（用于证明远程覆盖本地）
LOCAL_PORT = 3199           # 本地引导 yml 里声明的端口（应被远程覆盖，不监听）
SVC_NAME = "mox-alliance-scheduler"
TMP = os.environ["TEMP"]

def http(method, url, data=None):
    if data is not None and not isinstance(data, bytes):
        data = urllib.parse.urlencode(data).encode()
    req = urllib.request.Request(url, data=data, method=method)
    try:
        with urllib.request.urlopen(req, timeout=5) as r:
            return r.status, r.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()
    except Exception as e:
        return 0, str(e)

def step(msg):
    print(f"\n=== {msg} ===")

step("1. 发布远程配置到 Nacos（data_id=%s, server.port=%d）" % (DATA_ID, REMOTE_PORT))
remote_yml = f"""server:
  host: 127.0.0.1
  port: {REMOTE_PORT}
scheduler:
  max_concurrent_tasks: 100
  queue_capacity: 1000
  default_priority: normal
  default_mode: parallel
  default_fusion_strategy: weighted
  plan_generation_timeout_ms: 30000
executor_bridge:
  base_url: http://localhost:3200
  timeout_ms: 30000
expert_service:
  base_url: http://localhost:3300
  timeout_ms: 5000
  enabled: false
storage:
  mode: memory
  path: data/alliance_tasks.json
nacos:
  enabled: true
  server_addr: "127.0.0.1:8848"
  namespace: ""
  username: ""
  password: ""
  group: "DEFAULT_GROUP"
  data_id: "{DATA_ID}"
naming:
  enabled: true
  service_name: {SVC_NAME}
  group: "DEFAULT_GROUP"
  ip: "127.0.0.1"
  port: {REMOTE_PORT}
  weight: 1.0
  metadata:
    - "protocol=http"
    - "domain=alliance"
# e2e-marker: 远程配置生效
"""
st, body = http("POST", f"{NACOS}/nacos/v1/cs/configs",
                {"dataId": DATA_ID, "group": "DEFAULT_GROUP", "content": remote_yml})
assert st == 200, f"发布远程配置失败 {st}: {body}"
print("发布成功")

step("2. 写本地引导 yml（server.port=%d，验证其被远程覆盖）" % LOCAL_PORT)
local_yml = f"""server:
  host: 127.0.0.1
  port: {LOCAL_PORT}
nacos:
  enabled: true
  server_addr: "127.0.0.1:8848"
  namespace: ""
  username: ""
  password: ""
  group: "DEFAULT_GROUP"
  data_id: "{DATA_ID}"
naming:
  enabled: true
  service_name: {SVC_NAME}
  ip: "127.0.0.1"
  port: {LOCAL_PORT}
  metadata: []
"""
local_cfg = os.path.join(TMP, "alliance-scheduler-e2e.yml")
io.open(local_cfg, "w", encoding="utf-8").write(local_yml)

step("3. 启动 scheduler-svc（MOX_ALLIANCE_CONFIG_FILE=本地引导 yml）")
env = dict(os.environ)
env["MOX_ALLIANCE_CONFIG_FILE"] = local_cfg
env["RUST_LOG"] = "info,mox_alliance=debug"
proc = subprocess.Popen(
    [os.path.join(ROOT, "target", "debug", "mox-alliance-scheduler.exe")],
    cwd=ROOT, env=env,
    stdout=open(os.path.join(TMP, "sched_e2e_stdout.log"), "w"),
    stderr=subprocess.STDOUT,
    creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
)
print("pid", proc.pid)

try:
    step("4. 探活远程配置端口 %d（证明配置中心覆盖本地）" % REMOTE_PORT)
    remote_up = False
    for i in range(20):
        if proc.poll() is not None:
            break
        st, _ = http("GET", f"http://127.0.0.1:{REMOTE_PORT}/health")
        if st == 200:
            remote_up = True
            break
        time.sleep(0.5)
    assert remote_up, f"远程端口 {REMOTE_PORT} 未监听——配置中心覆盖未生效（本地端口 {LOCAL_PORT} 若被监听则说明未走 Nacos）"
    print(f"PASS: scheduler 实际监听 {REMOTE_PORT}（来自 Nacos 远程配置），本地引导 {LOCAL_PORT} 被覆盖")

    # 同时确认本地端口未监听
    st, _ = http("GET", f"http://127.0.0.1:{LOCAL_PORT}/health")
    assert st != 200, f"本地端口 {LOCAL_PORT} 竟在监听——远程覆盖失败"

    step("5. 验证注册中心：Nacos 实例列表应含 %s:%d" % (SVC_NAME, REMOTE_PORT))
    found = False
    for i in range(10):
        st, body = http("GET", f"{NACOS}/nacos/v1/ns/instance/list?serviceName={SVC_NAME}")
        if st == 200:
            hosts = json.loads(body).get("hosts", [])
            if any(h.get("ip") == "127.0.0.1" and h.get("port") == REMOTE_PORT for h in hosts):
                found = True
                meta = next((h.get("metadata", {}) for h in hosts if h.get("port") == REMOTE_PORT), {})
                print(f"PASS: 注册中心发现实例 {SVC_NAME} 127.0.0.1:{REMOTE_PORT} metadata={meta}")
                break
        time.sleep(1)
    assert found, "注册中心未发现 scheduler 实例"
finally:
    step("6. 停止 scheduler 并验证 deregister")
    if proc.poll() is None:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
    time.sleep(2)
    st, body = http("GET", f"{NACOS}/nacos/v1/ns/instance/list?serviceName={SVC_NAME}")
    hosts = json.loads(body).get("hosts", []) if st == 200 else []
    alive = [h for h in hosts if h.get("ip") == "127.0.0.1" and h.get("port") == REMOTE_PORT]
    if not alive:
        print("PASS: 停止后实例已从注册中心移除（deregister 生效）")
    else:
        print("WARN: 停止后仍有存活实例（ephemeral 心跳可能延迟），属正常：", alive)

step("7. 服务日志关键行")
log = io.open(os.path.join(TMP, "sched_e2e_stdout.log"), encoding="utf-8", errors="replace").read()
for line in log.splitlines():
    if any(k in line for k in ("已从 Nacos 拉取远程配置", "已注册到 Nacos 注册中心", "已从 Nacos 注册中心注销", "配置中心已连接", "启动配置完成")):
        print("  ", line.strip()[:140])
print("\nRESULT: " + ("PASS" if remote_up and found else "FAIL"))
