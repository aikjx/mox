# -*- coding: utf-8 -*-
"""企业级 Melody2Score API 服务（FastAPI）。
全维分析、工业级转换、专业乐谱生成。

端口：3008（与主项目 Node.js 后端 3002 配合）
设计原则：
  1. 零阻塞：CPU 密集推理跑线程池，不阻塞事件循环
  2. 可观测：每步精确计时 + 结构化日志
  3. 稳健：优雅降级 + 超时保护 + 共识合并
  4. 可扩展：15 种音色/37 条内置旋律/多格式输出
"""
import base64
import io
import json
import os
import re
import time
import traceback
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass

import numpy as np
from fastapi import FastAPI, File, Form, HTTPException, UploadFile, Query
from fastapi.responses import FileResponse, JSONResponse
from fastapi.middleware.cors import CORSMiddleware

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = HERE
os.sys.path.insert(0, ROOT)

from core.config import Config
from core.paths import resource_path
from core import capture, score_sheet

app = FastAPI(title="Melody2Score 企业级转谱引擎", version="2.0.0")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

SAVE_DIR = os.path.join(HERE, "app", "exports")
os.makedirs(SAVE_DIR, exist_ok=True)

MANIFEST_PATH = resource_path("audio", "manifest.json")


# ========== 数据模型 ==========

@dataclass
class RecognitionResult:
    """识别结果结构化封装"""
    jianpu: str
    bpm: float
    key: Dict
    note_count: int
    duration_sec: float
    backend: str
    notes: List[Dict]
    confidence: float
    robust_runs: int
    robust_kept: int
    perf: Dict
    midi_sequence: List[int] = None
    source: str = ""


# ========== 核心函数 ==========

def _build_config(model_size: str = "tiny", denoise: bool = True,
                  threads: int = 0, hop: int = 0, robust: bool = True,
                  vocal_mode: bool = True) -> Config:
    cfg = Config()
    cfg.model_size = model_size or cfg.model_size
    cfg.enable_denoise = denoise
    cfg.robust = robust
    cfg.vocal_mode = vocal_mode
    if threads and threads > 0:
        cfg.intra_op_threads = threads
    if hop and hop > 0:
        cfg.hop = hop
    cfg.preferred_backend = "crepe_onnx"
    return cfg


def _recognize_array(y, sr, cfg: Config) -> Dict:
    """核心识别：委托企业级编排器 Melody2Score。"""
    from core.pipeline import Melody2Score
    return Melody2Score(cfg).recognize({"kind": "array", "y": y, "sr": sr, "cfg": cfg})


def _load_bytes_fallback(data: bytes, sr: int) -> Tuple[np.ndarray, int]:
    """字节 → (y, sr)。复用 pipeline 的加载逻辑：librosa 优先（支持
    wav/mp3/flac/ogg/m4a 等多格式），失败回退 soundfile（仅 wav/flac）。

    修复：原实现直接 soundfile.read，mp3/m4a 上传会因 libsndfile 解码
    支持缺失而 400 报错——企业级 API 必须支持主流上传格式。
    """
    from core.pipeline import load_audio_bytes
    return load_audio_bytes(data, sr)


def _midi_name(m: int) -> str:
    names = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']
    return f"{names[m % 12]}{m // 12 - 1}"


def _notes_to_midi_sequence(notes: List[Dict]) -> List[int]:
    """提取音符的 MIDI 编号序列（用于前端播放）。"""
    return [int(n["midi"]) for n in notes if "midi" in n]


