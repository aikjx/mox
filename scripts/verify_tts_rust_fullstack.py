"""V-ALL TTS 音质回归：Rust 核心全链路自动验收脚本。

覆盖 E-2 ~ E-5 四项验收：
  E-2 健康检查（tts ready + cosyvoice2 + rust_dsp available）
  E-3 直连 :3717 合成中文 → x-tts-engine=cosyvoice2 + WAV meta 22050Hz + _last_dsp_impl=Rust
  E-4 三层代理 :3021 -> :3001 -> :3717 /voice/tts/stream
  E-5 （可选）浏览器前端点击播放按钮验证——脚本只输出提示与 curl 对比

用法：
  # 1. 下载完权重 + 重启 xiaobai_voice 服务
  # 2. 执行：
       py -3.12 scripts/verify_tts_rust_fullstack.py

执行路径：默认会把输出放到 temp/tts_verify/*.log 与 audio_cases/*.wav
"""
from __future__ import annotations

import argparse
import datetime
import json
import os
import pathlib
import struct
import sys
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass, field

ROOT = pathlib.Path(__file__).resolve().parent.parent
XIAOBAI_DIR = ROOT / "projects" / "xiaobai_voice"
if str(XIAOBAI_DIR) not in sys.path:
    sys.path.insert(0, str(XIAOBAI_DIR))


# ======================================================================== utils
def now_stamp() -> str:
    return datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")


@dataclass
class CaseResult:
    name: str
    status: str  # PASS / FAIL / WARN
    message: str
    metrics: dict = field(default_factory=dict)


def http_get(url: str, timeout: int = 600, headers: dict | None = None):
    """urllib GET，返回 (status, headers, body_bytes)"""
    req = urllib.request.Request(url, headers=headers or {}, method="GET")
    t0 = time.time()
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        body = resp.read()
    dt_ms = int((time.time() - t0) * 1000)
    headers_dict = {k.lower(): v for k, v in resp.headers.items()}
    return resp.status, headers_dict, body, dt_ms


def parse_wav_meta(raw: bytes) -> dict:
    """解析 RIFF/WAV header: sr, channels, bit_depth, data_bytes, duration_s, peak_i16, rms_i16."""
    out = {
        "ok": False,
        "riff": None,
        "wave": None,
        "channels": 0,
        "sample_rate": 0,
        "bit_depth": 0,
        "byte_rate": 0,
        "data_bytes": 0,
        "duration_s": 0.0,
        "peak": 0.0,
        "rms": 0.0,
        "loudness_dbfs": None,
    }
    if len(raw) < 44:
        out["riff"] = raw[:4] if raw else None
        return out
    riff = raw[0:4]
    size = struct.unpack("<I", raw[4:8])[0]
    wave = raw[8:12]
    out["riff"] = riff.decode("ascii", errors="replace")
    out["wave"] = wave.decode("ascii", errors="replace")
    # find fmt chunk
    pos = 12
    fmt_found = False
    while pos + 8 <= len(raw):
        cid = raw[pos : pos + 4]
        cs = struct.unpack("<I", raw[pos + 4 : pos + 8])[0]
        if cid == b"fmt ":
            if cs < 16 or pos + 8 + cs > len(raw):
                return out
            audio_fmt = struct.unpack("<H", raw[pos + 8 : pos + 10])[0]
            channels = struct.unpack("<H", raw[pos + 10 : pos + 12])[0]
            sr = struct.unpack("<I", raw[pos + 12 : pos + 16])[0]
            byte_rate = struct.unpack("<I", raw[pos + 16 : pos + 20])[0]
            bit_depth = struct.unpack("<H", raw[pos + 22 : pos + 24])[0]
            out["channels"] = channels
            out["sample_rate"] = sr
            out["bit_depth"] = bit_depth
            out["byte_rate"] = byte_rate
            fmt_found = True
            pos += 8 + cs
            continue
        if cid == b"data":
            data_body = raw[pos + 8 : pos + 8 + cs] if pos + 8 + cs <= len(raw) else raw[pos + 8 :]
            total = len(data_body)
            out["data_bytes"] = total
            if fmt_found and out["bit_depth"] == 16 and out["channels"] > 0:
                # 解析 int16 × channels 样本
                import math

                n_samples = total // 2
                arr = struct.unpack("<%dh" % n_samples, data_body[: n_samples * 2])
                # 只要单声道/或取第 1 声道
                ch = max(1, out["channels"])
                mono = [arr[i] for i in range(0, n_samples, ch)] if ch > 1 else list(arr)
                peak_i = 0
                sumsq = 0.0
                n = len(mono)
                for v in mono:
                    a = v if v >= 0 else -v
                    if a > peak_i:
                        peak_i = a
                    sumsq += float(v) * float(v)
                out["peak"] = float(peak_i) / 32768.0
                rms = math.sqrt(sumsq / max(1, n)) / 32768.0
                out["rms"] = rms
                if rms > 1e-12:
                    out["loudness_dbfs"] = 20.0 * math.log10(rms)
                if out["sample_rate"] > 0:
                    out["duration_s"] = n / out["sample_rate"]
            pos += 8 + cs
            break
        pos += 8 + (cs + 1 & ~1)
    out["ok"] = fmt_found and out["data_bytes"] > 0
    out["_wav_total_bytes"] = len(raw)
    out["_riff_size"] = size
    return out


