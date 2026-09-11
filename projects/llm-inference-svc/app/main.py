"""
LLM 推理服务主应用
OpenAI 兼容 API，支持多提供商后端
"""
import time
import uuid
from typing import Optional

import structlog
import uvicorn
from fastapi import Depends, FastAPI, Header, HTTPException, Request
from fastapi.responses import StreamingResponse
from prometheus_client import Counter, Histogram, make_asgi_app

from .config import settings
from .models import (
    ChatCompletionRequest,
    ChatCompletionResponse,
    HealthResponse,
    ModelInfo,
    ModelListResponse,
)
from .providers import get_available_models, get_provider

# 结构化日志
structlog.configure(
    processors=[
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.add_log_level,
        structlog.processors.JSONRenderer(),
    ]
)
logger = structlog.get_logger()

# Prometheus 指标
REQUEST_COUNT = Counter(
    "llm_requests_total",
    "Total LLM requests",
    ["provider", "model", "status"],
)
REQUEST_LATENCY = Histogram(
    "llm_request_duration_seconds",
    "LLM request latency",
    ["provider", "model"],
)

app = FastAPI(
    title="MOX LLM Inference Service",
    description="OpenAI-compatible LLM inference service with multi-provider support",
    version="1.0.0",
)

# Prometheus 指标端点
metrics_app = make_asgi_app()
app.mount("/metrics", metrics_app)


# ── 依赖注入 ──────────────────────────────────────────────


async def verify_api_key(
    x_api_key: Optional[str] = Header(default=None),
    authorization: Optional[str] = Header(default=None),
) -> bool:
    """验证服务 API Key（如果配置了的话）"""
    if not settings.service_api_key:
        return True  # 未配置则不鉴权

    # 支持 X-API-Key 头或 Authorization: Bearer 头
    if x_api_key == settings.service_api_key:
        return True
    if authorization and authorization.startswith("Bearer "):
        token = authorization[7:]
        if token == settings.service_api_key:
            return True

    raise HTTPException(status_code=401, detail="Invalid API key")


# ── 健康检查 ──────────────────────────────────────────────


@app.get("/health", response_model=HealthResponse)
async def health():
    """健康检查端点"""
    return HealthResponse(
        service=settings.service_name,
        provider=settings.default_provider,
        model=settings.default_model,
    )


@app.get("/v1/models", response_model=ModelListResponse)
async def list_models(_: bool = Depends(verify_api_key)):
    """列出可用模型（OpenAI 兼容）"""
    models = [
        ModelInfo(id=m, created=int(time.time()))
        for m in get_available_models()
    ]
    return ModelListResponse(data=models)


# ── Chat Completion ───────────────────────────────────────


@app.post("/v1/chat/completions", response_model=ChatCompletionResponse)
async def create_chat_completion(
    request: ChatCompletionRequest,
    _: bool = Depends(verify_api_key),
):
    """创建聊天补全（非流式）"""
    provider = get_provider()
    t0 = time.time()

    try:
        # 如果请求指定了模型但与默认不同，仍然使用同一提供商
        result = await provider.chat_completion(request)

        latency = time.time() - t0
        REQUEST_COUNT.labels(provider.name, request.model, "success").inc()
        REQUEST_LATENCY.labels(provider.name, request.model).observe(latency)

        logger.info(
            "chat.completion",
            provider=provider.name,
            model=request.model,
            latency_ms=int(latency * 1000),
            tokens=result.usage.total_tokens,
        )
        return result

    except Exception as e:
        latency = time.time() - t0
        REQUEST_COUNT.labels(provider.name, request.model, "error").inc()
        logger.error("chat.completion.error", error=str(e), latency_ms=int(latency * 1000))
        raise HTTPException(status_code=502, detail=f"LLM upstream error: {str(e)}")


@app.post("/v1/chat/completions/stream")
async def create_chat_completion_stream(
    request: ChatCompletionRequest,
    _: bool = Depends(verify_api_key),
):
    """创建聊天补全（流式 SSE）"""
    provider = get_provider()
    request.stream = True

    logger.info("chat.completion.stream", provider=provider.name, model=request.model)

    try:
        return StreamingResponse(
            provider.chat_completion_stream(request),
            media_type="text/event-stream",
            headers={
                "Cache-Control": "no-cache",
                "Connection": "keep-alive",
                "X-Accel-Buffering": "no",
            },
        )
    except Exception as e:
        logger.error("chat.completion.stream.error", error=str(e))
        raise HTTPException(status_code=502, detail=f"LLM stream error: {str(e)}")


# ── 嵌入式模型信息（用于 Rust 侧发现） ────────────────────


@app.get("/v1/info")
async def service_info():
    """服务信息端点（Rust 侧用于服务发现）"""
    return {
        "service": settings.service_name,
        "version": "1.0.0",
        "provider": settings.default_provider,
        "default_model": settings.default_model,
        "capabilities": {
            "chat_completion": True,
            "streaming": True,
            "json_mode": True,
            "function_calling": False,
        },
        "endpoints": {
            "chat": "/v1/chat/completions",
            "chat_stream": "/v1/chat/completions/stream",
            "models": "/v1/models",
            "health": "/health",
            "metrics": "/metrics",
        },
    }


# ── 启动 ──────────────────────────────────────────────────


def main():
    """启动服务"""
    logger.info(
        "service.starting",
        service=settings.service_name,
        host=settings.host,
        port=settings.port,
        provider=settings.default_provider,
        model=settings.default_model,
    )
    uvicorn.run(
        "app.main:app",
        host=settings.host,
        port=settings.port,
        log_level=settings.log_level.lower(),
        workers=1,
    )


if __name__ == "__main__":
    main()
