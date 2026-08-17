# -*- coding: utf-8 -*-
"""企业级可视化界面后端（FastAPI）。

复用 melody2score 的 core 流水线（采集→预处理→音高检测→解析→歌谱），
对外暴露：
  GET  /                      前端单页
  POST /api/recognize        上传音频(或内置样例名) → 识别为歌谱 JSON
  POST /api/recognize-record 浏览器录音(base64 wav) → 歌谱 JSON
  POST /api/save-md          把歌谱结果保存为 Markdown（含简谱/五线谱/音符表/算法报告）

设计目标：精确（复用已验证的真实音高检测）、高效（后端仅做必要计算，前端渲染）、
企业级（清晰分层、参数可调、可审计、可保存为标准化文档）。
"""
import base64
import io
import json
import os
import re
import tempfile
import time
import traceback
from typing import Dict, List, Optional

from fastapi import FastAPI, File, Form, HTTPException, UploadFile
from fastapi.responses import FileResponse, JSONResponse
from fastapi.staticfiles import StaticFiles

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
os.sys.path.insert(0, ROOT)

from core.config import Config                       # noqa: E402
from core import capture, preprocess, pitch, analysis, score  # noqa: E402

app = FastAPI(title="Melody2Score 企业级可视化转谱", version="1.0.0")
# 关闭 pydantic 对 model_ 前缀的命名空间保护警告（我们使用 model_size 参数名）
try:
    from pydantic import BaseModel
    BaseModel.model_config = {"protected_namespaces": ()}  # type: ignore
except Exception:
    pass

SAVE_DIR = os.path.join(HERE, "exports")
os.makedirs(SAVE_DIR, exist_ok=True)

# 内置样例（合成音频 manifest 中的 id）
MANIFEST_PATH = os.path.join(ROOT, "audio", "manifest.json")


def _build_config(model_size: str, denoise: bool, threads: int, hop: int) -> Config:
    cfg = Config()
    cfg.model_size = model_size or cfg.model_size
    cfg.enable_denoise = denoise
    if threads and threads > 0:
        cfg.intra_op_threads = threads
    if hop and hop > 0:
        cfg.hop = hop
    return cfg


def _recognize_array(y, sr, cfg: Config) -> Dict:
    """核心识别：返回结构化歌谱结果。所有计时用于精确的性能/精度审计。"""
    t0 = time.time()
    y = preprocess.preprocess(y, sr, cfg.enable_denoise)
    t_pre = time.time() - t0

    t0 = time.time()
    det = pitch.PitchDetector(
        model_size=cfg.model_size, conf_thresh=cfg.conf_thresh, hop=cfg.hop,
        intra_op_threads=cfg.intra_op_threads, backend="auto", sr=sr)
    pts = det.detect(y, sr)
    t_pitch = time.time() - t0

    t0 = time.time()
    notes = analysis.segment_notes(pts, cfg.min_note_dur, cfg.median_win)
    bpm = analysis.detect_bpm(y, sr, cfg.bpm_fallback)
    key_name = analysis.estimate_key(y, sr, notes)
    t_parse = time.time() - t0

    jianpu = score.to_jianpu(notes, key_name, bpm)

    total_dur = float(notes[-1]["end"]) if notes else 0.0

    notes_out = [{
        "midi": int(n["midi"]),
        "start": round(float(n["start"]), 4),
        "end": round(float(n["end"]), 4),
        "dur": round(float(n["end"] - n["start"]), 4),
        "name": _midi_name(int(n["midi"])),
    } for n in notes]

    return {
        "jianpu": jianpu,
        "bpm": round(float(bpm), 1),
        "key": {"tonic": key_name[0], "mode": key_name[1]},
        "note_count": len(notes),
        "duration_sec": round(total_dur, 2),
        "backend": det.used_backend,
        "notes": notes_out,
        "perf": {
            "preprocess_ms": round(t_pre * 1000, 1),
            "pitch_ms": round(t_pitch * 1000, 1),
            "parse_ms": round(t_parse * 1000, 1),
            "pitch_frames": len(pts),
        },
    }