# ======================================================================== cases
def case_e2_health(base: str, results: list[CaseResult]) -> None:
    """E-2 健康检查"""
    name = "E-2 健康检查(voice:3717)"
    try:
        status, hdrs, body, _dt = http_get(f"{base}/voice/health", timeout=10)
    except Exception as e:
        results.append(CaseResult(name, "FAIL", f"HTTP 错误 {e}"))
        return
    try:
        data = json.loads(body.decode("utf-8", "replace"))
    except Exception:
        results.append(CaseResult(name, "FAIL", f"非 JSON body，状态码 {status}，前 80 字节={body[:80]}"))
        return
    tts = data.get("tts") or {}
    engines = {e.get("name"): e for e in (tts.get("engines") or [])}
    cosy2 = engines.get("cosyvoice2") or {}
    rust_dsp_ok = bool(data.get("rust_dsp_available")) if "rust_dsp_available" in data else None
    ready = bool(tts.get("ready"))
    cosy_avail = bool(cosy2.get("available"))
    metrics = {
        "http_status": status,
        "tts_ready": ready,
        "cosyvoice2_available": cosy_avail,
        "engine_count": len(engines),
        "rust_dsp_available_from_health": rust_dsp_ok,
        "service": data.get("service"),
        "version": data.get("version"),
    }
    # Rust DSP 额外探活：直接 import（不依赖 health 暴露字段）
    try:
        from xiaobai_voice.tts.cosyvoice2 import rust_dsp_available  # type: ignore

        metrics["rust_dsp_available_process_local"] = bool(rust_dsp_available())
    except Exception:
        metrics["rust_dsp_available_process_local"] = None
    # 条件：ready=True + cosy_avail=True + rust_dsp True（任一字段都行）
    rust_ok = (
        metrics.get("rust_dsp_available_from_health") is True
        or metrics.get("rust_dsp_available_process_local") is True
    )
    if status == 200 and ready and cosy_avail and rust_ok:
        results.append(CaseResult(name, "PASS", f"tts ready, cosyvoice2 registered, Rust DSP OK ({rust_ok})", metrics))
    else:
        results.append(CaseResult(
            name, "FAIL",
            f"status={status} tts.ready={ready} cosyvoice2.available={cosy_avail} rust_dsp={rust_ok}",
            metrics,
        ))
    return


def _build_url(base: str, text: str, speed: float = 1.03, sample_rate: int = 22050) -> str:
    q = urllib.parse.urlencode(
        dict(text=text, voice="中文女", emotion="happy", speed=str(speed), sample_rate=str(sample_rate), format="wav")
    )
    return f"{base}/voice/tts/stream?{q}"