def _notes_to_vexflow(notes: List[Dict], key: Dict, bpm: float,
                       time_sig: str = "4/4") -> Dict:
    """把音符转为 VexFlow 可渲染的乐谱数据。
    
    返回结构化数据，前端直接用于 VexFlow StaveNote 渲染。
    """
    tonic = key.get("tonic", "C")
    mode = key.get("mode", "major")
    beat_dur = 60.0 / max(bpm, 30) if bpm and bpm >= 30 else 0.5

    # 调号映射
    key_signatures = {
        "C": "C", "G": "G", "D": "D", "A": "A", "E": "E", "B": "B",
        "F#": "F#", "C#": "C#", "F": "F", "Bb": "Bb", "Eb": "Eb",
        "Ab": "Ab", "Db": "Db", "Gb": "Gb", "Cb": "Cb"
    }
    ks = key_signatures.get(tonic, "C")
    # 小调关系调
    relative_map = {"A": "C", "E": "G", "B": "D", "F#": "A", "C#": "E",
                    "G#": "B", "D#": "F#", "D": "F", "G": "Bb",
                    "C": "Eb", "F": "Ab", "Bb": "Db", "Eb": "Gb"}
    if mode == "minor" and tonic in relative_map:
        ks = key_signatures.get(relative_map[tonic], "C")

    vex_notes = []
    for n in notes:
        midi = int(n["midi"])
        dur = float(n.get("dur", 0.25))
        beats = max(0.125, dur / beat_dur)

        # 时值映射
        dur_map = [
            (4.0, "w"), (3.0, "h"), (2.0, "h"), (1.5, "hd"),
            (1.0, "q"), (0.75, "qd"), (0.5, "8"), (0.25, "16"), (0.125, "32")
        ]
        note_dur = "q"
        for threshold, d in dur_map:
            if beats >= threshold - 0.05:
                note_dur = d
                break

        # MIDI 转音名 + 八度
        pc = midi % 12
        octave = (midi // 12) - 1
        note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
        nn = note_names[pc]
        vex_notes.append({
            "note": f"{nn}/{octave}",
            "duration": note_dur,
            "midi": midi
        })

    return {
        "key_signature": ks,
        "time_signature": time_sig,
        "bpm": int(round(bpm)) if bpm and bpm >= 30 else 120,
        "notes": vex_notes,
        "total_beats": sum(
            max(0.125, float(n.get("dur", 0.25)) / max(beat_dur, 0.125))
            for n in notes
        )
    }


def _enrich_result(res: Dict) -> Dict:
    """丰富识别结果，添加前端所需的数据。"""
    notes = res.get("notes", [])
    key = res.get("key", {"tonic": "C", "mode": "major"})
    bpm = float(res.get("bpm", 120))
    res["midi_sequence"] = _notes_to_midi_sequence(notes)
    res["vexflow_data"] = _notes_to_vexflow(notes, key, bpm)
    return res


# ========== API 端点 ==========

@app.get("/api/melody2score/health")
def health_check():
    return {"status": "ok", "service": "melody2score", "version": "2.0.0"}


@app.get("/api/melody2score/samples")
def list_samples():
    """列出内置经典旋律样例。"""
    if not os.path.exists(MANIFEST_PATH):
        return []
    with open(MANIFEST_PATH, encoding="utf-8") as f:
        manifest = json.load(f)
    groups = {}
    for it in manifest:
        g = groups.setdefault(it["melody_index"], {
            "melody_index": it["melody_index"],
            "title_zh": it["title_zh"],
            "title_en": it["title_en"],
            "timbres": []
        })
        g["timbres"].append({
            "timbre": it["timbre"],
            "file": it["file"],
            "timbre_zh": it.get("timbre_zh", it["timbre"])
        })
    return list(groups.values())


@app.get("/api/melody2score/sample-audio")
def get_sample_audio(file: str = Query(...)):
    """获取内置样例音频文件。"""
    fpath = resource_path("audio", file)
    if not os.path.exists(fpath):
        raise HTTPException(404, f"样例音频不存在: {file}")
    return FileResponse(fpath, media_type="audio/wav",
                        filename=os.path.basename(fpath))


@app.post("/api/melody2score/recognize")
async def recognize(
    file: UploadFile = File(...),
    model_size: str = Form("tiny"),
    denoise: bool = Form(True),
    robust: bool = Form(True),
    vocal_mode: bool = Form(True),
    hop: int = Form(0)
):
    """上传音频文件 → 识别为结构化歌谱 JSON。"""
    import anyio
    cfg = _build_config(model_size, denoise, 0, hop, robust, vocal_mode)
    data = await file.read()
    try:
        y, sr = _load_bytes_fallback(data, cfg.sr)
    except Exception as e:
        raise HTTPException(400, f"音频解码失败: {e}")
    try:
        res = await anyio.to_thread.run_sync(_recognize_array, y, sr, cfg)
    except Exception as e:
        traceback.print_exc()
        raise HTTPException(500, f"识别失败: {e}")
    res = _enrich_result(res)
    res["source"] = file.filename or "上传文件"
    return JSONResponse(res)


@app.post("/api/melody2score/recognize-sample")
async def recognize_sample(
    name: str = Form(...),
    model_size: str = Form("tiny"),
    denoise: bool = Form(True),
    robust: bool = Form(True),
    vocal_mode: bool = Form(True)
):
    """识别内置样例音频。"""
    import anyio
    cfg = _build_config(model_size, denoise, 0, 0, robust, vocal_mode)
    if not os.path.exists(MANIFEST_PATH):
        raise HTTPException(404, "未找到 audio/manifest.json")
    with open(MANIFEST_PATH, encoding="utf-8") as f:
        manifest = json.load(f)
    item = next((it for it in manifest if it["file"].endswith(name) or it["title_zh"] == name), None)
    if not item:
        raise HTTPException(404, f"样例不存在: {name}")
    y = capture.load_audio(resource_path(item["file"]), cfg.sr)
    try:
        res = await anyio.to_thread.run_sync(_recognize_array, y, cfg.sr, cfg)
    except Exception as e:
        traceback.print_exc()
        raise HTTPException(500, f"识别失败: {e}")
    res = _enrich_result(res)
    res["source"] = item["file"]
    res["sample_info"] = {
        "title_zh": item["title_zh"],
        "title_en": item["title_en"],
        "timbre": item["timbre"]
    }
    return JSONResponse(res)


@app.post("/api/melody2score/recognize-record")
async def recognize_record(
    audio_b64: str = Form(...),
    model_size: str = Form("tiny"),
    denoise: bool = Form(True),
    robust: bool = Form(True),
    vocal_mode: bool = Form(True)
):
    """浏览器录音(base64 wav) → 歌谱 JSON。"""
    import anyio
    cfg = _build_config(model_size, denoise, 0, 0, robust, vocal_mode)
    try:
        raw = base64.b64decode(audio_b64)
        y, sr = _load_bytes_fallback(raw, cfg.sr)
    except Exception as e:
        raise HTTPException(400, f"录音解码失败: {e}")
    try:
        res = await anyio.to_thread.run_sync(_recognize_array, y, sr, cfg)
    except Exception as e:
        traceback.print_exc()
        raise HTTPException(500, f"识别失败: {e}")
    res = _enrich_result(res)
    res["source"] = "现场录音"
    return JSONResponse(res)


@app.post("/api/melody2score/export-sheet")
async def export_sheet(payload: dict):
    """把识别结果导出为标准歌谱图片（PNG/PDF/SVG）。"""
    try:
        res = payload.get("result", {})
        title = payload.get("title", "未命名旋律") or "未命名旋律"
        fmt = (payload.get("format", "png") or "png").lower()
        if fmt not in ("png", "pdf", "svg"):
            raise HTTPException(400, "format 仅支持 png/pdf/svg")

        safe = re.sub(r"[^\w一-鿿-]", "_", title)[:40]
        ts = time.strftime("%Y%m%d_%H%M%S")
        fname = f"{safe or 'melody'}_标准歌谱_{ts}.{fmt}"
        fpath = os.path.join(SAVE_DIR, fname)

        score_sheet.export_score(
            notes=res.get("notes", []),
            key=res.get("key", {"tonic": "C", "mode": "major"}),
            bpm=float(res.get("bpm", 120)),
            output_path=fpath,
            title=title,
        )
        return JSONResponse({"file": fname, "path": fpath})
    except HTTPException:
        raise
    except Exception as e:
        traceback.print_exc()
        raise HTTPException(500, f"导出歌谱失败: {e}")


@app.post("/api/melody2score/save-report")
async def save_report(payload: dict):
    """把识别结果保存为企业级 Markdown 报告。"""
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
        confidence = res.get("confidence", 0)
        vexflow = res.get("vexflow_data", {})

        ts = time.strftime("%Y%m%d_%H%M%S")
        safe = re.sub(r"[^\w一-鿿-]", "_", title)[:40]
        fname = f"{safe or 'melody'}_{ts}.md"
        fpath = os.path.join(SAVE_DIR, fname)

        lines = []
        lines.append(f"# 旋律转谱报告：{title}\n")
        lines.append(f"> 生成时间：{time.strftime('%Y-%m-%d %H:%M:%S')}  |  ")
        lines.append(f"来源：{source}  |  音高后端：{backend}\n")
        lines.append("\n## 一、识别概要\n")
        lines.append("| 指标 | 值 |")
        lines.append("|------|-----|")
        lines.append(f"| 调式 | {key.get('tonic','?')} {key.get('mode','?')} |")
        lines.append(f"| 速度(BPM) | {bpm} |")
        lines.append(f"| 音符数 | {len(notes)} |")
        lines.append(f"| 时长(秒) | {res.get('duration_sec', 0)} |")
        lines.append(f"| 置信度 | {confidence} |")
        lines.append(f"| 稳健识别 | {res.get('robust_runs', 1)} 次 |")
        lines.append(f"| 预处理耗时 | {perf.get('preprocess_ms', 0)} ms |")
        lines.append(f"| 音高检测耗时 | {perf.get('pitch_ms', 0)} ms |")
        lines.append(f"| 解析耗时 | {perf.get('parse_ms', 0)} ms |")

        lines.append("\n## 二、简谱\n")
        lines.append("```text")
        lines.append(jianpu)
        lines.append("```\n")

        lines.append("## 三、音符明细\n")
        lines.append("| # | MIDI | 音名 | 起始(s) | 结束(s) | 时长(s) |")
        lines.append("|---|------|------|---------|---------|---------|")
        for i, n in enumerate(notes, 1):
            lines.append(f"| {i} | {n['midi']} | {n.get('name', '')} | "
                         f"{n.get('start', 0)} | {n.get('end', 0)} | {n.get('dur', 0)} |")

        lines.append("\n## 四、算法说明\n")
        lines.append(_algorithm_doc())

        with open(fpath, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))

        return JSONResponse({"file": fname, "path": fpath})
    except Exception as e:
        traceback.print_exc()
        raise HTTPException(500, f"保存报告失败: {e}")


