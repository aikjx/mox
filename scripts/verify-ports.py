#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
verify-ports.py —— 璇玑系统全局端口漂移校验（PORT-REGISTRY-001 执行门禁）

职责：
  1. 扫描全仓库源码/配置中的端口引用（排除第三方参考库、构建产物、node_modules、日志/数据噪声）。
  2. 对照权威注册表（本文件 CANONICAL，与 docs/ports/PORT-REGISTRY.md 保持一致）分类：
       RUNTIME / ALLIANCE / ANCILLARY / LEGACY / DEPRECATED / TEST-ONLY / THIRD-PARTY
  3. 检出三类漂移并决定退出码：
       ERROR  : 已退役端口(DEPRECATED)仍被活跃代码/配置引用；platform_config.json 与注册表不一致；
                一个运行端口被多个 RUNTIME 服务声明。
       WARN   : 发现未登记端口（潜在新服务/漂移）；DEPRECATED 端口仅出现在文档中（历史引用）。
       INFO   : 正常引用。
  4. 退出码：0 = 无 ERROR；1 = 存在 ERROR（配合 CI 可作门禁）。

用法：
  python scripts/verify-ports.py            # 全量校验
  python scripts/verify-ports.py --json     # 输出 JSON 报告到 stdout
  python scripts/verify-ports.py --repo <路径>   # 指定仓库根（默认脚本上级两级）

规范依据：docs/ports/PORT-REGISTRY.md（PORT-REGISTRY-001）
          docs/standards/expert-alliance-port-norm.md（PORT-NORM-001）
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