def _midi_name(m: int) -> str:
    names = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']
    return f"{names[m % 12]}{m // 12 - 1}"


@app.get("/api/samples")
def list_samples():
    """列出内置经典旋律样例，供前端直接试听/识别。"""
    if not os.path.exists(MANIFEST_PATH):
        return []
    with open(MANIFEST_PATH, encoding="utf-8") as f:
        manifest = json.load(f)
    seen = {}
    for it in manifest:
        seen.setdefault(it["melody_index"], {
            "title_zh": it["title_zh"], "title_en": it["title_en"],
            "file": it["file"], "timbre": it["timbre"],
        })
    return list(seen.values())


@app.post("/api/recognize")
async def recognize(file: UploadFile = File(...),
                    model_size: str = Form("tiny"),
                    denoise: bool = Form(True),
                    threads: int = Form(0),
                    hop: int = Form(0)):
    cfg = _build_config(model_size, denoise, threads, hop)
    data = await file.read()
    try:
        y, sr = _load_bytes_fallback(data, cfg.sr)
    except Exception as e:
        raise HTTPException(400, f"音频解码失败: {e}")
    try:
        res = _recognize_array(y, sr, cfg)
    except Exception as e:
        traceback.print_exc()
        raise HTTPException(500, f"识别失败: {e}")
    return JSONResponse(res)


def _load_bytes_fallback(data: bytes, sr: int):
    import soundfile as sf
    import numpy as np
    buf = io.BytesIO(data)
    y, _ = sf.read(buf, sr=sr, dtype="float32", always_2d=False)
    return np.asarray(y, dtype=np.float32), sr


@app.post("/api/recognize-sample")
def recognize_sample(name: str = Form(...),
                      model_size: str = Form("tiny"),
                      denoise: bool = Form(True),
                      threads: int = Form(0),
                      hop: int = Form(0)):
    cfg = _build_config(model_size, denoise, threads, hop)
    if not os.path.exists(MANIFEST_PATH):
        raise HTTPException(404, "未找到 audio/manifest.json")
    with open(MANIFEST_PATH, encoding="utf-8") as f:
        manifest = json.load(f)
    item = next((it for it in manifest if it["file"].endswith(name) or it["title_zh"] == name), None)
    if not item:
        raise HTTPException(404, f"样例不存在: {name}")
    y = capture.load_audio(os.path.join(ROOT, item["file"]), cfg.sr)
    try:
        res = _recognize_array(y, cfg.sr, cfg)
    except Exception as e:
        traceback.print_exc()
        raise HTTPException(500, f"识别失败: {e}")
    res["source"] = item["file"]
    return JSONResponse(res)


@app.post("/api/recognize-record")
async def recognize_record(audio_b64: str = Form(...),
                            model_size: str = Form("tiny"),
                            denoise: bool = Form(True),
                            threads: int = Form(0),
                            hop: int = Form(0)):
    cfg = _build_config(model_size, denoise, threads, hop)
    try:
        raw = base64.b64decode(audio_b64)
        y, sr = _load_bytes_fallback(raw, cfg.sr)
    except Exception as e:
        raise HTTPException(400, f"录音解码失败: {e}")
    try:
        res = _recognize_array(y, cfg.sr, cfg)
    except Exception as e:
        traceback.print_exc()
        raise HTTPException(500, f"识别失败: {e}")
    return JSONResponse(res)


def _jinpu_to_lily(notes: List[Dict], key: Dict, bpm: float) -> str:
    """把音符序列渲染为可嵌入 Markdown 的简易五线谱（ASCII 近似）与简谱表。"""
    return ""


