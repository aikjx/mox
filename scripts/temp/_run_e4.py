"""E-4: 三层代理链路（:3021→3001→3717）验证脚本。
E-4-1 通过 :3001 走 voice_proxy 请求 /voice/health，验证 TTS 引擎已正确上报。
E-4-2 通过 :3001 走 voice_proxy 请求 /voice/tts/stream（合成 10 字短句），验证 WAV 22050Hz + engine=cosyvoice2 + DSP=Rust。
"""
from __future__ import annotations

import struct
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

GATEWAY = "http://127.0.0.1:3001"
SHORT_TEXT = "你好，今天天气晴朗，适合工作。"  # 15 字以内，TTS 应 < 30s


def parse_wav_meta(data: bytes) -> dict:
    """WAV 元数据解析，返回 {sample_rate, channels, duration_ms, bytes_per_sec, valid}。"""
    if len(data) < 44 or data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        return {"valid": False}
    # fmt chunk
    p = 12
    while p + 8 < len(data):
        ck_id = data[p : p + 4]
        ck_sz = struct.unpack_from("<I", data, p + 4)[0]
        if ck_id == b"fmt ":
            if ck_sz < 16 or p + 8 + 16 > len(data):
                return {"valid": False}
            _, channels, sr, bps, _, _ = struct.unpack_from("<HHIIHH", data, p + 8)
            out = {"valid": True, "channels": channels, "sample_rate": sr, "bytes_per_sec": bps}
            # 找 data chunk 算 samples / duration
            q = p + 8 + ck_sz
            while q + 8 < len(data):
                id2 = data[q : q + 4]
                sz2 = struct.unpack_from("<I", data, q + 4)[0]
                if id2 == b"data":
                    bytes_per_sample = channels * ((bps + 7) // 8)
                    samples = sz2 // max(bytes_per_sample, 1)
                    out["data_bytes"] = sz2
                    out["samples"] = samples
                    out["duration_ms"] = (samples * 1000) // sr if sr else 0
                    break
                q += 8 + sz2
            return out
        p += 8 + ck_sz
    return {"valid": False}


def e41_health_via_gateway() -> bool:
    url = f"{GATEWAY}/voice/health"
    t0 = time.time()
    try:
        with urllib.request.urlopen(url, timeout=90) as resp:
            body = resp.read().decode("utf-8", "replace")
    except Exception as exc:  # noqa: BLE001
        print(f"E-4-1 FAIL: health via :3001 error={exc}")
        return False
    dt = time.time() - t0
    print(f"E-4-1 GET {url} status=200 time={dt:.2f}s body_1k={body[:600]}")
    import json

    try:
        payload = json.loads(body)
    except Exception as exc:  # noqa: BLE001
        print(f"E-4-1 FAIL: JSON 解析失败 {exc}")
        return False
    tts = payload.get("tts", {}) if isinstance(payload, dict) else {}
    engines = tts.get("engines", []) if isinstance(tts, dict) else []
    cosy = {}
    for e in engines:
        if isinstance(e, dict) and e.get("name") == "cosyvoice2":
            cosy = e
            break
    ok_ready = (tts.get("ready") is True) and (cosy.get("available") is True)
    ok_dsp = (tts.get("rust_dsp_available") is True) or (cosy.get("rust_dsp_available") is True)
    ok_active = tts.get("active") == "cosyvoice2"
    print(f"  -> tts.ready={tts.get('ready')}  tts.active={tts.get('active')}  cosy.available={cosy.get('available')}  rust_dsp_available(gw)={ok_dsp}")
    if not (ok_ready and ok_dsp and ok_active):
        print("E-4-1 FAIL: engine not ready / active != cosyvoice2 / rust dsp missing")
        return False
    return True


def e42_tts_via_gateway() -> bool:
    params = urllib.parse.urlencode({"text": SHORT_TEXT, "speed": "1.0", "engine": "cosyvoice2"})
    url = f"{GATEWAY}/voice/tts/stream?{params}"
    t0 = time.time()
    try:
        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=600) as resp:
            code = resp.getcode()
            # urllib 的 HTTPMessage headers 用 list of tuples 返回，且 header 名小写
            headers = {str(k).lower(): str(v) for k, v in resp.headers.items()}
            data = resp.read()
    except Exception as exc:  # noqa: BLE001
        dt = time.time() - t0
        print(f"E-4-2 FAIL: TTS via :3001 error={exc} time={dt:.2f}s")
        return False
    dt = time.time() - t0
    ct = headers.get("content-type", "")
    engine = headers.get("x-tts-engine", "")
    dsp_impl = headers.get("x-tts-dsp-impl", "")
    cl = headers.get("content-length", "")
    print(
        f"E-4-2 GET {url.split('?')[0]}?... status={code} time={dt:.2f}s bytes={len(data)} "
        f"Content-Type={ct}  Engine={engine}  DSP={dsp_impl}  CL={cl}"
    )
    if code != 200 or ct.lower().split(";")[0].strip() != "audio/wav":
        # 如果返回了 JSON，说明上游不可达/降级
        snippet = data[:300].decode("utf-8", "replace")
        print(f"  非 audio/wav，首 300 bytes={snippet}")
        return False
    if engine != "cosyvoice2":
        print("  FAIL: engine header 不是 cosyvoice2")
        return False
    if dsp_impl != "Rust":
        print(f"  FAIL: dsp_impl={dsp_impl!r} 不是 Rust")
        return False
    meta = parse_wav_meta(data)
    if not meta["valid"]:
        print("  FAIL: WAV 元数据无效")
        return False
    sr = meta["sample_rate"]
    dur = meta.get("duration_ms", 0)
    print(f"  -> WAV sample_rate={sr}Hz  duration={dur}ms  data_bytes={meta.get('data_bytes','?')}")
    if sr != 22050:
        print("  FAIL: 采样率 != 22050")
        return False
    if dur < 500:
        print("  FAIL: 合成过短 < 0.5s")
        return False
    return True


def main() -> int:
    h = e41_health_via_gateway()
    print()
    t = e42_tts_via_gateway()
    print()
    print(f"E-4 汇总：health={h}  tts={t}  overall={'PASS' if (h and t) else 'FAIL'}")
    return 0 if (h and t) else 1


if __name__ == "__main__":
    sys.exit(main())
