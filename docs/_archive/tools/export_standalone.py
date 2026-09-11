#!/usr/bin/env python3
"""
MOX 独立导出工具 — 将应用导出为可独立运行的Docker包
脱离MOX系统，接收方开箱即用
用法:
  python tools/export_standalone.py --app-key my-crm --output ./dist
  python tools/export_standalone.py --from-dir ./my-app --name "CRM" --output ./dist
"""
import os, sys, json, sqlite3, zipfile, shutil, argparse, tarfile
from datetime import datetime, timezone, timedelta

CST = timezone(timedelta(hours=8))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
META_DB = os.path.join(ROOT, "platform", "mox-server", "mox_meta.db")
BIZ_DB = os.path.join(ROOT, "platform", "mox-server", "mox_business.db")


def now_iso():
    return datetime.now(CST).isoformat()


def export_table(conn, table):
    try:
        cur = conn.execute(f"SELECT * FROM {table}")
        cols = [d[0] for d in cur.description]
        return [dict(zip(cols, row)) for row in cur.fetchall()]
    except Exception:
        return []


def generate_dockerfile(app_name, port=8600):
    return f"""FROM python:3.11-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends curl && rm -rf /var/lib/apt/lists/*
COPY backend/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY backend/ ./
COPY frontend/ /app/frontend/
COPY data/ /app/data/
EXPOSE {port}
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD curl -f http://localhost:{port}/api/health || exit 1
CMD ["python", "run.py", "{port}"]
"""


def generate_compose(app_name, port=8600):
    return f"""version: '3.8'
services:
  {app_name}:
    build: .
    ports:
      - "{port}:{port}"
    volumes:
      - app-data:/app/data
    restart: unless-stopped
    environment:
      - MOX_HOST=0.0.0.0
      - MOX_PORT={port}
volumes:
  app-data:
"""


def generate_readme(app_name, version, description, port):
    return f"""# {app_name} - 独立运行包

> 版本: {version} | 基于 MOX 低代码平台

## 简介

{description}

## 快速启动

### Docker 方式（推荐）

```bash
docker-compose up -d --build
```

访问: http://localhost:{port}

### 直接运行

```bash
cd backend
pip install -r requirements.txt
python run.py {port}
```

## 目录结构

```
{app_name}-standalone/
├── docker-compose.yml    # Docker编排
├── Dockerfile            # 镜像构建
├── start.sh              # 启动脚本
├── README.md             # 本文档
├── backend/              # 后端服务(MOX运行时+应用逻辑)
├── frontend/             # 前端静态文件
└── data/                 # 初始化数据
```

## 数据说明

- 应用数据已预置在 data/ 目录
- Docker 方式数据持久化在 app-data volume
- 直接运行方式数据在 backend/data/ 目录

## 技术栈

- 后端: Python + FastAPI + SQLite
- 前端: 纯静态 HTML/JS/CSS
- 数据: MXDEF 标准化格式

---
*由 MOX 低代码平台一键导出*
"""


def generate_start_sh(port):
    return f"""#!/bin/bash
set -e
cd "$(dirname "$0")"
echo "启动 {app_name}..."
cd backend
pip install -q -r requirements.txt 2>/dev/null || true
python run.py {port}
"""