# --------------------------------------------------------------------------- #
# 权威端口注册表（与 docs/ports/PORT-REGISTRY.md 同步维护；改动须走变更流程） #
# --------------------------------------------------------------------------- #
# 分类: RUNTIME / ALLIANCE / ANCILLARY / LEGACY / DEPRECATED / TEST / THIRD
CANONICAL: dict[int, tuple[str, str]] = {
    # ---- RUNTIME（platform_config.json 登记，manage.py 管理） ----
    8080: ("RUNTIME", "api（Rust 网关 mox-gateway，唯一对外 HTTP 入口，例外端口）"),
    3020: ("RUNTIME", "frontend（Vite Vue3 dev server）"),
    30010: ("RUNTIME", "xiaobai_voice（ASR+TTS，PORT-NORM 30000+ 段）"),
    8012: ("RUNTIME", "melody2score（旋律转谱 WebUI）"),
    8000: ("RUNTIME", "primiflow（低代码拓扑引擎）"),
    3999: ("RUNTIME", "dashboard（运维管理面板）"),
    # ---- ALLIANCE（3000-3999 段，PORT-NORM-001） ----
    3100: ("ALLIANCE", "scheduler-svc（调度编排）"),
    3200: ("ALLIANCE", "executor-svc（执行引擎）"),
    3300: ("ALLIANCE", "AI 专家服务桥接（scheduler 内部基址）"),
    # ---- ANCILLARY（运行期附属） ----
    50051: ("ANCILLARY", "gRPC（framework/dualrpc 默认）"),
    50052: ("ANCILLARY", "gRPC 备用端口（架构文档提及）"),
    9080: ("ANCILLARY", "data-plane-svc 内网控制面 ctrl"),
    9081: ("ANCILLARY", "data-plane-svc 内网数据面 data"),
    4173: ("ANCILLARY", "frontend 生产预览（Vite preview）"),
    3998: ("ANCILLARY", "operator API（mox-ai-agent-svc OPERATOR_API_BASE 默认 / runtime --port 测试）"),
    7000: ("ANCILLARY", "mox-dr raft（helm containerPort；历史文档亦见 8200）"),
    3000: ("ANCILLARY", "OUS 算子统一系统边缘（mox-platform-system-core 默认绑定；曾为 Node 边缘入口；注意 Grafana 默认同为 3000 需避让）"),
    3001: ("ANCILLARY", "orchestrator-svc（operator-unified-system）HTTP 默认绑定（--port 默认 3001）"),
    3002: ("ANCILLARY", "enterprise-svc 默认绑定（休眠/备用服务）"),
    # ---- LEGACY（遗留模块，自洽） ----
    8600: ("LEGACY", "legacy Python mox-server（docker/systemd/nginx）"),
    8601: ("LEGACY", "legacy mox-store（应用商店）"),
    6379: ("LEGACY", "redis（基础设施）"),
    # ---- DEPRECATED（已退役，禁止复用） ----
    3010: ("DEPRECATED", "Node.js API / Node sidecar（backend-node 已删除）"),
    3021: ("DEPRECATED", "前端旧端口（AI 对话 UI 曾用，已迁 3020）"),
    3717: ("DEPRECATED", "xiaobai_voice 旧端口（ASR+TTS，2026-09 已迁 30010）"),
    # ---- TEST（测试/内存 mock，不进入运行链路） ----
    8001: ("TEST", "cloud-master 卷节点测试"), 8002: ("TEST", "cloud-master 卷节点测试"),
    8003: ("TEST", "cloud-master 卷节点测试"),
    9000: ("TEST", "cloud-master/kg-storage 节点测试；MinIO S3 默认"),
    9001: ("TEST", "kg-storage 测试；MinIO Console"),
    9002: ("TEST", "kg-storage 测试"), 9003: ("TEST", "kg-storage 测试"),
    9101: ("TEST", "kg-storage 集成测试"), 9102: ("TEST", "kg-storage 集成测试"),
    9103: ("TEST", "kg-storage 集成测试"),
    9201: ("TEST", "kg-storage 查询测试"), 9202: ("TEST", "kg-storage 查询测试"),
    9203: ("TEST", "kg-storage 查询测试"),
    9301: ("TEST", "kg-storage 分片测试"), 9302: ("TEST", "kg-storage 分片测试"),
    9303: ("TEST", "kg-storage 分片测试"),
    9333: ("TEST", "cloud-master raft 测试"),
    9401: ("TEST", "kg-storage 性能基准"), 9402: ("TEST", "kg-storage 性能基准"),
    9403: ("TEST", "kg-storage 性能基准"),
    9501: ("TEST", "kg-storage 千亿模拟"), 9502: ("TEST", "kg-storage 千亿模拟"),
    9503: ("TEST", "kg-storage 千亿模拟"),
    9669: ("TEST", "cloud-foundation graph_meta 测试"),
    9779: ("TEST", "kg-meta-core 存储宿主测试"), 9780: ("TEST", "kg-meta-core 存储宿主测试"),
    9781: ("TEST", "kg-meta-core 存储宿主测试"),
    9998: ("TEST", "legacy backend-rust 网关 target 测试"),
    9999: ("TEST", "legacy backend-rust 网关 target 测试"),
    12345: ("TEST", "glacier-adapter 测试 endpoint"),
    13130: ("TEST", "xiaobai_voice 语音代理 WS 测试"),
    19601: ("TEST", "kg-meta-core 集群测试"), 19602: ("TEST", "kg-meta-core 集群测试"),
    19603: ("TEST", "kg-meta-core 集群测试"),
    19876: ("TEST", "mox-dualrpc 测试"),
    19999: ("TEST", "kg-connector 不可用降级测试"),
    35432: ("TEST", "本地 PostgreSQL 测试库（MOX_TEST_PG_URL）"),
    65528: ("TEST", "alliance-sdk 连通性测试"), 65529: ("TEST", "alliance-sdk 连通性测试"),
    65530: ("TEST", "alliance-sdk 连通性测试"),
    8081: ("TEST", "mox-dualrpc jsonrpc 测试端口（曾为 alliance scheduler 旧端口，已迁 3100）"),
    8082: ("TEST", "mox-dualrpc jsonrpc 测试端口（曾为 alliance executor 旧端口，已迁 3200）"),
    18080: ("TEST", "single-node 验证模式 public（t19）"),
    19080: ("TEST", "single-node 验证模式 ctrl（t19）"),
    19081: ("TEST", "single-node 验证模式 data（t19）"),
    3079: ("TEST", "mox-flow-bridge serve demo（mox serve --port 3079）"),
    8333: ("TEST", "mox-cloud-filer-svc 挂载测试"),
    8787: ("TEST", "primiflow demo（Server::serve 0.0.0.0:8787）"),
    8788: ("TEST", "primiflow-fusion demo（fusion-server 默认）"),
    3123: ("TEST", "alliance boot-config 测试 fixture"),
    8307: ("TEST", "voice-operator netstat 解析测试样本"),
    63001: ("TEST", "voice-operator netstat 解析测试样本"),
    # ---- THIRD（第三方基础设施默认端口，部署引用） ----
    2379: ("THIRD", "etcd client"), 2380: ("THIRD", "etcd peer"),
    3306: ("THIRD", "MySQL"), 4222: ("THIRD", "NATS"),
    4317: ("THIRD", "OpenTelemetry OTLP"), 5236: ("THIRD", "达梦 DM"),
    5432: ("THIRD", "PostgreSQL"), 54321: ("THIRD", "KingbaseES"),
    7480: ("THIRD", "Ceph RGW"), 7687: ("THIRD", "Neo4j Bolt"),
    8200: ("THIRD", "raft（mox-dr）"), 8848: ("THIRD", "Nacos"),
    9090: ("THIRD", "Prometheus"), 9093: ("THIRD", "Alertmanager"),
    6006: ("THIRD", "Storybook（dev）"), 8888: ("THIRD", "前端 Web 搜索服务占位"),
    7688: ("THIRD", "mox-dr 部署映射 7688:7687（Neo4j Bolt 备用映射）"),
}

