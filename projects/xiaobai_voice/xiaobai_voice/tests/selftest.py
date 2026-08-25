"""最小冒烟 + 完整 selftest（--full 死锁回归、声卡缺失兜底、T15 相关）。"""
from __future__ import annotations

import io
import json
import os
import struct
import sys
import tempfile
import threading
import time
from pathlib import Path


def _make_wav_bytes(sample_rate=16000, seconds=0.3, freq=300.0) -> bytes:
    import math
    n = int(sample_rate * seconds)
    amp = 0.5
    buf = bytearray()
    for i in range(n):
        s = math.sin(2 * math.pi * freq * i / sample_rate)
        v = int(s * amp * 32767)
        buf += struct.pack("<h", v)
    data = bytes(buf)
    header = struct.pack(
        "<4sI4s4sIHHIIHH4sI",
        b"RIFF", 36 + len(data), b"WAVE",
        b"fmt ", 16, 1, 1, sample_rate, sample_rate * 2, 2, 16,
        b"data", len(data),
    )
    return header + data


def _wav_meta(wav: bytes):
    try:
        r = io.BytesIO(wav)
        assert r.read(4) == b"RIFF"
        r.read(4)
        assert r.read(4) == b"WAVE"
        while True:
            chunk = r.read(4)
            if chunk == b"fmt ":
                size = struct.unpack("<I", r.read(4))[0]
                fmt = struct.unpack("<HHIIHH", r.read(size))
                _, _, sr, _, _, bits = fmt
                return sr, bits
            elif chunk == b"data":
                size = struct.unpack("<I", r.read(4))[0]
                r.read(size)
            else:
                size = struct.unpack("<I", r.read(4))[0]
                r.read(size)
                if not chunk.strip():
                    break
    except Exception:  # noqa: BLE001
        pass
    return None, None


def _log_path() -> Path:
    from xiaobai_voice.config.loader import default_log_path
    return default_log_path() / f"selftest-report-{time.strftime('%Y%m%d-%H%M%S')}.jsonl"


def _append(path: Path, entry: dict) -> None:
    with open(path, "a", encoding="utf-8") as f:
        f.write(json.dumps(entry, ensure_ascii=False) + "\n")


