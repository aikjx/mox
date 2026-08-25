"""统一分级错误。ImportError → MISSING_DEP / FileNotFound → MISSING_MODEL / OSError DLL → DLL_LOAD_FAIL / GPU_OOM。"""
from __future__ import annotations

import enum
from typing import Any


class ErrorCode(str, enum.Enum):
    OK = "OK"
    MISSING_DEP = "MISSING_DEP"
    MISSING_MODEL = "MISSING_MODEL"
    DLL_LOAD_FAIL = "DLL_LOAD_FAIL"
    GPU_OOM = "GPU_OOM"
    LICENSE_GATE = "LICENSE_GATE"
    CONFIG_INVALID = "CONFIG_INVALID"
    RUNTIME = "RUNTIME"
    UNKNOWN = "UNKNOWN"


class XiaobaiError(Exception):
    def __init__(
        self,
        code: str | ErrorCode = ErrorCode.UNKNOWN,
        message: str = "",
        cause: BaseException | None = None,
        details: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(message)
        self.code = code if isinstance(code, ErrorCode) else ErrorCode(str(code))
        self.message = message
        self.cause = cause
        self.details = dict(details or {})

    def to_dict(self) -> dict:
        return {
            "code": self.code.value,
            "message": self.message or str(self),
            "cause": None if self.cause is None else f"{type(self.cause).__name__}: {self.cause}",
            "details": self.details,
        }

    def __repr__(self) -> str:  # pragma: no cover
        return f"XiaobaiError({self.code.value}, {self.message!r})"
