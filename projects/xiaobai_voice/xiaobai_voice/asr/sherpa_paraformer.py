"""Paraformer-zh INT8 用 sherpa-onnx 封装。

说明
----
1. 只做"流式识别"统一入口：full = 全部块喂完再读 final；
2. VAD 直接用 sherpa-onnx 自带 silero-vad（`enable_vad=True`），避免 DLL 地狱；
3. ImportError / FileNotFoundError / OSError 统一转换成：
   MISSING_DEP / MISSING_MODEL / DLL_LOAD_FAIL；
4. 首条冷启动 prewarm() 跑"你好，小白"的零音频预热，防止用户首句被吞。
"""
from __future__ import annotations

import asyncio
import io
import time
from dataclasses import dataclass
from typing import Any, AsyncGenerator

from .base import ASRBackend, ASRFullResult, ASRPartial
from ..errors import ErrorCode, XiaobaiError


@dataclass
class _ModelPaths:
    tokens: str
    encoder: str
    decoder: str | None = None


class SherpaParaformerBackend(ASRBackend):
    name = "sherpa_paraformer"

    def __init__(self, cfg: dict, models_registry: Any | None = None) -> None:
        super().__init__(cfg, models_registry)
        self._recognizer = None
        self._display = None
        self._paths: _ModelPaths | None = None
        self._loaded_at_ms: float = 0.0
        self._vad_threshold_ms = int(self.cfg.get("vad_threshold_ms") or 800)

        try:
            self._paths = self._resolve_model_paths()
            self._load_engine()
        except XiaobaiError:
            raise
        except ImportError as exc:  # 外部依赖/打包 venv 缺失
            raise XiaobaiError(
                code=ErrorCode.MISSING_DEP,
                message=(
                    "sherpa-onnx 未安装或外部 venv 未注入。"
                    "请: pip install sherpa-onnx，或用 build_exe.ps1 -UseVenv 指定外部环境。"
                ),
                cause=exc,
            ) from exc
        except FileNotFoundError as exc:
            raise XiaobaiError(
                code=ErrorCode.MISSING_MODEL,
                message=(
                    f"Paraformer 模型不完整（{exc.filename or ''}），"
                    f"请在桌面小白或前端下载中心下载 ASR 默认模型。"
                ),
                cause=exc,
            ) from exc
        except OSError as exc:
            # onnxruntime DLL / VC++ Redist 缺失
            raise XiaobaiError(
                code=ErrorCode.DLL_LOAD_FAIL,
                message=(
                    "加载 sherpa-onnx/onnxruntime DLL 失败。"
                    "请确认打包时已注入 onnxruntime/capi、numpy/.libs，或安装 VC++ 2022 Redist。"
                ),
                cause=exc,
            ) from exc

    # ================================================================ internal
    def _resolve_model_paths(self) -> _ModelPaths:
        """解析 3 层模型路径：<exe同级>/models > 用户目录 > 仓库 models/。"""
        registry = self.models
        model_id = "asr-paraformer-int8"
        if registry is not None and hasattr(registry, "resolve"):
            resolved = registry.resolve(model_id)
            if resolved:
                return _ModelPaths(
                    tokens=resolved["entry"]["tokens"],
                    encoder=resolved["entry"]["encoder"],
                    decoder=resolved["entry"].get("decoder") or None,
                )
        # 无 models registry 时，按约定的目录兜底，避免启动脚本无 registry 时崩。
        import os
        candidates = []
        if getattr(__import__("sys"), "frozen", False):
            exe_dir = os.path.dirname(os.path.abspath(__import__("sys").executable))
            candidates.append(os.path.join(exe_dir, "models"))
        home = os.path.expanduser("~")
        candidates.append(os.path.join(home, ".xuanji", "models", "voice"))
        candidates.append(
            os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "models"))
        )
        for root in candidates:
            sub = os.path.join(root, "asr-paraformer-int8")
            tok = os.path.join(sub, "tokens.txt")
            enc = os.path.join(sub, "model.int8.onnx")
            if os.path.isfile(tok) and os.path.isfile(enc):
                dec = os.path.join(sub, "decoder.onnx")
                return _ModelPaths(tokens=tok, encoder=enc, decoder=dec if os.path.isfile(dec) else None)
        raise FileNotFoundError("asr-paraformer-int8 模型目录未找到；请运行：xiaobai download --defaults")

    def _load_engine(self) -> None:
        assert self._paths is not None
        import sherpa_onnx  # type: ignore

        kwargs = dict(
            tokens=self._paths.tokens,
            paraformer=dict(
                encoder=self._paths.encoder,
                decoder=self._paths.decoder or "",
            ),
            num_threads=int(self.cfg.get("num_threads") or 4),
            provider=self.cfg.get("provider") or "cpu",
            enable_vad=True,
        )
        try:
            self._recognizer = sherpa_onnx.OnlineRecognizer.from_paraformer(**kwargs)
        except TypeError:
            # 老版本 sherpa-onnx 可能不用 from_paraformer
            cfg = sherpa_onnx.OnlineRecognizerConfig(
                tokens=kwargs["tokens"],
                num_threads=kwargs["num_threads"],
                provider=kwargs["provider"],
                feat_config=sherpa_onnx.FeatureConfig(sample_rate=self.sample_rate),
                model_config=sherpa_onnx.ModelConfig(
                    paraformer=sherpa_onnx.OfflineModelConfig(
                        **{
                            "encoder": kwargs["paraformer"]["encoder"],
                            "decoder": kwargs["paraformer"]["decoder"],
                        }
                    )
                ),
                enable_endpoint_detection=True,
            )
            self._recognizer = sherpa_onnx.OnlineRecognizer(cfg)
        self._loaded_at_ms = time.time() * 1000

    # ================================================================ lifecycle
    def prewarm(self) -> float:
        start = time.perf_counter()
        # 喂 120 ms 静音（16000 Hz × 16 bit = 32000 bytes/s → 3840 bytes）
        import numpy as np

        silent = np.zeros(int(self.sample_rate * 0.12), dtype=np.float32)
        stream = self._recognizer.create_stream()
        stream.accept_waveform(self.sample_rate, silent)
        while self._recognizer.is_ready(stream):
            self._recognizer.decode_stream(stream)
        _ = self._recognizer.get_result(stream)
        self._recognizer.reset(stream)
        ms = (time.perf_counter() - start) * 1000
        return ms

    def close(self) -> None:
        try:
            if self._recognizer is not None and hasattr(self._recognizer, "__del__"):
                self._recognizer = None
        except Exception:  # noqa: BLE001
            pass

    # ============================================================== streaming
    async def recognize_stream(
        self,
        chunks: AsyncGenerator[bytes, None],
        sample_rate: int | None = None,
    ) -> AsyncGenerator[ASRPartial, None]:
        sr = int(sample_rate or self.sample_rate)
        if self._recognizer is None:
            raise RuntimeError("Sherpa recognizer 未初始化。")
        stream = self._recognizer.create_stream()
        import numpy as np

        last_text = ""
        final_promise: asyncio.Future[ASRPartial | None] = asyncio.Future()

        def _feed_pcm(pcm16: bytes) -> str:
            nonlocal last_text
            arr = np.frombuffer(pcm16, dtype=np.int16).astype(np.float32) / 32768.0
            stream.accept_waveform(sr, arr)
            while self._recognizer.is_ready(stream):
                self._recognizer.decode_stream(stream)
            result = self._recognizer.get_result(stream)
            if self._recognizer.is_endpoint(stream):
                self._recognizer.reset(stream)
            return result

        async for block in chunks:
            if not block:
                continue
            text = await asyncio.to_thread(_feed_pcm, block)
            if text != last_text:
                last_text = text
                yield ASRPartial(text=text, is_final=False, confidence=0.9)

        # flush：尾部强行触发 endpoint
        stream.input_finished()
        while self._recognizer.is_ready(stream):
            await asyncio.to_thread(self._recognizer.decode_stream, stream)
        final_text = self._recognizer.get_result(stream) or last_text
        self._recognizer.reset(stream)
        if final_text:
            yield ASRPartial(text=final_text, is_final=True, confidence=0.95)

    # ---------------------------------------------------------------- full
    async def recognize_full(
        self,
        audio_bytes: bytes,
        sample_rate: int | None = None,
        fmt: str = "wav",
    ) -> ASRFullResult:
        sr = int(sample_rate or self.sample_rate)
        import numpy as np
        import soundfile as sf  # wav/webm/flac 全支持

        if fmt.lower() in {"wav", "webm", "flac", "ogg", "m4a", "mp3"}:
            bio = io.BytesIO(audio_bytes)
            try:
                data, file_sr = sf.read(bio, dtype="float32", always_2d=False)
            except Exception as exc:  # sf.LibsndfileError 封装缺失
                raise XiaobaiError(
                    code=ErrorCode.DLL_LOAD_FAIL,
                    message="soundfile/libsndfile 无法解析音频文件。",
                    cause=exc,
                ) from exc
            if data.ndim > 1:
                data = data.mean(axis=1)
            if int(file_sr) != sr:
                # 简单重采样：线性（比 resampy 省依赖）
                import math

                ratio = sr / float(file_sr)
                new_len = int(math.ceil(len(data) * ratio))
                idx = (np.arange(new_len) / ratio).astype(np.int64).clip(0, len(data) - 1)
                data = data[idx]
        else:  # raw int16 PCM
            data = np.frombuffer(audio_bytes, dtype=np.int16).astype(np.float32) / 32768.0

        if self._recognizer is None:
            raise RuntimeError("Sherpa recognizer 未初始化。")

        stream = self._recognizer.create_stream()
        # 以 960 样本为一块（60 ms @ 16k）
        chunk_samples = max(960, sr // 100)
        offset = 0
        while offset < len(data):
            seg = data[offset : offset + chunk_samples]
            stream.accept_waveform(sr, seg.astype(np.float32, copy=False))
            while self._recognizer.is_ready(stream):
                await asyncio.to_thread(self._recognizer.decode_stream, stream)
            offset += chunk_samples
        stream.input_finished()
        while self._recognizer.is_ready(stream):
            await asyncio.to_thread(self._recognizer.decode_stream, stream)
        text = self._recognizer.get_result(stream)
        self._recognizer.reset(stream)
        duration_ms = int(len(data) / sr * 1000) if sr else 0
        return ASRFullResult(text=text or "", duration_ms=duration_ms, confidence=0.95)

    # ================================================================ hotwords
    def set_hotwords(self, words):  # 类型见基类
        """sherpa-onnx paraformer 对热词通过上下文/解码侧支持有限；

        这里保存列表给后续 SenseVoice/CustomDecoder，防止空数据。
        """
        self.cfg["hotwords"] = list(words or [])