@app.get("/api/melody2score/download/{fname:path}")
def download(fname: str):
    fpath = os.path.join(SAVE_DIR, fname)
    if not os.path.exists(fpath):
        raise HTTPException(404)
    ext = os.path.splitext(fname)[1].lower()
    mt = {
        "png": "image/png", "pdf": "application/pdf", "svg": "image/svg+xml",
        "md": "text/markdown", "xml": "application/xml", "musicxml": "application/xml",
        "mid": "audio/midi", "midi": "audio/midi"
    }.get(ext, "application/octet-stream")
    return FileResponse(fpath, filename=fname, media_type=mt)


@app.get("/api/melody2score/status")
def service_status():
    """服务状态与统计。"""
    from core.pipeline import _RESULT_CACHE
    exports = os.listdir(SAVE_DIR) if os.path.exists(SAVE_DIR) else []
    return {
        "status": "running",
        "version": "2.0.0",
        "cache_size": len(_RESULT_CACHE._d) if hasattr(_RESULT_CACHE, '_d') else 0,
        "export_count": len(exports),
        "export_dir": SAVE_DIR,
        "samples_count": len(list_samples()) if os.path.exists(MANIFEST_PATH) else 0
    }


@app.post("/api/melody2score/batch-recognize")
async def batch_recognize(files: List[UploadFile] = File(...)):
    """批量识别：同时处理多个音频文件。"""
    import anyio
    cfg = _build_config()
    results = []
    for file in files:
        try:
            data = await file.read()
            y, sr = _load_bytes_fallback(data, cfg.sr)
            res = await anyio.to_thread.run_sync(_recognize_array, y, sr, cfg)
            res = _enrich_result(res)
            res["source"] = file.filename or "未知"
            results.append({"file": file.filename, "success": True, "result": res})
        except Exception as e:
            results.append({"file": file.filename, "success": False, "error": str(e)})
    return JSONResponse({"total": len(files), "results": results})


