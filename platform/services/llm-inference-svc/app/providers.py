"""
LLM 提供商抽象层
支持多种后端：OpenAI / Azure / Ollama / Qwen / Doubao / 自定义
统一为 OpenAI 兼容格式
"""
import abc
import json
import time
import uuid
from typing import AsyncGenerator, Optional

import httpx
import structlog

from .config import settings
from .models import (
    ChatCompletionRequest,
    ChatCompletionResponse,
    ChatChoice,
    ChatMessage,
    Usage,
)

logger = structlog.get_logger()


class LLMProvider(abc.ABC):
    """LLM 提供商基类"""

    @abc.abstractmethod
    async def chat_completion(
        self, request: ChatCompletionRequest
    ) -> ChatCompletionResponse:
        """非流式聊天补全"""
        ...

    @abc.abstractmethod
    async def chat_completion_stream(
        self, request: ChatCompletionRequest
    ) -> AsyncGenerator[str, None]:
        """流式聊天补全（SSE 格式）"""
        ...

    @property
    @abc.abstractmethod
    def name(self) -> str:
        """提供商名称"""
        ...


class OpenAICompatibleProvider(LLMProvider):
    """OpenAI 兼容提供商（适用于 OpenAI / Qwen / Doubao / 自定义端点）"""

    def __init__(self, api_base: str, api_key: Optional[str], provider_name: str = "openai"):
        self.api_base = api_base.rstrip("/")
        self.api_key = api_key
        self._provider_name = provider_name
        self._client = httpx.AsyncClient(
            timeout=httpx.Timeout(settings.request_timeout_seconds),
            limits=httpx.Limits(max_connections=settings.max_concurrent_requests),
        )

    @property
    def name(self) -> str:
        return self._provider_name

    def _headers(self) -> dict:
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"
        return headers

    async def chat_completion(
        self, request: ChatCompletionRequest
    ) -> ChatCompletionResponse:
        url = f"{self.api_base}/chat/completions"
        payload = request.model_dump(exclude_none=True)

        log = logger.bind(provider=self.name, model=request.model)
        log.info("llm.request", messages=len(request.messages))

        t0 = time.time()
        resp = await self._client.post(url, json=payload, headers=self._headers())
        latency_ms = int((time.time() - t0) * 1000)

        if resp.status_code != 200:
            log.error("llm.error", status=resp.status_code, body=resp.text[:500])
            raise RuntimeError(f"LLM API error {resp.status_code}: {resp.text[:200]}")

        data = resp.json()
        log.info("llm.response", latency_ms=latency_ms, tokens=data.get("usage", {}))

        return ChatCompletionResponse(**data)

    async def chat_completion_stream(
        self, request: ChatCompletionRequest
    ) -> AsyncGenerator[str, None]:
        url = f"{self.api_base}/chat/completions"
        payload = request.model_dump(exclude_none=True)
        payload["stream"] = True

        log = logger.bind(provider=self.name, model=request.model, stream=True)
        log.info("llm.request.stream")

        async with self._client.stream(
            "POST", url, json=payload, headers=self._headers()
        ) as resp:
            if resp.status_code != 200:
                body = await resp.aread()
                log.error("llm.stream.error", status=resp.status_code, body=body[:500])
                raise RuntimeError(f"LLM stream error {resp.status_code}")

            async for line in resp.aiter_lines():
                if line.startswith("data: "):
                    data = line[6:]
                    if data == "[DONE]":
                        yield "data: [DONE]\n\n"
                        break
                    yield f"data: {data}\n\n"