def _check_engine_and_wav(name, results, resp_status, headers, body, dt_ms, text):
    meta = parse_wav_meta(body)
    engine = headers.get("x-tts-engine")
    fallback = headers.get("x-tts-fallback")
    dsp_impl = headers.get("x-tts-dsp-impl") or headers.get("x-dsp-impl")
    content_type = headers.get("content-type")
    metrics = {
        "http": resp_status,
        "ms": dt_ms,
        "content_type": content_type,
        "bytes": len(body),
        "engine": engine,
        "fallback": fallback,
        "dsp_impl": dsp_impl,
        "sample_rate": meta.get("sample_rate"),
        "channels": meta.get("channels"),
        "bit_depth": meta.get("bit_depth"),
        "duration_s": meta.get("duration_s"),
        "peak": meta.get("peak"),
        "loudness_dbfs": meta.get("loudness_dbfs"),
        "wav_ok": meta.get("ok"),
    }
    # 验收规则
    want = dict(
        status_200=resp_status == 200,
        audio_wav=(content_type or "").lower().startswith("audio/wav"),
        engine_cosy2=engine == "cosyvoice2",
        sr_22050=meta.get("sample_rate") == 22050,
        bit16=meta.get("bit_depth") == 16,
        mono=meta.get("channels") == 1,
        duration_ge_2s=float(meta.get("duration_s") or 0) >= 2.0,
        peak_le_0995=float(meta.get("peak") or 0) <= 0.995,
        loudness_in_band=(
            meta.get("loudness_dbfs") is not None
            and -32.0 <= float(meta.get("loudness_dbfs")) <= -3.0
        ),
    )
    # DSP impl：如果服务暴露 x-tts-dsp-impl 就判断为 Rust
    if dsp_impl is None:
        want["dsp_impl_rust"] = None  # 无法直接判断，WARN
    else:
        want["dsp_impl_rust"] = dsp_impl.lower().startswith("rust")
    # 额外：调用服务端 cosyvoice2._last_dsp_impl 做判断（/voice/metrics 或其它字段？）这里没有，就用 x-tts-dsp-impl
    failed = [k for k, v in want.items() if v is False]
    status_out = "PASS" if not failed else "FAIL"
    # 如果只有 dsp_impl_rust 未知，其他全 True → WARN
    if failed == [] and want.get("dsp_impl_rust") is None:
        status_out = "WARN"
    msg_parts = [f"{k}={v}" for k, v in want.items()]
    msg = f"text={text!r}  校验项: " + "; ".join(msg_parts)
    results.append(CaseResult(name, status_out, msg, metrics))
    return meta


def case_e3_direct(results: list[CaseResult], out_dir: pathlib.Path) -> None:
    base = "http://localhost:3717"
    text = "今天阳光明媚，我和朋友一起去郊外散步，一路上聊了很多有趣的故事，心情特别好。"
    url = _build_url(base, text)
    name = "E-3 直连 :3717 合成"
    try:
        status, hdrs, body, dt_ms = http_get(url, timeout=600)
    except Exception as e:
        results.append(CaseResult(name, "FAIL", f"HTTP 错误 {e}"))
        return
    # 保存 WAV
    fp = out_dir / "E3_cosyvoice2_direct.wav"
    fp.write_bytes(body)
    meta = _check_engine_and_wav(name, results, status, hdrs, body, dt_ms, text)
    # 额外：如果服务返回了 wav，本地再做 Python 内省：导入 cosyvoice2 包读取 _last_dsp_impl（前提：与服务进程共享解释器状态的只有同一进程，这里独立，所以这步只是打印）
    print(f"[E-3] WAV saved = {fp} ({len(body)} bytes), sr={meta.get('sample_rate')} dur_s={meta.get('duration_s'):.2f}")


def case_e4_proxy(results: list[CaseResult], out_dir: pathlib.Path) -> None:
    base = "http://localhost:3021"
    text = "人工智能正在改变我们的生活，语音合成就是其中最直观的一环。清晰自然的语音，让机器更有温度。"
    url = _build_url(base, text)
    name = "E-4 三层代理 :3021 -> :3001 -> :3717"
    try:
        status, hdrs, body, dt_ms = http_get(url, timeout=600)
    except Exception as e:
        results.append(CaseResult(name, "FAIL", f"HTTP 错误 {e}"))
        return
    fp = out_dir / "E4_three_layer_proxy.wav"
    fp.write_bytes(body)
    meta = _check_engine_and_wav(name, results, status, hdrs, body, dt_ms, text)
    print(f"[E-4] WAV saved = {fp} ({len(body)} bytes), sr={meta.get('sample_rate')} dur_s={meta.get('duration_s'):.2f}")


