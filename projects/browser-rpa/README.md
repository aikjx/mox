# browser-rpa — 浏览器 RPA 单容器服务

基于 Playwright + Python 的浏览器 RPA 服务，单容器交付。核心能力：

1. **录制**：`playwright codegen` 录制浏览器操作，直接产出 **Python 脚本**并按任务版本化存档。
2. **AI 改脚本（自愈）**：页面变动导致脚本报错（定位器失效/超时）时，自动抓取失败现场页面快照，调用 LLM 重写脚本并复跑；LLM 不可用时退化为离线模糊定位器修复。
3. **智能运维**：内置调度器（间隔任务）、失败自动重跑/自愈闭环、运行留痕（runs/events）、健康聚合端点。
4. **需求归一化**：自然语言需求经 `intake` 归一化为结构化 `TaskSpec`（schema `RPA-TASKSPEC-V1`），字段别名、URL 抽取、意图词表统一映射。

## 目录结构

```
projects/browser-rpa/
├── run.py              # 启动入口（uvicorn，端口 30400，RPA_PORT 可覆盖）
├── rpa/
│   ├── api.py          # FastAPI HTTP 层
│   ├── config.py       # 路径/端口/LLM 配置
│   ├── store.py        # 文件存储：任务/脚本版本/运行记录/事件
│   ├── recorder.py     # codegen 录制封装（start/stop → Python 脚本）
│   ├── runner.py       # 脚本执行器（子进程 + 超时 + 留痕）
│   ├── healer.py       # AI 自愈：快照采集 → LLM 重写 → 离线定位器修复回退
│   ├── intake.py       # 需求归一化 → TaskSpec
│   ├── ops.py          # 智能运维：调度器、健康聚合、事件总线
│   └── llm.py          # OpenAI 兼容客户端（未配置时离线回退）
├── tests/              # 离线单元测试（不依赖网络/浏览器）
├── Dockerfile          # 单容器（Playwright 官方基础镜像 + Xvfb 有头录制）
└── docker-compose.yml
```

## HTTP API（默认 http://localhost:30400）

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/health` | 健康 + 运行统计聚合 |
| POST | `/intake` | `{requirement_text}` → 归一化 TaskSpec |
| GET/POST | `/tasks` | 任务列表 / 新建（POST 传归一后 spec 或原文） |
| GET | `/tasks/{id}` | 任务详情（含脚本版本列表） |
| POST | `/tasks/{id}/record/start` | 启动 codegen 录制 |
| POST | `/tasks/{id}/record/stop` | 停止录制，Python 脚本存为新版 |
| PUT | `/tasks/{id}/script` | 手工上传脚本 `{code}` |
| GET | `/tasks/{id}/script?version=N` | 取脚本内容 |
| POST | `/tasks/{id}/run` | 执行（`heal=true` 时走自愈闭环） |
| GET | `/runs` `/runs/{id}` | 运行记录 |
| POST | `/tasks/{id}/heal` | 手工触发自愈 |
| GET | `/events` | 最近运维事件 |

## 本地运行

```bash
cd projects/browser-rpa
pip install -r requirements.txt && playwright install chromium
python run.py                 # 30400
python -m unittest discover tests -v   # 离线自检
```

## 容器运行（一个容器）

```bash
docker compose up -d --build
curl http://localhost:30400/health
```

## 环境变量

`RPA_PORT`（默认 30400）、`RPA_DATA_DIR`、`RPA_LLM_BASE_URL` / `RPA_LLM_API_KEY` / `RPA_LLM_MODEL`（OpenAI 兼容；不配置则 AI 走离线规则回退）、`RPA_HEADLESS`、`RPA_HEAL_MAX_ATTEMPTS`。
