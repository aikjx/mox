"""CosyVoice2 封装（Apache2，信创回退默认）。

T3：实现最小可行封装：未安装或权重缺失时，转为 XiaobaiError；已安装时用指令模式 + 流式 chunk。
"""
from __future__ import annotations

import os
from collections.abc import Generator
from typing import Any

from .base import TTSBackend, TTSOptions
from ..errors import ErrorCode, XiaobaiError


# 情绪 → CosyVoice 指令前缀（指令微调 0.5B 模型常见用法）
_EMOTION_PROMPT = {
    "neutral": "请用温暖、自然、中性的中文语气朗读：",
    "happy":   "请用愉悦、欢快的中文语气朗读：",
    "sad":     "请用低沉、略带哀伤的中文语气朗读：",
    "serious": "请用严肃、稳重、专业的中文语气朗读：",
}


class CosyVoice2Backend(TTSBackend):
    name = "cosyvoice2"

    def __init__(self, cfg: dict, models_registry: Any | None = None) -> None:
        super().__init__(cfg, models_registry)
        self._model = None
        self._ckpt_dir = self._resolve_model_dir(models_registry)
        try:
            self._load_engine()
        except XiaobaiError:
            raise
        except ImportError as exc:
            raise XiaobaiError(
                code=ErrorCode.MISSING_DEP,
                message=(
                    "CosyVoice2 未安装。请执行：pip install cosyvoice>=0.2.0 （Apache2）。"
                    "或把 license_tier=apache2 切回 auto，允许浏览器 TTS 兜底。"
                ),
                cause=exc,
            ) from exc
        except FileNotFoundError as exc:
            raise XiaobaiError(
                code=ErrorCode.MISSING_MODEL,
                message=f"CosyVoice2 权重目录缺失：{exc.filename or self._ckpt_dir}。请下载 tts-cosyvoice2-0.5b。",
                cause=exc,
            ) from exc
        except OSError as exc:
            raise XiaobaiError(
                code=ErrorCode.DLL_LOAD_FAIL,
                message="CosyVoice2 加载 DLL/torch/onnxruntime 失败。",
                cause=exc,
            ) from exc

    def _resolve_model_dir(self, registry: Any | None) -> str:
        if registry is not None and hasattr(registry, "resolve"):
            r = registry.resolve("tts-cosyvoice2-0.5b")
            if r:
                return r["root"]
        candidates = []
        import sys

        if getattr(sys, "frozen", False):
            candidates.append(os.path.join(os.path.dirname(sys.executable), "models", "tts-cosyvoice2-0.5b"))
        candidates.append(os.path.join(os.path.expanduser("~"), ".xuanji", "models", "voice", "tts-cosyvoice2-0.5b"))
        candidates.append(
            os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "models", "tts-cosyvoice2-0.5b"))
        )
        for c in candidates:
            # CosyVoice2 根目录至少需含 configuration.json
            if os.path.isfile(os.path.join(c, "configuration.json")) or os.path.isdir(c):
                return c
        raise FileNotFoundError(candidates[-1])

    def _load_engine(self) -> None:
        # 延迟 import：仅在构造时调用。apache2 模式下 Fish 不会被 import。
        import cosyvoice  # type: ignore  # noqa: F401

        # 优先 torch，其次 onnxruntime（取决于 cosyvoice 版本）
        try:
            self._model = cosyvoice.CosyVoice2(self._ckpt_dir)
        except Exception:
            # 老版本 CosyVoice 类名
            self._model = cosyvoice.CosyVoice(self._ckpt_dir)  # type: ignore[attr-defined]

    # -------------------------------------------------------------- synthesize
    def synthesize(self, opts: TTSOptions) -> Generator[bytes, None, None]:
        import numpy as np

        sr = opts.sample_rate or self.sample_rate or 22050
        if opts.emotion not in _EMOTION_PROMPT:
            opts.emotion = "neutral"
        instruction = _EMOTION_PROMPT[opts.emotion] + (opts.text or "")
        try:
            synth_iter = self._model.inference_sft(instruction, "中文女")  # type: ignore[union-attr]
        except Exception as exc:  # noqa: BLE001
            raise XiaobaiError(
                code=ErrorCode.RUNTIME,
                message=f"CosyVoice2 合成失败：{exc}",
                cause=exc,
            ) from exc

        # CosyVoice 常见返回是 { "tts_speech": (sr, ndarray) } 或 generator 吐 chunk
        audio_chunks: list[np.ndarray] = []
        sample_rate_out = sr
        try:
            for item in synth_iter:
                if isinstance(item, tuple) and len(item) == 2:
                    sample_rate_out, arr = item
                elif isinstance(item, dict) and "tts_speech" in item:
                    sample_rate_out, arr = item["tts_speech"]
                else:
                    continue
                audio_chunks.append(np.asarray(arr, dtype=np.float32))
        except Exception as exc:  # noqa: BLE001
            raise XiaobaiError(ErrorCode.RUNTIME, f"CosyVoice2 合成中断: {exc}", cause=exc) from exc

        if not audio_chunks:
            raise XiaobaiError(ErrorCode.RUNTIME, "CosyVoice2 合成结果为空。可能是空文本或权重不完整。")

        audio = np.concatenate(audio_chunks, axis=0)
        # 重采样到目标 sr（简单线性）
        if int(sample_rate_out) != int(sr):
            ratio = sr / float(sample_rate_out)
            idx = (np.arange(int(len(audio) * ratio)) / ratio).astype(np.int64).clip(0, len(audio) - 1)
            audio = audio[idx]

        # 归一化 float → int16
        peak = float(np.max(np.abs(audio))) if audio.size else 0.0
        if peak > 0:
            audio = audio / peak * 0.9
        int16 = (audio * 32767.0).clip(-32768, 32767).astype("<i2")
        raw = int16.tobytes()
        # WAV 头 + 整块数据；为了"流式"体验，我们先以 512B 字节块发出
        from .browser_fallback import _make_wav_header
        yield _make_wav_header(sr=sr, channels=1, bits=16, data_len=len(raw))
        chunk_bytes = max(1024, int(sr * 2 * (opts.stream_chunk_ms / 1000.0)))
        for i in range(0, len(raw), chunk_bytes):
            yield raw[i : i + chunk_bytes]

    def close(self) -> None:
        if self._model is not None:
            self._model = None
