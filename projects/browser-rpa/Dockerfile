# 单容器交付：Playwright 官方 Python 镜像（含浏览器）+ Xvfb（codegen 有头录制）
FROM mcr.microsoft.com/playwright/python:v1.48.0-noble

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends xvfb \
    && rm -rf /var/lib/apt/lists/*

# 基础镜像内置 chromium（/ms-playwright），playwright pip 包须自装且版本与镜像 tag 对齐
COPY requirements.txt ./
RUN pip install --no-cache-dir fastapi "uvicorn>=0.23" httpx "playwright==1.48.0"

COPY rpa ./rpa
COPY web ./web
COPY run.py entrypoint.sh README.md ./
RUN chmod +x entrypoint.sh

ENV RPA_HOST=0.0.0.0 \
    RPA_PORT=30400 \
    RPA_HEADLESS=1
EXPOSE 30400

ENTRYPOINT ["./entrypoint.sh"]