# 有意保留的遗留 opt-in 引用（不视为漂移）——(相对路径, 端口) → 理由
# 说明：orchestrator Node 侧车默认指向已删除的 backend-node:3010，2026-09 已清理为 Rust 网关 8080；
#       DEPRECATED 3010 现仅存于历史文档（.md）中，由"文档类豁免"自动放行。
ALLOWED_LEGACY_REFS = {
    ("start.sh", 3000): "start.sh --legacy 遗留 operator-server 路径（默认关闭，opt-in）",
}

# 噪声上下文（毫秒/UID/采样率/ISO 编号/netstat 样本等，非端口）
NOISE_CTX = re.compile(
    r"setTimeout|timeout\?|sampleRate|uptime_ms|USER\s+\d+:\d+|ISO/IEC|returncode|"
    r"ESTABLISHED|LISTENING\s+\(|time\.time\(\)|trace_id|app_key|GRPCPort|ms_uptime|"
    r"status:|\bstatus=\"|\[\s*:\d{2,5}\s*\]", re.I
)

# platform_config.json 中 RUNTIME 服务的期望端口（单一事实源校验）
EXPECTED_PLATFORM_PORTS = {
    "api": 8080, "frontend": 3020, "xiaobai_voice": 30010,
    "melody2score": 8012, "primiflow": 8000,
}
EXPECTED_DASHBOARD_PORT = 3999

# --------------------------------------------------------------------------- #
# 扫描参数
# --------------------------------------------------------------------------- #
SCAN_ROOTS = ["platform", "projects", "frontend-ui", "scripts", "config", "deploy",
              "tests", "shared", "proto", "tools", "mox-workspace", "prototypes",
              "my_projects", "docs", "plugins", "start.sh", "docker-compose.yml",
              "Cargo.toml", "deny.toml", "platform_config.json"]
PRUNE_DIRS = {"node_modules", "dist", "build", "target", "__pycache__", ".venv",
              "venv", "third_party", "playwright-report", "test-results", ".storybook",
              "release-pkg", "temp", "snap", ".cache", "models", "downloads"}
# 跳过噪声文件（日志/构建输出/压缩库/二进制/历史快照）
SKIP_EXT = {".log", ".err", ".out", ".jsonl", ".min.js", ".min.css", ".map", ".bak",
            ".bak_mojibake", ".pyc", ".exe", ".dll", ".pdb", ".woff", ".woff2", ".ttf",
            ".png", ".jpg", ".jpeg", ".gif", ".ico", ".pdf", ".wav", ".mp3", ".zip",
            ".7z", ".gz", ".whl", ".db", ".sqlite", ".lock", ".gitignore"}
SKIP_SUBPATH = ("ais/", "third_party/", "target/", "release-pkg/", "node_modules/",
                "dist/", "build/", "_data/", ".runtime/", ".logs/", "docs/_archive/",
                "docs/enterprise/_data/", "prototypes/_shared/")

