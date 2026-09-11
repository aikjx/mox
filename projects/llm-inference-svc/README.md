# MOX LLM Inference Service

OpenAI 兼容的 LLM 推理网关服务，支持多提供商后端。

## 功能特性

- **OpenAI 兼容 API**：`/v1/chat/completions`，可直接被任何 OpenAI SDK 调用
- **多提供商支持**：OpenAI / Azure OpenAI / Ollama（本地）/ 通义千问 / 豆包 / 自定义端点
- **流式输出**：SSE 流式响应，支持实时展示
- **JSON 模式**：支持 `response_format: { type: "json_object" }`
- **Prometheus 指标**：`/metrics` 端点，QPS / 延迟 / 错误率
- **结构化日志**：JSON 格式日志，含 trace 信息
- **API Key 鉴权**：可选的服务级 API Key 验证

## 快速开始

### 1. 安装依赖

```bash
pip install -r requirements.txt
```

### 2. 配置

```bash
cp .env.example .env
# 编辑 .env，设置提供商和 API Key
```

### 3. 使用本地 Ollama（推荐开发环境）

```bash
# 安装 Ollama: https://ollama.com
ollama pull qwen2.5
ollama serve

# 在 .env 中设置:
# LLM_PROVIDER=ollama
# LLM_MODEL=qwen2.5
```

### 4. 启动服务

```bash
python -m app.main
```

服务启动在 `http://localhost:8001`

## API 文档

启动后访问 `http://localhost:8001/docs` 查看 Swagger UI。

### 主要端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/v1/models` | GET | 列出可用模型 |
| `/v1/chat/completions` | POST | 聊天补全（非流式） |
| `/v1/chat/completions/stream` | POST | 聊天补全（流式 SSE） |
| `/v1/info` | GET | 服务信息（用于服务发现） |
| `/metrics` | GET | Prometheus 指标 |

### 请求示例

```bash
curl http://localhost:8001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen2.5",
    "messages": [
      {"role": "system", "content": "你是安全专家"},
      {"role": "user", "content": "分析这个代码的安全风险"}
    ],
    "temperature": 0.3,
    "max_tokens": 1024,
    "response_format": {"type": "json_object"}
  }'
```

## Docker 部署

```bash
# 构建镜像
docker build -t mox-llm-inference-svc .

# 运行容器
docker run -d \
  --name mox-llm-svc \
  -p 8001:8001 \
  -e LLM_PROVIDER=ollama \
  -e LLM_MODEL=qwen2.5 \
  -e OLLAMA_BASE_URL=http://host.docker.internal:11434 \
  mox-llm-inference-svc
```

## 与 Rust 联盟引擎集成

在 Rust 侧，使用 `HttpLLMConsultant` 连接本服务：

```rust
use mox_ai_alliance_engine::{HttpLLMConsultant, LLMConfig, DebateEngine};

let config = LLMConfig {
    api_base: "http://localhost:8001/v1".into(),
    api_key: "your-service-api-key".into(),  // 如果配置了 SERVICE_API_KEY
    model: "qwen2.5".into(),
    timeout_secs: 60,
    ..Default::default()
};

let consultant = HttpLLMConsultant::new(config);
let engine = DebateEngine::with_consultant(consultant);
```

## 支持的提供商

| 提供商 | 环境变量前缀 | 说明 |
|--------|-------------|------|
| OpenAI | `OPENAI_` | 官方 API |
| Azure OpenAI | `AZURE_` | 微软云 |
| Ollama | `OLLAMA_` | 本地开源模型 |
| 通义千问 | `QWEN_` | 阿里云 |
| 豆包 | `DOUBAO_` | 火山引擎 |
| 自定义 | `OPENAI_` | 任何 OpenAI 兼容端点 |

## 监控

Prometheus 指标：

- `llm_requests_total{provider,model,status}` — 请求总数
- `llm_request_duration_seconds{provider,model}` — 请求延迟直方图

Grafana 仪表盘可基于这些指标构建。

## 许可证

MIT
