#!/usr/bin/env python3
"""
MOX 一键打包工具 — 生成可分发的完整发布包
包含: 内核代码 + 前端静态文件 + 初始化数据 + 配置模板 + 部署脚本
"""
import os, sys, json, shutil, argparse, tarfile, zipfile, hashlib
from datetime import datetime, timezone, timedelta

CST = timezone(timedelta(hours=8))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

INCLUDE_DIRS = [
    "platform/mox-server",
    "frontend-ui/mox-website",
    "frontend-ui/mox-console",
    "tools",
    "docs",
    "deploy",
]
INCLUDE_FILES = [
    "README.md",
    "requirements.txt",
    "docker-compose.yml",
    ".env.example",
]
EXCLUDE_PATTERNS = [
    "__pycache__", ".pyc", ".pyo", ".git", ".svn",
    "node_modules", ".DS_Store", "Thumbs.db",
    "*.log", "*.tmp", ".env", "data/mox_data.db",
    "exports/", "*.gz",
]


def should_exclude(path):
    name = os.path.basename(path)
    for pat in EXCLUDE_PATTERNS:
        if pat.startswith("*"):
            if name.endswith(pat[1:]):
                return True
        elif pat in path:
            return True
    return False


def collect_files():
    files = []
    for d in INCLUDE_DIRS:
        full = os.path.join(ROOT, d)
        if not os.path.exists(full):
            continue
        for dp, dn, fn in os.walk(full):
            dn[:] = [x for x in dn if not should_exclude(os.path.join(dp, x))]
            for f in fn:
                fp = os.path.join(dp, f)
                if not should_exclude(fp):
                    files.append((fp, os.path.relpath(fp, ROOT)))
    for f in INCLUDE_FILES:
        fp = os.path.join(ROOT, f)
        if os.path.exists(fp):
            files.append((fp, f))
    return files


def make_manifest(files, version):
    t0 = datetime.now(CST)
    manifest = {
        "product": "MOX Lowcode Platform",
        "version": version,
        "packaged_at": t0.isoformat(),
        "file_count": len(files),
        "total_size": sum(os.path.getsize(f[0]) for f in files),
        "components": {
            "backend": "mox-server (FastAPI + SQLite + Redis)",
            "frontend": "mox-website + mox-console (纯静态SPA)",
            "tools": "export/import/validate/package/deploy",
            "docs": "architecture + data-exchange + deployment",
        },
        "layers": {
            "L1_kernel": "SQL模板/数据源/应用/权限配置",
            "L2_data": "业务数据/知识图谱(按app_key隔离)",
            "L3_runtime": "审计日志/缓存(不打包,系统自生)",
        },
        "checksums": {},
    }
    for fp, rel in files:
        with open(fp, "rb") as f:
            manifest["checksums"][rel] = hashlib.sha256(f.read()).hexdigest()[:16]
    return manifest


def main():
    p = argparse.ArgumentParser(description="MOX 一键打包工具")
    p.add_argument("--version", default="1.0.0", help="版本号")
    p.add_argument("--format", choices=["zip", "tar.gz", "both"], default="zip", help="打包格式")
    p.add_argument("--output", default="dist", help="输出目录")
    p.add_argument("--with-data", action="store_true", help="包含初始化数据库(默认不含)")
    p.add_argument("--name", default=None, help="包名前缀(默认mox-platform)")
    args = p.parse_args()

    t0 = datetime.now(CST)
    ts = t0.strftime("%Y%m%d-%H%M%S")
    name = args.name or "mox-platform"
    base = f"{name}-{args.version}-{ts}"

    print("[1/4] 收集文件...")
    files = collect_files()
    if args.with_data:
        db = os.path.join(ROOT, "platform/mox-server/data/mox_data.db")
        if os.path.exists(db):
            files.append((db, "platform/mox-server/data/mox_data.db"))
            print(f"  包含初始化数据库")
    print(f"  共 {len(files)} 个文件")

    print("[2/4] 生成manifest...")
    manifest = make_manifest(files, args.version)

    out_dir = args.output
    if not os.path.isabs(out_dir):
        out_dir = os.path.join(ROOT, out_dir)
    os.makedirs(out_dir, exist_ok=True)

    work_dir = os.path.join(out_dir, base)
    if os.path.exists(work_dir):
        shutil.rmtree(work_dir)
    os.makedirs(work_dir)

    for fp, rel in files:
        dst = os.path.join(work_dir, rel)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copy2(fp, dst)
    with open(os.path.join(work_dir, "manifest.json"), "w", encoding="utf-8") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=2)

    print("[3/4] 压缩打包...")
    outputs = []
    if args.format in ("zip", "both"):
        zip_path = os.path.join(out_dir, base + ".zip")
        with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
            for dp, dn, fn in os.walk(work_dir):
                for f in fn:
                    fp = os.path.join(dp, f)
                    zf.write(fp, os.path.relpath(fp, work_dir))
        outputs.append(zip_path)
        print(f"  ZIP: {zip_path} ({os.path.getsize(zip_path)/1024/1024:.2f} MB)")

    if args.format in ("tar.gz", "both"):
        tar_path = os.path.join(out_dir, base + ".tar.gz")
        with tarfile.open(tar_path, "w:gz") as tf:
            tf.add(work_dir, arcname=base)
        outputs.append(tar_path)
        print(f"  TAR: {tar_path} ({os.path.getsize(tar_path)/1024/1024:.2f} MB)")

    shutil.rmtree(work_dir)

    elapsed = (datetime.now(CST) - t0).total_seconds()
    print(f"\n[4/4] 完成! 耗时 {elapsed:.2f}s")
    print(f"  版本: {args.version}")
    print(f"  文件: {len(files)} 个")
    print(f"  大小: {manifest['total_size']/1024/1024:.2f} MB (未压缩)")
    for o in outputs:
        print(f"  输出: {o}")


if __name__ == "__main__":
    main()