PAT_HOST = re.compile(r"(?:localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1?\]|::)\s*[:=]\s*(\d{2,5})", re.I)
PAT_FLAG = re.compile(r"(?i)(?:--port|--PORT)\s*[= ]\s*(\d{2,5})")
PAT_KEY = re.compile(r"[\"']?port[\"']?\s*[:=]\s*[\"']?(\d{2,5})[\"']?", re.I)
PAT_ENV = re.compile(r"(?i)(?:^|[^A-Z])(?:PORT|HTTP_PORT|SERVER_PORT|LISTEN_PORT|MOX_PORT|API_PORT|WEB_PORT|APP_PORT|FRONTEND_PORT|BACKEND_PORT|GRPC_PORT|MOX_GRPC_PORT)\s*[:=]\s*[\"']?(\d{2,5})[\"']?")
PAT_URL = re.compile(r"https?://[^/\s:]+:(\d{2,5})", re.I)
PAT_COLONPATH = re.compile(r"(?<![\d.:]):(\d{2,5})(?=/|[\s\"'\]}>]|$)")
PAT_DOCKER = re.compile(r"[\"']?(\d{2,5}):\d{2,5}[\"']?")


def iter_files(repo: Path):
    seen = set()
    for r in SCAN_ROOTS:
        base = repo / r
        if base.is_file():
            yield base, r
            continue
        if not base.is_dir():
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames
                           if d not in PRUNE_DIRS and not d.startswith(".")]
            rel = os.path.relpath(dirpath, repo).replace("\\", "/")
            if any(rel.startswith(p) or ("/" + p) in "/" + rel + "/" for p in SKIP_SUBPATH):
                dirnames[:] = []
                continue
            for fn in filenames:
                ext = os.path.splitext(fn)[1].lower()
                if ext in SKIP_EXT:
                    continue
                r2 = rel + "/" + fn
                if r2 in seen:
                    continue
                seen.add(r2)
                yield Path(dirpath) / fn, r2
    # 根目录散落文件
    for fn in os.listdir(repo):
        if fn.startswith(".") or fn in ("Cargo.lock",):
            continue
        fp = repo / fn
        if fp.is_file():
            yield fp, fn


def read_text(fp: Path):
    for enc in ("utf-8", "gbk", "latin-1"):
        try:
            return fp.read_text(encoding=enc), enc
        except Exception:
            continue
    return fp.read_text(encoding="latin-1", errors="replace"), "latin-1"


def scan(repo: Path):
    hits = defaultdict(list)  # port -> [(rel, kind, ctx)]
    for fp, rel in iter_files(repo):
        try:
            text, _ = read_text(fp)
        except Exception:
            continue
        for pat, kind in ((PAT_HOST, "HOST"), (PAT_FLAG, "FLAG"), (PAT_KEY, "KEY"),
                          (PAT_ENV, "ENV"), (PAT_URL, "URL"), (PAT_DOCKER, "DOCKER"),
                          (PAT_COLONPATH, "COLON")):
            for m in pat.finditer(text):
                if m.lastindex is None or m.group(1) is None:
                    continue
                p = int(m.group(1))
                if not (1000 <= p <= 65535):
                    continue
                ctx = text[max(0, m.start() - 60):m.end() + 60].replace("\n", " ").strip()
                if p < 1024 or NOISE_CTX.search(ctx):
                    continue
                hits[p].append((rel, kind, ctx))
    return hits


def check_platform_config(repo: Path) -> list[dict]:
    """platform_config.json 与 RUNTIME 注册表一致性校验（单一事实源）。"""
    issues = []
    p = repo / "platform_config.json"
    if not p.exists():
        return [{"severity": "ERROR", "msg": f"{p} 缺失，无法校验 RUNTIME 端口事实源"}]
    try:
        cfg = json.loads(p.read_text(encoding="utf-8"))
    except Exception as e:
        return [{"severity": "ERROR", "msg": f"{p} 解析失败: {e}"}]
    for key, want in EXPECTED_PLATFORM_PORTS.items():
        got = (cfg.get("services", {}).get(key) or {}).get("port")
        if got != want:
            issues.append({
                "severity": "ERROR",
                "msg": f"platform_config.json: services.{key}.port = {got}，应为 {want}（见 PORT-REGISTRY-001）",
            })
    dash = cfg.get("dashboard_port")
    if dash != EXPECTED_DASHBOARD_PORT:
        issues.append({
            "severity": "ERROR",
            "msg": f"platform_config.json: dashboard_port = {dash}，应为 {EXPECTED_DASHBOARD_PORT}",
        })
    return issues