class OllamaProvider(LLMProvider):
    """Ollama 本地模型提供商"""

    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip("/")
        self._client = httpx.AsyncClient(
            timeout=httpx.Timeout(settings.request_timeout_seconds * 2),
        )

    @property
    def name(self) -> str:
        return "ollama"

    async def chat_completion(
        self, request: ChatCompletionRequest
    ) -> ChatCompletionResponse:
        url = f"{self.base_url}/api/chat"
        payload = {
            "model": request.model,
            "messages": [m.model_dump() for m in request.messages],
            "stream": False,
            "options": {
                "temperature": request.temperature or 0.7,
                "num_predict": request.max_tokens or 1024,
            },
        }

        log = logger.bind(provider="ollama", model=request.model)
        log.info("ollama.request")

        t0 = time.time()
        resp = await self._client.post(url, json=payload)
        latency_ms = int((time.time() - t0) * 1000)

        if resp.status_code != 200:
            log.error("ollama.error", status=resp.status_code)
            raise RuntimeError(f"Ollama error {resp.status_code}")

        data = resp.json()
        content = data.get("message", {}).get("content", "")

        log.info("ollama.response", latency_ms=latency_ms)

        resp_id = f"chatcmpl-{uuid.uuid4().hex[:24]}"
        return ChatCompletionResponse(
            id=resp_id,
            created=int(time.time()),
            model=request.model,
            choices=[
                ChatChoice(
                    index=0,
                    message=ChatMessage(role="assistant", content=content),
                    finish_reason="stop",
                )
            ],
            usage=Usage(
                prompt_tokens=data.get("prompt_eval_count", 0),
                completion_tokens=data.get("eval_count", 0),
                total_tokens=data.get("prompt_eval_count", 0) + data.get("eval_count", 0),
            ),
        )

    async def chat_completion_stream(
        self, request: ChatCompletionRequest
    ) -> AsyncGenerator[str, None]:
        # Ollama 流式需要转换为 OpenAI SSE 格式
        url = f"{self.base_url}/api/chat"
        payload = {
            "model": request.model,
            "messages": [m.model_dump() for m in request.messages],
            "stream": True,
        }

        resp_id = f"chatcmpl-{uuid.uuid4().hex[:24]}"
        created = int(time.time())

        async with self._client.stream("POST", url, json=payload) as resp:
            async for line in resp.aiter_lines():
                if not line.strip():
                    continue
                try:
                    data = json.loads(line)
                except json.JSONDecodeError:
                    continue

                content = data.get("message", {}).get("content", "")
                if content:
                    chunk = {
                        "id": resp_id,
                        "object": "chat.completion.chunk",
                        "created": created,
                        "model": request.model,
                        "choices": [
                            {
                                "index": 0,
                                "delta": {"role": "assistant", "content": content},
                                "finish_reason": None,
                            }
                        ],
                    }
                    yield f"data: {json.dumps(chunk)}\n\n"

                if data.get("done"):
                    yield "data: [DONE]\n\n"
                    break


def get_provider(provider_name: Optional[str] = None) -> LLMProvider:
    """工厂函数：根据配置获取 LLM 提供商"""
    name = provider_name or settings.default_provider

    if name == "ollama":
        return OllamaProvider(settings.ollama_base_url)

    if name == "azure" and settings.azure_endpoint and settings.azure_api_key:
        # Azure 使用 OpenAI 兼容格式，但 URL 不同
        api_base = (
            f"{settings.azure_endpoint.rstrip('/')}/openai/deployments"
            f"/{settings.default_model}"
        )
        # 注意：Azure 需要 api-version 查询参数，这里简化处理
        return OpenAICompatibleProvider(api_base, settings.azure_api_key, "azure")

    if name == "qwen" and settings.qwen_api_key:
        return OpenAICompatibleProvider(settings.qwen_base_url, settings.qwen_api_key, "qwen")

    if name == "doubao" and settings.doubao_api_key:
        return OpenAICompatibleProvider(
            settings.doubao_base_url, settings.doubao_api_key, "doubao"
        )

    # 默认使用 OpenAI 兼容
    return OpenAICompatibleProvider(
        settings.openai_api_base, settings.openai_api_key, "openai"
    )


def get_available_models() -> list[str]:
    """获取可用模型列表"""
    models = [settings.default_model]
    # 可以根据提供商扩展更多模型
    if settings.default_provider == "ollama":
        models.extend(["llama3.1", "qwen2.5", "deepseek-r1"])
    elif settings.default_provider == "openai":
        models.extend(["gpt-4o", "gpt-4o-mini", "gpt-3.5-turbo"])
    elif settings.default_provider == "qwen":
        models.extend(["qwen-max", "qwen-plus", "qwen-turbo"])
    elif settings.default_provider == "doubao":
        models.extend(["doubao-pro-32k", "doubao-lite-32k"])
    return list(dict.fromkeys(models))  # 去重保序
