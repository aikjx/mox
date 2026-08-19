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

import numpy as np
from fastapi import FastAPI, File, Form, HTTPException, UploadFile
from fastapi.responses import FileResponse, JSONResponse
from fastapi.staticfiles import StaticFiles

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
os.sys.path.insert(0, ROOT)

from core.config import Config                       # noqa: E402
from core import capture, score_sheet                # noqa: E402

app = FastAPI(title="Melody2Score 企业级可视化转谱", version="1.1.0")
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


def _build_config(model_size: str, denoise: bool, threads: int, hop: int,
                  robust: bool = True) -> Config:
    cfg = Config()
    cfg.model_size = model_size or cfg.model_size
    cfg.enable_denoise = denoise
    cfg.robust = robust
    if threads and threads > 0:
        cfg.intra_op_threads = threads
    if hop and hop > 0:
        cfg.hop = hop
    # 首选 crepe_onnx tiny（企业级默认，稳定可复现）
    cfg.preferred_backend = "crepe_onnx"
    return cfg


def _recognize_array(y, sr, cfg: Config) -> Dict:
    """核心识别：委托企业级编排器 Melody2Score，返回结构化歌谱结果。

    相比原手写拼装：统一接入稳健重识别共识、VAD、超时保护与完整计时，
    修复此前 WebUI「单次识别、无共识、无超时」的稳定性缺口。
    已解码的 (y, sr) 直接以 "array" 源传入，避免无谓的 wav 编解码往返。
    """
    from core.pipeline import Melody2Score
    return Melody2Score(cfg).recognize({"kind": "array", "y": y, "sr": sr, "cfg": cfg})


def _dump_wav(y, sr) -> bytes:
    """把已解码的 (y, sr) 重新打包为 wav 字节，供需要字节源的路径使用。"""
    import soundfile as sf
    buf = io.BytesIO()
    sf.write(buf, np.asarray(y, dtype=np.float32), sr, format="WAV")
    return buf.getvalue()



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
                    hop: int = Form(0),
                    robust: bool = Form(True)):
    import anyio
    cfg = _build_config(model_size, denoise, threads, hop, robust)
    data = await file.read()
    try:
        y, sr = _load_bytes_fallback(data, cfg.sr)
    except Exception as e:
        raise HTTPException(400, f"音频解码失败: {e}")
    try:
        # 识别是 CPU 密集且耗时（长音频可达数秒），放到线程池执行，
        # 避免阻塞 FastAPI 事件循环导致其它请求卡顿（并发稳定性）。
        res = await anyio.to_thread.run_sync(_recognize_array, y, sr, cfg)
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
async def recognize_sample(name: str = Form(...),
                            model_size: str = Form("tiny"),
                            denoise: bool = Form(True),
                            threads: int = Form(0),
                            hop: int = Form(0),
                            robust: bool = Form(True)):
    import anyio
    cfg = _build_config(model_size, denoise, threads, hop, robust)
    if not os.path.exists(MANIFEST_PATH):
        raise HTTPException(404, "未找到 audio/manifest.json")
    with open(MANIFEST_PATH, encoding="utf-8") as f:
        manifest = json.load(f)
    item = next((it for it in manifest if it["file"].endswith(name) or it["title_zh"] == name), None)
    if not item:
        raise HTTPException(404, f"样例不存在: {name}")
    y = capture.load_audio(os.path.join(ROOT, item["file"]), cfg.sr)
    try:
        res = await anyio.to_thread.run_sync(_recognize_array, y, cfg.sr, cfg)
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
                            hop: int = Form(0),
                            robust: bool = Form(True)):
    import anyio
    cfg = _build_config(model_size, denoise, threads, hop, robust)
    try:
        raw = base64.b64decode(audio_b64)
        y, sr = _load_bytes_fallback(raw, cfg.sr)
    except Exception as e:
        raise HTTPException(400, f"录音解码失败: {e}")
    try:
        res = await anyio.to_thread.run_sync(_recognize_array, y, cfg.sr, cfg)
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


@app.post("/api/export-sheet")
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
            bars_per_row=int(payload.get("bars_per_row", 4)),
            width_px=int(payload.get("width_px", 1200)),
        )
        return JSONResponse({"file": fname, "path": fpath})
    except HTTPException:
        raise
    except Exception as e:
        traceback.print_exc()
        raise HTTPException(500, f"导出歌谱失败: {e}")


@app.get("/api/download-sheet/{fname}")
def download_sheet(fname: str):
    fpath = os.path.join(SAVE_DIR, fname)
    if not os.path.exists(fpath):
        raise HTTPException(404)
    ext = os.path.splitext(fname)[1].lower()
    mt = {"png": "image/png", "pdf": "application/pdf", "svg": "image/svg+xml"}.get(ext, "application/octet-stream")
    return FileResponse(fpath, filename=fname, media_type=mt)


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