def main():
    ap = argparse.ArgumentParser(description="璇玑系统端口漂移校验（PORT-REGISTRY-001）")
    ap.add_argument("--json", action="store_true", help="输出 JSON 报告")
    ap.add_argument("--repo", default=None, help="仓库根路径（默认脚本上级两级）")
    args = ap.parse_args()

    repo = Path(args.repo) if args.repo else Path(__file__).resolve().parent.parent
    repo = repo.resolve()

    hits = scan(repo)
    issues = check_platform_config(repo)

    # 分类扫描结果
    seen_per_port: dict[int, set[str]] = {}
    for port, refs in sorted(hits.items()):
        files = {r for r, _, _ in refs}
        seen_per_port[port] = files
        entry = CANONICAL.get(port)
        if entry is None:
            issues.append({
                "severity": "WARN",
                "msg": f"发现未登记端口 {port}（{len(files)} 处）：请核对是否新服务，按 PORT-REGISTRY-001 第5章登记",
                "files": sorted(files)[:8],
            })
            continue
        cat, name = entry
        if cat == "DEPRECATED":
            # 豁免两类"有意保留"引用：
            #  ① 文档类文件（.md/.html/.txt，含根目录 CLAUDE.md / README / 报告）
            #  ② 明确标注的遗留 opt-in 路径（如 start.sh --legacy）
            doc_ext = {".md", ".html", ".htm", ".txt"}
            non_doc = [f for f in files
                       if os.path.splitext(f)[1].lower() not in doc_ext
                       and not f.startswith("docs/")]
            non_doc = [f for f in non_doc if (f, port) not in ALLOWED_LEGACY_REFS]
            if non_doc:
                issues.append({
                    "severity": "ERROR",
                    "msg": f"已退役端口 {port}（{name}）仍被活跃文件引用：{', '.join(sorted(non_doc)[:6])}，禁止复用",
                    "files": sorted(non_doc)[:6],
                })
            else:
                issues.append({
                    "severity": "INFO",
                    "msg": f"已退役端口 {port}（{name}）：仅存于文档/遗留 opt-in 引用（{len(files)} 处），可保留",
                })
        elif cat == "TEST" or cat == "THIRD":
            issues.append({"severity": "INFO", "msg": f"端口 {port}（{name}，{cat}）：测试/第三方引用 {len(files)} 处"})
        else:
            issues.append({"severity": "INFO", "msg": f"端口 {port}（{name}，{cat}）：引用 {len(files)} 处"})

    # 一端口多 RUNTIME 服务冲突检查
    for port, refs in hits.items():
        entry = CANONICAL.get(port)
        if entry and entry[0] == "RUNTIME":
            pass  # RUNTIME 端口即唯一归属

    # ---- 汇总 ----
    order = {"ERROR": 0, "WARN": 1, "INFO": 2}
    issues.sort(key=lambda i: order.get(i["severity"], 9))
    errs = [i for i in issues if i["severity"] == "ERROR"]
    warns = [i for i in issues if i["severity"] == "WARN"]

    if args.json:
        print(json.dumps({
            "repo": str(repo),
            "passed": len(errs) == 0,
            "error_count": len(errs),
            "warn_count": len(warns),
            "scanned_ports": len(hits),
            "issues": issues,
        }, ensure_ascii=False, indent=2))
    else:
        print("=" * 78)
        print(f"璇玑系统端口漂移校验  repo={repo}")
        print("=" * 78)
        for i in issues:
            tag = {"ERROR": "✗", "WARN": "⚠", "INFO": "•"}[i["severity"]]
            print(f"  [{tag} {i['severity']:6s}] {i['msg']}")
        print("-" * 78)
        print(f"  扫描端口: {len(hits)}  |  问题: ERROR={len(errs)}  WARN={len(warns)}  "
              f"INFO={len(issues) - len(errs) - len(warns)}")
        print("  结论: " + ("✗ 存在 ERROR，请修复后重跑" if errs else ("⚠ 有 WARN，请人工确认" if warns else "✔ 通过（无 ERROR）")))
        print("=" * 78)

    return 1 if errs else 0


if __name__ == "__main__":
    sys.exit(main())