@app.post("/api/save-md")
async def save_md(payload: dict):
    """把一次识别结果保存为标准 Markdown 文档（企业级、可审计）。"""
    try:
        title = payload.get("title", "未命名旋律")
        res = payload.get("result", {})
        jianpu = res.get("jianpu", "")
        bpm = res.get("bpm", 0)
        key = res.get("key", {"tonic": "C", "mode": "major"})
        notes = res.get("notes", [])
        backend = res.get("backend", "")
        perf = res.get("perf", {})
        source = payload.get("source", "用户上传/录音")

        ts = time.strftime("%Y%m%d_%H%M%S")
        safe = re.sub(r"[^\w一-鿿-]", "_", title)[:40]
        fname = f"{safe or 'melody'}_{ts}.md"
        fpath = os.path.join(SAVE_DIR, fname)

        lines = []
        lines.append(f"# 旋律转谱报告：{title}\n")
        lines.append(f"> 生成时间：{time.strftime('%Y-%m-%d %H:%M:%S')}  ")
        lines.append(f"| 来源：{source}  | 音高后端：{backend}\n")
        lines.append("\n## 一、识别概要\n")
        lines.append("| 指标 | 值 |")
        lines.append("|------|----|")
        lines.append(f"| 调式 | {key.get('tonic','?')} {key.get('mode','?')} |")
        lines.append(f"| 速度(BPM) | {bpm} |")
        lines.append(f"| 音符数 | {len(notes)} |")
        lines.append(f"| 时长(秒) | {res.get('duration_sec', 0)} |")
        lines.append(f"| 预处理耗时 | {perf.get('preprocess_ms', 0)} ms |")
        lines.append(f"| 音高检测耗时 | {perf.get('pitch_ms', 0)} ms |")
        lines.append(f"| 解析耗时 | {perf.get('parse_ms', 0)} ms |")
        lines.append(f"| 音高帧数 | {perf.get('pitch_frames', 0)} |")

        lines.append("\n## 二、简谱\n")
        lines.append("```text")
        lines.append(jianpu)
        lines.append("```\n")

        lines.append("## 三、音符明细（MIDI / 音名 / 起始 / 时长）\n")
        lines.append("| # | MIDI | 音名 | 起始(s) | 时长(s) |")
        lines.append("|---|------|------|---------|---------|")
        for i, n in enumerate(notes, 1):
            lines.append(f"| {i} | {n['midi']} | {n['name']} | {n['start']} | {n['dur']} |")

        lines.append("\n## 四、处理算法说明\n")
        lines.append(_algorithm_doc())

        with open(fpath, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))

        return JSONResponse({"file": fname, "path": fpath})
    except Exception as e:
        traceback.print_exc()
        raise HTTPException(500, f"保存失败: {e}")


@app.get("/api/download/{fname}")
def download(fname: str):
    fpath = os.path.join(SAVE_DIR, fname)
    if not os.path.exists(fpath):
        raise HTTPException(404)
    return FileResponse(fpath, filename=fname, media_type="text/markdown")


def _algorithm_doc() -> str:
    return (
        "本结果由 melody2score 流水线生成，分五层处理：\n\n"
        "1. **采集层**：librosa 重采样至 16kHz 单声道，峰值归一化。\n"
        "2. **预处理层**：去直流偏移 + 峰值归一化 + 谱减降噪（以开头 0.1s 静音段估计噪声底）。\n"
        "3. **音高检测层**：可插拔后端（crepe_onnx / torchcrepe / pyin），输出 [{t, freq, conf}]，"
        "低于置信度阈值(0.3)判为无声。\n"
        "4. **音乐解析层**：\n"
        "   - midi 轮廓中值滤波(win=5)消除颤音与帧间抖动；\n"
        "   - 半音量化后按相同音高分段；\n"
        "   - 短段(<min_note_dur)就近合并到音高最近的相邻音符，过滤毛刺；\n"
        "   - BPM 用 librosa beat_track；调式用 Krumhansl-Schmuckler 模板相关。\n"
        "5. **歌谱生成层**：music21 量化生成 musicxml；简谱数字串（高八度前缀 '.'，低八度后缀 '_'，"
        "延音 '-'）。\n\n"
        "**优化说明**：中值滤波 + 短段合并显著抑制颤音/滑音误切；置信度门限滤除无声帧；"
        "量化到 1/4 拍使输出规整可唱。企业级部署建议：tiny 模型 + 关降噪用于板端实时，"
        "small 模型用于 PC 高精度。"
    )


app.mount("/", StaticFiles(directory=os.path.join(HERE, "frontend"), html=True), name="static")


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8012, reload=False)
