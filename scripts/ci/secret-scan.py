#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
secret-scan.py — 全仓敏感信息扫描门禁（A-2 / G-2）

用途：
  扫描仓库内源码与配置文件中的明文凭据/密钥模式，防止
  platform_config.json admin123 一类硬编码凭据回潮。

扫描类别：
  1. 常见弱口令/占位密钥（admin123 / password / secret / token / api_key 赋值）
  2. 私钥块（BEGIN PRIVATE KEY / RSA / OPENSSH / DSA / EC）
  3. 高熵随机字符串（长度>=24 且含大小写字母+数字，疑似 token/secret）
  4. AWS/阿里云/腾讯云 access key 特征
  5. JWT / Bearer token 硬编码

用法：
  python scripts/ci/secret-scan.py                 # 全仓扫描（跳过 .git/target/node_modules 等）
  python scripts/ci/secret-scan.py --path <dir>    # 指定目录
  python scripts/ci/secret-scan.py --json          # 输出 JSON（CI 解析用）
  python scripts/ci/secret-scan.py --fail-fast     # 命中即退出码 1

退出码：
  0 = 未发现风险
  1 = 发现风险（供 CI 门禁使用）
  2 = 运行错误
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path

# 仓库根（脚本位于 <repo>/scripts/ci/）
REPO_ROOT = Path(__file__).resolve().parents[2]

# 默认跳过目录（含软链、构建、依赖、运行时数据）
SKIP_DIRS = {
    ".git", "node_modules", "target", "dist", "build", "release-pkg",
    ".logs", ".runtime", "__pycache__", ".pytest_cache", ".trae", ".workbuddy",
    ".ous", ".ous_smoke", "ais", "third_party", "data", "artifacts", "exports",
    "log", "temp",
}

# 跳过文件扩展名（二进制/锁文件/媒体）
SKIP_EXT = {
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".woff", ".woff2", ".ttf", ".eot",
    ".pdf", ".zip", ".gz", ".tar", ".7z", ".exe", ".dll", ".so", ".dylib",
    ".lock", ".bin", ".wav", ".mp3", ".mp4", ".db", ".sqlite",
}

# ---- 正则模式（命中即报）----
PATTERNS = [
    # 1. 常见弱口令字面量（JSON/Python/JS 赋值）
    (re.compile(
        r'["\']?(?:password|passwd|pwd|secret|api[_-]?key|token|access[_-]?key)'
        r'["\']?\s*[:=]\s*["\']'
        r'(admin123|123456|password|secret|changeme|12345678|qwerty|letmein)["\']',
        re.IGNORECASE,
    ), "常见弱口令/占位密钥"),

    # 2. 私钥块
    (re.compile(
        r'-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP )?PRIVATE KEY-----',
    ), "私钥块"),

    # 3. 高熵 token（限定在引号字符串内，长度>=24 混合大小写数字；排除代码标识符）
    (re.compile(
        r'["\'`]([A-Za-z0-9_-]{24,64})["\'`]',
    ), "疑似高熵 token（人工复核）"),

    # 4. 云厂商 access key 特征
    (re.compile(r'\b(AKIA[0-9A-Z]{16}|LTAI[0-9A-Za-z]{12,20}|AKID[A-Za-z0-9]{13,20})\b'),
     "云厂商 AccessKey"),

    # 5. JWT 硬编码（eyJ 开头的三段式）
    (re.compile(r'["\']eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}["\']'),
     "硬编码 JWT"),
]

# 高熵 token 需要进一步校验（排除 UUID / 时间戳 / 引用 / 环境变量名）
UUID_RE = re.compile(
    r'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$',
    re.IGNORECASE,
)
# 全大写+下划线/连字符（环境变量名、常量名、模块名、路径段）
ENVVAR_RE = re.compile(r'^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+$')
# 全大写+连字符（如 ROCKSDB-PERFORMANCE-OPTIMIZATION）
UPPER_HYPHEN_RE = re.compile(r'^[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+$')
# 含常见单词分隔的路径/短语/标识符（a-b-c / a_b_c / 2_x_y / Cargo_clippy_x 混合大小写段）
PATHLIKE_RE = re.compile(r'^(?:[A-Za-z0-9]+[_-]){2,}[A-Za-z0-9]+$')


def should_skip(path: Path) -> bool:
    """判断是否跳过该文件。"""
    for part in path.parts:
        if part in SKIP_DIRS:
            return True
    if path.suffix.lower() in SKIP_EXT:
        return True
    # 跳过已存在于 .gitignore 中的运行时类目录（防御性）
    name = path.name.lower()
    if name in {"graph.json", "graph.enterprise.json"} and path.parent.name == "data":
        return True
    return False


def entropy_ok(candidate: str) -> bool:
    """高熵候选人工复核辅助：UUID/纯数字/过短/环境变量名视为误报。"""
    if UUID_RE.match(candidate):
        return False
    if candidate.isdigit():
        return False
    if len(set(candidate)) < 6:  # 字符种类太少，不像随机 token
        return False
    if ENVVAR_RE.match(candidate) or UPPER_HYPHEN_RE.match(candidate):  # 大写环境变量/常量/路径段
        return False
    if PATHLIKE_RE.match(candidate):  # 小写路径/短语
        return False
    # 纯字母单词（可能是标识符/英文单词）
    if candidate.isalpha():
        return False
    return True


def scan_file(path: Path) -> list[dict]:
    """扫描单文件，返回命中列表。"""
    hits = []
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            content = fh.read()
    except Exception:
        return hits
    for lineno, line in enumerate(content.splitlines(), 1):
        for pattern, desc in PATTERNS:
            for m in pattern.finditer(line):
                if desc == "疑似高熵 token（人工复核）":
                    cand = m.group(1)
                    if not entropy_ok(cand):
                        continue
                hits.append({
                    "file": str(path.relative_to(REPO_ROOT)).replace("\\", "/"),
                    "line": lineno,
                    "type": desc,
                    "match": m.group(0)[:120],
                })
    return hits


def main() -> int:
    parser = argparse.ArgumentParser(description="MOX 全仓敏感信息扫描门禁")
    parser.add_argument("--path", default=str(REPO_ROOT), help="扫描根目录")
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--fail-fast", action="store_true", help="命中即退出码 1")
    args = parser.parse_args()

    root = Path(args.path)
    if not root.is_dir():
        print(f"[ERROR] 目录不存在: {root}", file=sys.stderr)
        return 2

    all_hits = []
    scanned = 0
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        if should_skip(path):
            continue
        scanned += 1
        hits = scan_file(path)
        if hits:
            all_hits.extend(hits)

    if args.json:
        print(json.dumps({
            "scanned": scanned,
            "total_hits": len(all_hits),
            "hits": all_hits,
        }, ensure_ascii=False, indent=2))
    else:
        print(f"已扫描 {scanned} 个文件，发现 {len(all_hits)} 处命中")
        for h in all_hits:
            print(f"  [{h['type']}] {h['file']}:{h['line']} -> {h['match']}")

    if all_hits:
        print("[FAIL] 检测到敏感信息，请修复后重跑（secret-scan 门禁）", file=sys.stderr)
        return 1
    print("[PASS] secret-scan 通过：未发现明文敏感信息")
    return 0


if __name__ == "__main__":
    sys.exit(main())
