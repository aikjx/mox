# -*- coding: utf-8 -*-
"""xiaobai_voice 3717 → 30010 全链路迁移 + orchestrator 侧车 3010 清理（一次性批量执行）。"""
import os, io, sys, pathlib
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
ROOT = pathlib.Path(r"D:\a10\aikjx\gitcode\infotopograph")

def sub(path, mapping, note=""):
    """mapping: list of (old,new) or (old,new) single; 全部替换并报告次数。"""
    fp = ROOT / path
    if not fp.exists():
        print(f"  [MISS] {path}")
        return
    text = fp.read_text(encoding="utf-8")
    orig = text
    total = 0
    if isinstance(mapping, tuple):
        mapping = [mapping]
    for old, new in mapping:
        n = text.count(old)
        if n:
            text = text.replace(old, new)
            total += n
    if text != orig:
        fp.write_text(text, encoding="utf-8")
        print(f"  [OK  ] {path}  (替换 {total} 处)  {note}")
    else:
        print(f"  [----] {path}  无变化  {note}")

def rename(path, newname, note=""):
    fp = ROOT / path
    if not fp.exists():
        print(f"  [MISS] {path}")
        return
    target = fp.parent / newname
    fp.rename(target)
    print(f"  [REN ] {path} -> {newname}  {note}")

print("======== A. Python xiaobai_voice: 3717 -> 30010 ========")
for p in [
    "projects/xiaobai_voice/README.md",
    "projects/xiaobai_voice/xiaobai_voice/cli.py",
    "projects/xiaobai_voice/xiaobai_voice/__init__.py",
    "projects/xiaobai_voice/xiaobai_voice/config/default_config.yaml",
    "projects/xiaobai_voice/xiaobai_voice/desktop/app.py",
    "projects/xiaobai_voice/xiaobai_voice/desktop/ball_widget.py",
    "projects/xiaobai_voice/xiaobai_voice/desktop/main_window.py",
    "projects/xiaobai_voice/xiaobai_voice/proxy/voice_proxy.py",
    "projects/xiaobai_voice/xiaobai_voice/service/main.py",
]:
    sub(p, ("3717", "30010"))

print("======== B. Rust orchestrator: voice 3717 -> 30010 ========")
for p in [
    "platform/domains/platform/svc/mox-platform-orchestrator-svc/src/subservers.rs",
    "platform/domains/platform/svc/mox-platform-orchestrator-svc/src/routes/voice_proxy.rs",
]:
    sub(p, ("3717", "30010"))
# main.rs：仅注释含 3717（3010 侧车单独处理）
sub("platform/domains/platform/svc/mox-platform-orchestrator-svc/src/main.rs", ("3717", "30010"), "(注释)")

print("======== C. Rust voice crate: server_3717 -> server_30010 + 3717 -> 30010 ========")
# 1) 模块/feature 改名（先做，避免把 server_30010 里的 30010 误伤——按序先替换名字再替换裸 3717）
for p in [
    "platform/domains/voice/svc/mox-voice-operator-svc/Cargo.toml",
    "platform/domains/voice/svc/mox-voice-operator-svc/src/lib.rs",
    "platform/domains/voice/svc/mox-voice-operator-svc/src/server_3717.rs",
    "platform/domains/voice/svc/mox-voice-operator-svc/README.md",
    "platform/domains/voice/svc/mox-voice-desktop-app/Cargo.toml",
    "platform/domains/voice/svc/mox-voice-desktop-app/src/lib.rs",
    "platform/domains/voice/svc/mox-voice-desktop-app/src/ball_widget.rs",
    "platform/domains/voice/svc/mox-voice-desktop-app/src/main.rs",
    "scripts/temp/_main_orig.rs",
    "scripts/validation/fix_pass2.py",
]:
    sub(p, [("server-3717", "server-30010"), ("server_3717", "server_30010"), ("3717", "30010")])
# 2) 文件改名
rename("platform/domains/voice/svc/mox-voice-operator-svc/src/server_3717.rs",
       "server_30010.rs", "(已改内容，随包引用同步)")
rename("scripts/temp/_probe3717.py", "_probe30010.py")

print("======== D. 配置 / 脚本 ========")
sub("platform_config.json", [
    ('"port": 3717', '"port": 30010'),
    ("端口 3717，供 Rust 网关 /voice/** 代理调用", "端口 30010，供 Rust 网关 /voice/** 代理调用"),
    ('"_probe3717.py"', '"_probe30010.py"'),
])
sub("scripts/server-manage.py", [
    ("端口 3717", "端口 30010"),
    ('"port": 3717', '"port": 30010'),
])
sub("scripts/validation/verify_tts_rust_fullstack.py", ("3717", "30010"))
sub("scripts/README.md", [
    ('_probe3717.py', '_probe30010.py'),
    ('| xiaobai_voice | 小白语音服务（ASR + TTS） | 3717 | Python | ✅ |',
     '| xiaobai_voice | 小白语音服务（ASR + TTS） | 30010 | Python | ✅ |'),
])
# _probe30010.py 内容同步
sub("scripts/temp/_probe30010.py", ("3717", "30010"))
sub("frontend-ui/vite.config.js", ("3717", "30010"), "(注释)")

print("======== E. orchestrator 侧车 3010 清理 ========")
sub("platform/domains/platform/svc/mox-platform-orchestrator-svc/src/main.rs", [
    ('unwrap_or_else(|_| "http://127.0.0.1:3010".to_string())',
     'unwrap_or_else(|_| "http://127.0.0.1:8080".to_string()) // backend-node 已删除(2026-09)，默认指向接管其职责的 Rust 网关 8080'),
])
sub("platform/domains/platform/svc/mox-platform-orchestrator-svc/src/handlers/ai_engine.rs", [
    ('NodeSidecarClient::new("http://127.0.0.1:3010")',
     'NodeSidecarClient::new("http://127.0.0.1:8080") // backend-node 已删除(2026-09)，默认指向 Rust 网关 8080'),
])
sub("platform/domains/platform/svc/mox-platform-orchestrator-svc/src/sidecar/mod.rs", [
    ("node_sidecar：Node 127.0.0.1:3010 内部 API 调用（含 fallback 与指标）",
     "node_sidecar：Node 内部 API 客户端（backend-node 已删除 2026-09，默认指向 Rust 网关 127.0.0.1:8080；不可达时本地 fallback 与指标）"),
])
sub("platform/domains/platform/svc/mox-platform-orchestrator-svc/src/sidecar/node_sidecar.rs", [
    ("Node sidecar 客户端：本地 127.0.0.1:3010 的内部 endpoints",
     "Node sidecar 客户端：本地 127.0.0.1:8080（Rust 网关，已接管 backend-node）的内部 endpoints（backend-node 已删除，不可达时走本地 fallback）"),
])
sub("frontend-ui/test-api.html", ("localhost:3010", "localhost:8080"), "(测试页指向现行 API 网关)")

print("\n全部完成。")