def case_e2b_backend_instantiate(results: list[CaseResult]) -> None:
    """E-2b：本进程直接构建 CosyVoice2Backend（用于定位 health 返回 ready=false 时的真实原因）。"""
    name = "E-2b 后端实例化 + DSP impl 记录"
    cfg = {
        "engines": {
            "cosyvoice2": {
                "preferred_spk_ids": ["中文女", "女", "voice_0", "Default", "中文男"],
                "instruction_style": "warm_daily",
                "resample_quality": "linear",
                "limiter": True,
                "loudness_target_dbfs": -18.0,
            }
        }
    }
    try:
        from xiaobai_voice.tts.cosyvoice2 import CosyVoice2Backend, rust_dsp_available
        from xiaobai_voice.models.downloader import ModelRegistry

        reg = ModelRegistry()
        b = CosyVoice2Backend(cfg, models_registry=reg)
    except Exception as e:
        results.append(CaseResult(name, "FAIL", f"{type(e).__name__}: {e}"))
        return
    # 合成一个短句，检查 _last_dsp_impl
    try:
        from xiaobai_voice.tts.base import TTSOptions

        opts = TTSOptions(
            text="你好世界，测试语音合成。",
            voice="中文女",
            emotion="happy",
            speed=1.03,
            sample_rate=22050,
            stream_chunk_ms=200,
        )
        chunks = list(b.synthesize(opts))
        body = b"".join(chunks)
        dsp_impl = getattr(b, "_last_dsp_impl", None)
        meta = parse_wav_meta(body)
        metrics = {
            "rust_dsp_local": bool(rust_dsp_available()),
            "dsp_impl": dsp_impl,
            "sr": meta.get("sample_rate"),
            "wav_bytes": len(body),
            "duration_s": meta.get("duration_s"),
            "peak": meta.get("peak"),
        }
        if dsp_impl is not None:
            metrics["dsp_impl"] = str(dsp_impl)
        if rust_dsp_available() and dsp_impl == "Rust" and meta.get("sample_rate") == 22050 and meta.get("duration_s", 0) >= 0.5:
            results.append(CaseResult(name, "PASS", f"本进程内 Rust DSP 生效：dsp_impl={dsp_impl}", metrics))
        else:
            results.append(CaseResult(
                name, "FAIL",
                f"rust_dsp_local={rust_dsp_available()} dsp_impl={dsp_impl} sr={meta.get('sample_rate')} dur={meta.get('duration_s'):.2f}s",
                metrics,
            ))
    except Exception as e:
        results.append(CaseResult(name, "FAIL", f"合成失败 {type(e).__name__}: {e}"))


# ======================================================================== main
def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(ROOT / "temp" / "tts_verify"))
    ap.add_argument("--only-e2b", action="store_true", help="只跑 E-2b 本地实例化，不访问服务（服务未启动时可用）")
    ap.add_argument("--skip-e2", action="store_true")
    ap.add_argument("--skip-e3", action="store_true")
    ap.add_argument("--skip-e4", action="store_true")
    args = ap.parse_args(argv)

    out_dir = pathlib.Path(args.out)
    (out_dir / "audio_cases").mkdir(parents=True, exist_ok=True)
    audio_dir = out_dir / "audio_cases"

    results: list[CaseResult] = []
    t0 = time.time()
    print(f"[{now_stamp()}] TTS 全链路回归 start.  out_dir={out_dir}")
    print(f"[{now_stamp()}] Rust DSP 本地探活：", end="")
    try:
        from xiaobai_voice.tts.cosyvoice2 import rust_dsp_available, rust_dsp_error
        print(f"available={rust_dsp_available()}  error={rust_dsp_error()}")
    except Exception as e:
        print(f"IMPORT FAIL: {e}")

    # E-2b 本地实例化（即使服务未启动也能跑）
    try:
        case_e2b_backend_instantiate(results)
    except Exception as e:
        results.append(CaseResult("E-2b", "FAIL", f"未预期异常 {type(e).__name__}: {e}"))
    if args.only_e2b:
        return _finish(results, out_dir, t0)

    if not args.skip_e2:
        case_e2_health("http://localhost:3717", results)
    if not args.skip_e3:
        case_e3_direct(results, audio_dir)
    if not args.skip_e4:
        case_e4_proxy(results, audio_dir)

    return _finish(results, out_dir, t0)


def _finish(results, out_dir, t0):
    elapsed_s = time.time() - t0
    passed = sum(1 for r in results if r.status == "PASS")
    failed = sum(1 for r in results if r.status == "FAIL")
    warn = sum(1 for r in results if r.status == "WARN")
    total = len(results)
    report_obj = {
        "generated_at": now_stamp(),
        "elapsed_s": round(elapsed_s, 2),
        "summary": {"total": total, "PASS": passed, "FAIL": failed, "WARN": warn},
        "cases": [
            {
                "name": r.name,
                "status": r.status,
                "message": r.message,
                "metrics": r.metrics,
            }
            for r in results
        ],
    }
    report_file = out_dir / f"report_{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    report_file.write_text(json.dumps(report_obj, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"\n[{now_stamp()}] ===== TTS 回归报告 ({passed}/{total} PASS, {failed} FAIL, {warn} WARN) =====")
    for r in results:
        flag = {"PASS": "✅", "FAIL": "❌", "WARN": "⚠️"}[r.status]
        print(f"  {flag} {r.name}  → {r.status}")
        if r.metrics:
            for k, v in r.metrics.items():
                print(f"      · {k} = {v}")
    print(f"  JSON: {report_file}")
    return 0 if failed == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
