#!/usr/bin/env python3
"""
MOX 一体化部署工具 — 一键部署到目标服务器
支持: 本地部署 / Docker部署 / 远程部署
"""
import os, sys, json, argparse, subprocess, shutil
from datetime import datetime, timezone, timedelta

CST = timezone(timedelta(hours=8))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def run(cmd, cwd=None, check=True):
    print(f"  $ {cmd}")
    r = subprocess.run(cmd, shell=True, cwd=cwd, capture_output=True, text=True)
    if r.stdout:
        print(r.stdout.strip()[:500])
    if r.returncode != 0 and check:
        print(f"  ERROR: {r.stderr.strip()[:300]}")
        sys.exit(1)
    return r


def deploy_local(args):
    """本地一体化部署: 安装依赖 + 初始化数据库 + 导入数据 + 启动服务"""
    print("\n=== MOX 本地一体化部署 ===")
    server_dir = os.path.join(ROOT, "platform/mox-server")

    print("\n[1/5] 安装Python依赖...")
    run(f"pip install -r {os.path.join(server_dir, 'requirements.txt')}")

    print("\n[2/5] 初始化数据库...")
    init_script = os.path.join(server_dir, "init_db.py")
    if os.path.exists(init_script):
        run(f"python {init_script}", cwd=server_dir)
    else:
        print("  init_db.py 不存在, 跳过(服务启动时自动建表)")

    print("\n[3/5] 导入初始化数据...")
    if args.data:
        import_tool = os.path.join(ROOT, "tools/import_data.py")
        run(f"python {import_tool} {args.data} --db {os.path.join(server_dir, 'data/mox_data.db')}")
    else:
        print("  未指定数据文件, 跳过(使用空数据库)")

    print("\n[4/5] 配置环境...")
    env_file = os.path.join(server_dir, ".env")
    if not os.path.exists(env_file):
        with open(env_file, "w") as f:
            f.write("MOX_HOST=0.0.0.0\nMOX_PORT=8600\nMOX_REDIS=redis://localhost:6379/0\n")
        print(f"  已生成默认配置: {env_file}")

    print("\n[5/5] 启动服务...")
    print(f"  后端: http://localhost:{args.port}")
    print(f"  前端: file:///{os.path.join(ROOT, 'frontend-ui/mox-website/index.html')}")
    print(f"\n  启动命令: cd {server_dir} && python run.py {args.port}")
    if args.start:
        run(f"python run.py {args.port}", cwd=server_dir, check=False)


def deploy_docker(args):
    """Docker一体化部署"""
    print("\n=== MOX Docker 一体化部署 ===")
    compose = os.path.join(ROOT, "docker-compose.yml")
    if not os.path.exists(compose):
        print("  docker-compose.yml 不存在, 生成默认配置...")
        generate_docker_compose()

    print("\n[1/3] 构建镜像...")
    run("docker-compose build", cwd=ROOT)

    print("\n[2/3] 启动服务...")
    run("docker-compose up -d", cwd=ROOT)

    print("\n[3/3] 等待服务就绪...")
    import time
    time.sleep(5)
    run("docker-compose ps", cwd=ROOT)

    print(f"\n=== 部署完成 ===")
    print(f"  后端API: http://localhost:{args.port}/api/health")
    print(f"  前端:    http://localhost:{args.port}/")
    print(f"  管理:    docker-compose logs -f")


def generate_docker_compose():
    content = """version: '3.8'
services:
  mox-server:
    build:
      context: .
      dockerfile: deploy/Dockerfile
    ports:
      - "8600:8600"
    volumes:
      - mox-data:/app/data
      - ./frontend-ui:/app/frontend:ro
    environment:
      - MOX_HOST=0.0.0.0
      - MOX_PORT=8600
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8600/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    restart: unless-stopped

volumes:
  mox-data:
  redis-data:
"""
    with open(os.path.join(ROOT, "docker-compose.yml"), "w") as f:
        f.write(content)

    dockerfile = """FROM python:3.11-slim
WORKDIR /app
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY platform/mox-server/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY platform/mox-server/ ./
COPY tools/ /app/tools/
EXPOSE 8600
CMD ["python", "run.py", "8600"]
"""
    os.makedirs(os.path.join(ROOT, "deploy"), exist_ok=True)
    with open(os.path.join(ROOT, "deploy/Dockerfile"), "w") as f:
        f.write(dockerfile)


def main():
    p = argparse.ArgumentParser(description="MOX 一体化部署工具")
    sub = p.add_subparsers(dest="mode", required=True)

    local = sub.add_parser("local", help="本地部署")
    local.add_argument("--port", default="8600", help="服务端口")
    local.add_argument("--data", default=None, help="初始化数据文件(MXDEF JSON)")
    local.add_argument("--start", action="store_true", help="部署后立即启动")

    docker = sub.add_parser("docker", help="Docker部署")
    docker.add_argument("--port", default="8600", help="服务端口")

    args = p.parse_args()
    if args.mode == "local":
        deploy_local(args)
    elif args.mode == "docker":
        deploy_docker(args)


if __name__ == "__main__":
    main()