def export_standalone(app_key, app_name, version, description, output_dir, port=8600):
    ts = datetime.now(CST).strftime("%Y%m%d-%H%M%S")
    pkg_name = f"{app_key}-standalone-{version}"
    pkg_dir = os.path.join(output_dir, pkg_name)

    # 清理并创建目录
    if os.path.exists(pkg_dir):
        shutil.rmtree(pkg_dir)
    for sub in ["backend", "frontend", "data"]:
        os.makedirs(os.path.join(pkg_dir, sub), exist_ok=True)

    print(f"[1/5] 导出应用数据...")
    # 导出元数据
    meta_conn = sqlite3.connect(META_DB)
    meta_data = {}
    for table in ["dsql_sqls", "datasources", "apps", "roles", "field_permissions", "kg_vertices", "kg_edges"]:
        meta_data[table] = export_table(meta_conn, table)
    meta_conn.close()
    with open(os.path.join(pkg_dir, "data", "meta.json"), "w", encoding="utf-8") as f:
        json.dump(meta_data, f, ensure_ascii=False, indent=2)

    # 导出业务数据
    if os.path.exists(BIZ_DB):
        biz_conn = sqlite3.connect(BIZ_DB)
        biz_data = {}
        for table in ["products", "news", "cases", "team", "banners", "messages"]:
            biz_data[table] = export_table(biz_conn, table)
        biz_conn.close()
        with open(os.path.join(pkg_dir, "data", "business.json"), "w", encoding="utf-8") as f:
            json.dump(biz_data, f, ensure_ascii=False, indent=2)

    print(f"[2/5] 复制MOX运行时...")
    # 复制后端核心文件（精简版）
    backend_src = os.path.join(ROOT, "platform", "mox-server")
    if os.path.exists(backend_src):
        for item in os.listdir(backend_src):
            src = os.path.join(backend_src, item)
            dst = os.path.join(pkg_dir, "backend", item)
            if item in ["mox", "run.py", "requirements.txt", "start_server.py"]:
                if os.path.isdir(src):
                    shutil.copytree(src, dst, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
                else:
                    shutil.copy2(src, dst)

    print(f"[3/5] 复制前端文件...")
    frontend_src = os.path.join(ROOT, "frontend-ui", "mox-website")
    if os.path.exists(frontend_src):
        shutil.copy2(os.path.join(frontend_src, "index.html"),
                     os.path.join(pkg_dir, "frontend", "index.html"))

    print(f"[4/5] 生成配置文件...")
    with open(os.path.join(pkg_dir, "Dockerfile"), "w") as f:
        f.write(generate_dockerfile(app_name, port))
    with open(os.path.join(pkg_dir, "docker-compose.yml"), "w") as f:
        f.write(generate_compose(app_key, port))
    with open(os.path.join(pkg_dir, "README.md"), "w", encoding="utf-8") as f:
        f.write(generate_readme(app_name, version, description, port))
    with open(os.path.join(pkg_dir, "start.sh"), "w") as f:
        f.write(generate_start_sh(port))
    os.chmod(os.path.join(pkg_dir, "start.sh"), 0o755)

    print(f"[5/5] 打包...")
    # 打包为tar.gz
    tar_path = os.path.join(output_dir, f"{pkg_name}.tar.gz")
    with tarfile.open(tar_path, "w:gz") as tar:
        tar.add(pkg_dir, arcname=pkg_name)

    # 计算大小
    def get_dir_size(path):
        total = 0
        for dp, dn, fn in os.walk(path):
            for f in fn:
                total += os.path.getsize(os.path.join(dp, f))
        return total

    dir_size = get_dir_size(pkg_dir)
    tar_size = os.path.getsize(tar_path)

    print(f"\n{'='*60}")
    print(f"  独立导出完成!")
    print(f"  应用: {app_name} ({app_key}) v{version}")
    print(f"  目录: {pkg_dir}")
    print(f"  压缩包: {tar_path}")
    print(f"  目录大小: {dir_size/1024/1024:.2f} MB")
    print(f"  压缩包大小: {tar_size/1024/1024:.2f} MB")
    print(f"  访问端口: {port}")
    print(f"{'='*60}")
    print(f"\n接收方使用:")
    print(f"  tar xzf {pkg_name}.tar.gz")
    print(f"  cd {pkg_name}")
    print(f"  docker-compose up -d --build")
    print(f"  # 访问 http://localhost:{port}")

    return tar_path


def main():
    p = argparse.ArgumentParser(description="MOX 独立导出工具")
    p.add_argument("--app-key", default="mox", help="应用标识")
    p.add_argument("--name", default=None, help="应用名称")
    p.add_argument("--version", default="1.0.0", help="版本号")
    p.add_argument("--description", default="基于MOX低代码平台构建的应用", help="描述")
    p.add_argument("--output", default=os.path.join(ROOT, "dist"), help="输出目录")
    p.add_argument("--port", type=int, default=8600, help="服务端口")
    args = p.parse_args()

    os.makedirs(args.output, exist_ok=True)
    name = args.name or args.app_key
    export_standalone(args.app_key, name, args.version, args.description, args.output, args.port)


if __name__ == "__main__":
    main()