# ========== 工具函数 ==========

def _algorithm_doc() -> str:
    return (
        "本结果由 Melody2Score v2.0 企业级引擎生成，全维分析流水线：\n\n"
        "1. **采集层**：librosa/soundfile 加载，重采样至 16kHz 单声道，峰值归一化。\n"
        "2. **预处理层**：去直流偏移 + 峰值归一化 + 谱减降噪（以开头 0.1s 静音段估计噪声底）。\n"
        "3. **音高检测层**：可插拔后端（首选 crepe_onnx tiny，降级 pyin→torchcrepe），"
        "输出 [{t, freq, conf}]，低于置信度阈值(0.3)判为无声。\n"
        "4. **稳健共识层**：3 次独立识别（扰动置信阈值），音符级时间重叠共识合并，"
        "抑制单次偶发假音高与漏音。\n"
        "5. **音乐解析层**：\n"
        "   - midi 轮廓中值滤波(win=5~7)消除颤音与帧间抖动；\n"
        "   - 半音量化后按相同音高分段；\n"
        "   - 短段(<min_note_dur)就近合并到音高最近的长邻居；\n"
        "   - VAD 过滤呼吸/停顿/气声假音高；\n"
        "   - BPM 用音符时长众数反推（优先）+ librosa beat_track 降级；\n"
        "   - 调式用 Krumhansl-Schmuckler 模板相关（首尾音加分权重）。\n"
        "6. **歌谱生成层**：简谱（首调数字 1-7 + 八度点/延音线）+ 五线谱(VexFlow) + "
        "MusicXML + 标准歌谱图片(LilyPond)。\n\n"
        "**企业级特性**：首选项 crepe_onnx tiny（CPU 友好、可复现）；稳健重识别 3 次共识；"
        "超时保护 60s 防卡死；VAD 人声/乐器自适应；可插拔后端降级。"
    )


# ========== 启动入口 ==========

if __name__ == "__main__":
    import uvicorn
    port = int(os.environ.get("MELODY2SCORE_PORT", "3008"))
    print(f"[Melody2Score] 企业级转谱引擎启动于 http://0.0.0.0:{port}")
    print(f"[Melody2Score] API 文档 http://0.0.0.0:{port}/docs")
    uvicorn.run(app, host="0.0.0.0", port=port, reload=False, log_level="info")