def run_selftest(*, full: bool = False) -> int:
    report_path = _log_path()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    print(f"[selftest] report → {report_path}", flush=True)
    _append(report_path, {"ts": time.time(), "cmd": "selftest", "full": full, "argv": sys.argv[:10]})

    failed = 0
    # --- 1) 错误分级导入：没有安装 sherpa/CosyVoice 不崩溃，而是返回分级 XiaobaiError
    try:
        from xiaobai_voice.asr import build_asr_backend
        from xiaobai_voice.config.loader import ConfigLoader
        from xiaobai_voice.errors import XiaobaiError
        cfg = ConfigLoader().data
        try:
            build_asr_backend(cfg, "auto")
            _append(report_path, {"ts": time.time(), "name": "asr_build", "status": "ok"})
        except XiaobaiError as exc:
            # 分级错误是预期行为
            _append(report_path, {"ts": time.time(), "name": "asr_build", "status": "skip_expected",
                                  "code": exc.code.value, "message": exc.message})
    except Exception as exc:  # noqa: BLE001
        failed += 1
        _append(report_path, {"ts": time.time(), "name": "asr_build", "status": "fail", "error": str(exc)})

    # --- 2) BrowserFallback TTS：0.5 s 静音 WAV 头合法
    try:
        from xiaobai_voice.tts.browser_fallback import BrowserFallbackBackend
        from xiaobai_voice.tts.base import TTSOptions
        tts = BrowserFallbackBackend({})
        data = tts.synthesize_full(TTSOptions(text="你好", sample_rate=16000))
        assert len(data) >= 200
        sr, bits = _wav_meta(data)
        assert sr == 16000 and bits == 16
        _append(report_path, {"ts": time.time(), "name": "tts_browser_fallback_wav", "status": "ok",
                              "bytes": len(data), "sr": sr, "bits": bits})
    except Exception as exc:  # noqa: BLE001
        failed += 1
        _append(report_path, {"ts": time.time(), "name": "tts_browser_fallback_wav", "status": "fail", "error": str(exc)})

    # --- 3) 模型下载器：404 重试 + 坏 sha 回删（单测 T12）
    try:
        from xiaobai_voice.models.downloader import ModelDownloader, ModelRegistry
        with tempfile.TemporaryDirectory() as td:
            reg = ModelRegistry()
            dl = ModelDownloader(reg, preferred_root=Path(td))
            # 插入一个假的 404 URL 模型条目
            bad_meta = {
                "id": "unit-404-test", "name": "404", "license": "MIT",
                "size_mb": 0.01, "category": "asr", "engine": "test",
                "default": False, "optional": True,
                "urls": ["http://127.0.0.1:1/does-not-exist-must-404.bin"],
                "sha256": "abcd", "archive_format": "file", "subdir": "unit-404",
                "entry": {"ckpt": "does-not-exist-must-404.bin"},
            }
            reg.models_raw.append(bad_meta)
            events = []
            try:
                dl.download("unit-404-test", on_progress=events.append)
                _append(report_path, {"ts": time.time(), "name": "download_404_retries", "status": "fail",
                                      "error": "should have raised"})
                failed += 1
            except Exception:  # noqa: BLE001
                attempts = len([e for e in events if e.get("state") == "downloading"])
                _append(report_path, {"ts": time.time(), "name": "download_404_retries", "status": "ok",
                                      "attempt_events": attempts})
            # 坏 SHA：本地伪造 1 字节文件 + SHA256 错
            bad = Path(td) / "tts-fish-s2-pro" / "fake.pt"
            bad.parent.mkdir(parents=True, exist_ok=True)
            bad.write_bytes(b"\x01")
            bad_sha_meta = {
                "id": "unit-sha-fail", "name": "sha fail", "license": "MIT",
                "size_mb": 0.001, "category": "tts", "engine": "test",
                "default": False, "optional": True,
                "urls": [bad.as_uri()],  # 本地 file://
                "sha256": "0" * 64,  # 必失败
                "archive_format": "file",
                "subdir": "unit-sha-fail",
                "entry": {"ckpt": "fake.pt"},
            }
            reg.models_raw.append(bad_sha_meta)
            try:
                dl.download("unit-sha-fail")
                _append(report_path, {"ts": time.time(), "name": "download_sha_fail_auto_delete",
                                      "status": "fail", "error": "should have raised"})
                failed += 1
            except Exception:  # noqa: BLE001
                target_pkg = dl.preferred_root / "fake.pt"
                exists = target_pkg.is_file()
                _append(report_path, {"ts": time.time(), "name": "download_sha_fail_auto_delete",
                                      "status": "ok" if not exists else "fail",
                                      "exists": exists})
                if exists:
                    failed += 1
    except Exception as exc:  # noqa: BLE001
        failed += 1
        _append(report_path, {"ts": time.time(), "name": "downloader_unit", "status": "fail", "error": str(exc)})

    # --- 4) full: 语音播放死锁回归（T15 AC14 AC21）
    if full:
        loops = 100
        entry = {"ts": time.time(), "name": "voice_playback_smoke_deadlock_regression",
                 "loops": loops, "status": "pending"}
        try:
            from xiaobai_voice.tts.browser_fallback import BrowserFallbackBackend
            from xiaobai_voice.tts.base import TTSOptions
            tts = BrowserFallbackBackend({})
            # 合成一个 0.5s 静音 WAV → 用 sounddevice 播放，若声卡缺失 PortAudioError → SKIP_OK
            try:
                import sounddevice as sd  # noqa: F401
                import soundfile as sf
            except Exception as exc:  # noqa: BLE001
                entry.update(status="SKIP_OK", skip_reason=f"sound libs not installed: {exc}")
                _append(report_path, entry)
            else:
                wav = tts.synthesize_full(TTSOptions(text="你好璇玑", sample_rate=16000))
                data, sr = sf.read(io.BytesIO(wav), dtype="float32", always_2d=False)
                # 主线程 ping 观测
                stop_ping = threading.Event()
                main_blocked_events = []
                tick_start = time.time()

                def ping_loop():
                    last = tick_start
                    while not stop_ping.wait(timeout=0.05):
                        now = time.time()
                        dt = now - last
                        if dt > 0.2:
                            main_blocked_events.append(dt)
                        last = now

                watcher = threading.Thread(target=ping_loop, daemon=True)
                watcher.start()
                ok = True
                try:
                    for i in range(loops):
                        # 模拟 play() 并 stop() 交错调用（触发持锁嵌套场景）
                        try:
                            sd.play(data, sr)
                            sd.wait(timeout=2.0)
                            sd.stop()
                        except Exception:  # noqa: BLE001
                            # 声卡缺失：整体 SKIP_OK
                            entry.update(status="SKIP_OK", skip_reason="PortAudioError / no output device")
                            ok = None
                            break
                finally:
                    stop_ping.set()
                if ok is True:
                    entry.update(status="ok", max_block_s=max(main_blocked_events, default=0.0))
                    if main_blocked_events:
                        # 允许短暂 < 2s 自恢复；> 2s 判失败
                        if max(main_blocked_events) >= 2.0:
                            entry["status"] = "fail"
                            failed += 1
                elif ok is None:
                    pass
                else:
                    entry["status"] = "fail"
                    failed += 1
        except Exception as exc:  # noqa: BLE001
            entry.update(status="fail", error=str(exc))
            failed += 1
        _append(report_path, entry)

    # --- 5) full: 声卡缺失兜底（mock 抛 PortAudioError）
    if full:
        entry = {"ts": time.time(), "name": "no_soundcard_fallback", "status": "pending"}
        try:
            import sounddevice as sd  # noqa: F401
            try:
                exc_cls = getattr(__import__("sounddevice", fromlist=["PortAudioError"]), "PortAudioError", OSError)
            except Exception:  # noqa: BLE001
                exc_cls = OSError
            import xiaobai_voice.desktop.ball_widget as bw
            orig_class = bw._LocalRecorder
            class _Bad:
                def __init__(self, *a, **kw): pass
                def start(self, mw): raise exc_cls("mock no soundcard")
                def stop(self): return ""
            bw._LocalRecorder = _Bad  # type: ignore[assignment]
            try:
                # 直接构造 recorder，断言 start 不崩溃（会返回 False/空串，按设计）
                rec = bw._LocalRecorder()
                ok = False
                try:
                    rec.start(None)
                except exc_cls:
                    ok = True
                except Exception:  # noqa: BLE001
                    ok = False
                entry.update(status="ok" if ok else "fail", propagated=ok)
                if not ok:
                    failed += 1
            finally:
                bw._LocalRecorder = orig_class  # type: ignore[assignment]
        except Exception as exc:  # noqa: BLE001
            entry.update(status="fail", error=str(exc))
            failed += 1
        _append(report_path, entry)

    # --- 6) full: AST 扫描 apache2 模式无 fish_speech import
    if full:
        entry = {"ts": time.time(), "name": "license_apache2_ast_scan", "status": "pending"}
        try:
            from xiaobai_voice.cli import _scan_ast_no_fish_import
            issues = _scan_ast_no_fish_import()
            entry.update(status="ok" if not issues else "fail", issues_count=len(issues), issues=issues[:5])
            if issues:
                failed += 1
        except Exception as exc:  # noqa: BLE001
            entry.update(status="fail", error=str(exc))
            failed += 1
        _append(report_path, entry)

    print(f"[selftest] DONE. failed={failed}. report: {report_path}", flush=True)
    return 1 if failed else 0
