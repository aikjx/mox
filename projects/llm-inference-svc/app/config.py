"""
LLM 推理服务配置
通过环境变量或 .env 文件配置
"""
from pydantic_settings import BaseSettings, SettingsConfigDict
from pydantic import Field
from typing import Optional


class Settings(BaseSettings):
    """服务配置"""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # 服务配置
    service_name: str = Field(default="mox-llm-inference-svc", alias="SERVICE_NAME")
    host: str = Field(default="0.0.0.0", alias="HOST")
    port: int = Field(default=8001, alias="PORT")
    log_level: str = Field(default="INFO", alias="LOG_LEVEL")

    # 默认 LLM 提供商配置
    # 支持: openai / azure / ollama / qwen / doubao / custom
    default_provider: str = Field(default="openai", alias="LLM_PROVIDER")
    default_model: str = Field(default="gpt-4o-mini", alias="LLM_MODEL")

    # OpenAI 兼容配置
    openai_api_base: str = Field(default="https://api.openai.com/v1", alias="OPENAI_API_BASE")
    openai_api_key: Optional[str] = Field(default=None, alias="OPENAI_API_KEY")

    # Azure OpenAI 配置
    azure_endpoint: Optional[str] = Field(default=None, alias="AZURE_ENDPOINT")
    azure_api_key: Optional[str] = Field(default=None, alias="AZURE_API_KEY")
    azure_api_version: str = Field(default="2024-02-15-preview", alias="AZURE_API_VERSION")

    # Ollama 本地模型配置
    ollama_base_url: str = Field(default="http://localhost:11434", alias="OLLAMA_BASE_URL")

    # 通义千问配置
    qwen_api_key: Optional[str] = Field(default=None, alias="QWEN_API_KEY")
    qwen_base_url: str = Field(default="https://dashscope.aliyuncs.com/compatible-mode/v1", alias="QWEN_BASE_URL")

    # 豆包配置
    doubao_api_key: Optional[str] = Field(default=None, alias="DOUBAO_API_KEY")
    doubao_base_url: str = Field(default="https://ark.cn-beijing.volces.com/api/v3", alias="DOUBAO_BASE_URL")

    # 限流配置
    max_requests_per_minute: int = Field(default=60, alias="MAX_RPM")
    max_concurrent_requests: int = Field(default=10, alias="MAX_CONCURRENT")

    # 超时配置
    request_timeout_seconds: int = Field(default=60, alias="REQUEST_TIMEOUT")

    # 认证配置（服务自身的 API Key，用于 Rust 侧调用时鉴权）
    service_api_key: Optional[str] = Field(default=None, alias="SERVICE_API_KEY")


settings = Settings()
