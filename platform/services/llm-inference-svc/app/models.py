"""
OpenAI 兼容 API 数据模型
参考: https://platform.openai.com/docs/api-reference/chat
"""
from pydantic import BaseModel, Field
from typing import Optional, List, Literal, Dict, Any


class ChatMessage(BaseModel):
    """聊天消息"""
    role: Literal["system", "user", "assistant", "tool"]
    content: str
    name: Optional[str] = None


class ResponseFormat(BaseModel):
    """响应格式"""
    type: Literal["text", "json_object"] = "text"


class ChatCompletionRequest(BaseModel):
    """Chat Completion 请求"""
    model: str
    messages: List[ChatMessage]
    temperature: Optional[float] = Field(default=1.0, ge=0, le=2)
    top_p: Optional[float] = Field(default=1.0, ge=0, le=1)
    n: Optional[int] = Field(default=1, ge=1, le=10)
    stream: Optional[bool] = False
    stop: Optional[List[str]] = None
    max_tokens: Optional[int] = Field(default=None, ge=1)
    presence_penalty: Optional[float] = Field(default=0, ge=-2, le=2)
    frequency_penalty: Optional[float] = Field(default=0, ge=-2, le=2)
    response_format: Optional[ResponseFormat] = None
    user: Optional[str] = None


class Usage(BaseModel):
    """Token 使用量"""
    prompt_tokens: int = 0
    completion_tokens: int = 0
    total_tokens: int = 0


class ChatChoice(BaseModel):
    """聊天选择"""
    index: int = 0
    message: ChatMessage
    finish_reason: Optional[str] = "stop"


class ChatCompletionResponse(BaseModel):
    """Chat Completion 响应"""
    id: str
    object: str = "chat.completion"
    created: int
    model: str
    choices: List[ChatChoice]
    usage: Usage


class StreamDelta(BaseModel):
    """流式增量"""
    role: Optional[str] = None
    content: Optional[str] = None


class StreamChoice(BaseModel):
    """流式选择"""
    index: int = 0
    delta: StreamDelta
    finish_reason: Optional[str] = None


class ChatCompletionChunk(BaseModel):
    """流式响应块"""
    id: str
    object: str = "chat.completion.chunk"
    created: int
    model: str
    choices: List[StreamChoice]


class ModelInfo(BaseModel):
    """模型信息"""
    id: str
    object: str = "model"
    created: int = 0
    owned_by: str = "mox"


class ModelListResponse(BaseModel):
    """模型列表响应"""
    object: str = "list"
    data: List[ModelInfo]


class HealthResponse(BaseModel):
    """健康检查响应"""
    status: str = "ok"
    service: str
    version: str = "1.0.0"
    provider: str
    model: str
