# -*- coding: utf-8 -*-
"""
mox-server 启动入口
===================
用法：
    python run.py            # 默认 0.0.0.0:8600
    python run.py 9000       # 指定端口
环境变量：
    MOX_CACHE=memory|redis   # 缓存驱动
    MOX_PORT=8600
"""
import os
import sys

import uvicorn


def main():
    port = int(os.environ.get("MOX_PORT", sys.argv[1] if len(sys.argv) > 1 else "8600"))
    host = os.environ.get("MOX_HOST", "0.0.0.0")
    print("=" * 64)
    print("mox-server 低代码平台运行服务")
    print(f"  DSQL 动态SQL引擎 + 自研知识图谱引擎")
    print(f"  监听: http://{host}:{port}")
    print(f"  健康: http://{host}:{port}/api/health")
    print(f"  控制台: http://{host}:{port}/console  (配置台静态页)")
    print("=" * 64)
    uvicorn.run("mox.server:app", host=host, port=port, log_level="info")


if __name__ == "__main__":
    main()
