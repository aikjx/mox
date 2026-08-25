"""TTS backends: Fish-S2-Pro（Research 延迟 import）/ CosyVoice2（Apache2）/ BrowserFallback。"""
from __future__ import annotations

from .base import TTSBackend, TTSOptions  # noqa: F401


def build_tts_backend(
    config: dict,
    license_tier: str = "auto",
    models_registry: object | None = None,
) -> TTSBackend:
    """按 license_tier + 配置 voice.tts.engine 选择 TTS 后端。

    - license_tier=apache2 → 禁止 Fish，强制 CosyVoice2（ImportError 时 BrowserFallback）
    - license_tier=research → 优先 Fish，权重缺失或未安装 → CosyVoice2 → Browser
    - license_tier=auto → 检测 Fish 权重完整才用，否则 CosyVoice2 → Browser
    """
    from .browser_fallback import BrowserFallbackBackend
    from .cosyvoice2 import CosyVoice2Backend
    from ..errors import ErrorCode, XiaobaiError

    voice = (config or {}).get("voice", {})
    tts_cfg = dict(voice.get("tts", {}) or {})
    engine = (tts_cfg.get("engine") or "auto").strip().lower()

    licence_ok_fish = license_tier in {"auto", "research"}
    candidates: list[str] = []
    if engine == "fish_s2" and licence_ok_fish:
        candidates = ["fish_s2", "cosyvoice2", "browser"]
    elif engine == "cosyvoice2":
        candidates = ["cosyvoice2", "browser"]
    elif engine == "browser":
        candidates = ["browser"]
    else:  # auto
        if licence_ok_fish:
            candidates = ["fish_s2", "cosyvoice2", "browser"]
        else:
            candidates = ["cosyvoice2", "browser"]

    last_exc: Exception | None = None
    for cand in candidates:
        try:
            if cand == "fish_s2":
                from .fish_s2 import FishS2Backend  # 延迟 import 防 license 污染

                return FishS2Backend(tts_cfg, models_registry)
            if cand == "cosyvoice2":
                return CosyVoice2Backend(tts_cfg, models_registry)
            return BrowserFallbackBackend(tts_cfg)
        except XiaobaiError as exc:
            last_exc = exc
            continue
        except Exception as exc:  # noqa: BLE001
            last_exc = exc
            continue

    raise XiaobaiError(
        code=ErrorCode.RUNTIME,
        message="TTS 所有后端均不可用。建议安装 CosyVoice2 或改用浏览器朗读。",
        cause=last_exc,
    )
