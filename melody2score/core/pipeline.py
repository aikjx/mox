# -*- coding: utf-8 -*-
"""企业级旋律识别编排器（统一收口 gui / webui / demo 三处流水线）。

职责：
  - 规范化输入：文件字节 / 录音字节 / 样例名 → 统一 (y, sr)。
  - 稳定化流水线：预处理 → 音高检测(首选 crepe_onnx + 优雅降级 + 超时)
    → 可选 VAD → 音符分割 → 稳健重识别共识 → BPM/调式 → 出简谱/五线谱/量化轮廓。
  - 稳健重识别：同一音频跑 N 次（扰动置信阈值），对音符做时间重叠共识合并，
    抑制单次偶发假音高与漏音，显著提升稳定性（gui 既有逻辑提炼至此）。
  - 可观测：完整分段计时、所用后端、识别次数、共识保留率、置信度。
  - 确定性：不引入随机源，同一输入相同输出。

输出 dict 与现有 GUI/WebUI 契约一致，便于零改动接入。
"""
import io
import os
import time
import traceback
from typing import Dict, List, Optional, Tuple

import numpy as np
import soundfile as sf

from core.config import Config
from core import capture, preprocess, pitch, analysis, score, vad, score_sheet

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

_MIDI_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']


def midi_name(m: int) -> str:
    return f"{_MIDI_NAMES[m % 12]}{m // 12 - 1}"


def load_audio_bytes(data: bytes, sr: int):
    """字节 → (y, sr)。优先 librosa（支持多格式），失败回退 soundfile。"""
    import librosa
    try:
        y, _ = librosa.load(io.BytesIO(data), sr=sr, mono=True)
        return np.asarray(y, dtype=np.float32), sr
    except Exception:
        y, _ = sf.read(io.BytesIO(data), samplerate=sr, dtype="float32", always_2d=False)
        return np.asarray(y, dtype=np.float32), sr


def _load_source(source: Dict, cfg: Config) -> Tuple[np.ndarray, int]:
    kind = source["kind"]
    if kind in ("file", "record"):
        return load_audio_bytes(source["data"], cfg.sr)
    if kind == "sample":
        y = capture.load_audio(os.path.join(ROOT, source["name"]), cfg.sr)
        return np.asarray(y, dtype=np.float32), cfg.sr
    if kind == "array":
        # 已解码的 (y, sr)：直接复用，避免 WebUI 重复 wav 编解码往返（性能/延迟）
        return np.asarray(source["y"], dtype=np.float32), int(source["sr"])
    raise ValueError(f"未知音源类型: {kind}")


def _conf(notes) -> float:
    """置信度：基于音符平均时长与数量（越长越多越可信）。"""
    if not notes:
        return 0.0
    durs = [n["end"] - n["start"] for n in notes]
    avg = float(np.mean(durs))
    return float(min(1.0, 0.4 + avg / 0.3))


def _consensus(runs: List[List[Dict]], cfg: Config) -> Tuple[List[Dict], Dict]:
    """音符级共识合并（时间重叠聚簇，多数/长音保留）。"""
    all_notes: List[Dict] = []
    for notes in runs:
        all_notes.extend(notes)
    all_notes.sort(key=lambda n: n["start"])

    clusters: List[Dict] = []
    for n in all_notes:
        placed = False
        for c in clusters:
            overlap = False
            for m in c["members"]:
                ov = min(n["end"], m["end"]) - max(n["start"], m["start"])
                short = min(n["end"] - n["start"], m["end"] - m["start"])
                if short > 0 and ov > 0.6 * short:
                    overlap = True
                    break
            if overlap:
                c["members"].append(n)
                placed = True
                break
        if not placed:
            clusters.append({"members": [n]})

    merged: List[Dict] = []
    kept = 0
    conf_sum = 0.0
    for c in clusters:
        members = c["members"]
        counts: Dict[int, int] = {}
        for m in members:
            counts[m["midi"]] = counts.get(m["midi"], 0) + 1
        best_midi = max(counts.items(), key=lambda kv: (kv[1], -kv[1]))[0]
        longest = max(members, key=lambda m: m["end"] - m["start"])
        start = min(m["start"] for m in members)
        end = max(m["end"] for m in members)
        # 仅在 ≥2 次识别都出现，或单次但长音，才保留
        if counts[best_midi] >= 2 or (end - start) > 2 * cfg.min_note_dur:
            merged.append({"midi": best_midi, "start": start, "end": end})
            kept += 1
            conf_sum += counts[best_midi] / len(runs)

    merged.sort(key=lambda n: n["start"])
    return merged, {"kept": kept, "confidence": conf_sum / max(1, len(merged))}


def _segment_once(y, sr, cfg, k: int, det: "pitch.PitchDetector"):
    """单次识别：扰动置信阈值 → 检测 → 可选 VAD → 分割音符。"""
    eff = Config()
    eff.conf_thresh = max(0.05, cfg.conf_thresh - 0.06 + 0.04 * k)
    eff.hop = cfg.hop
    eff.min_note_dur = cfg.min_note_dur
    eff.median_win = cfg.median_win
    eff.vocal_mode = cfg.vocal_mode

    pts = det.detect(y, sr)

    vad_mask = None
    if cfg.vocal_mode and cfg.enable_vad:
        vad_mask = vad.voice_activity_mask(
            y, sr, energy_thresh=cfg.vad_energy_thresh,
            centroid_min=cfg.vad_centroid_min, centroid_max=cfg.vad_centroid_max,
            flatness_max=cfg.vad_flatness_max, hop_ms=eff.hop,
            min_voiced_ms=cfg.min_voiced_ms)

    notes = analysis.segment_notes(
        pts, cfg.min_note_dur, cfg.median_win,
        vocal_mode=cfg.vocal_mode, vad_mask=vad_mask)
    return notes, pts


class Melody2Score:
    """统一编排器。纯逻辑，无 UI 依赖，可同步调用（GUI/WebUI 自行丢线程）。"""

    def __init__(self, cfg: Optional[Config] = None):
        self.cfg = cfg or Config()

    def recognize(self, source: Dict, progress_cb=None) -> Dict:
        """source: {kind:'file'|'sample'|'record', data/bytes/name, cfg?, source?}

        progress_cb(stage:str, msg:str, fraction:float) 可选，用于 CLI/WebUI
        实时进度反馈，避免长音频下「假死」观感。fraction 取值 0~1。
        """
        cfg = self.cfg
        try:
            if progress_cb:
                progress_cb("load", "载入音频…", 0.02)
            y, sr = _load_source(source, cfg)

            if progress_cb:
                progress_cb("preprocess", "预处理/降噪…", 0.12)
            t0 = time.time()
            y = preprocess.preprocess(y, sr, cfg.enable_denoise)
            t_pre = time.time() - t0

            n_runs = 4 if cfg.robust else 1
            runs: List[List[Dict]] = []
            t_pitch_total = 0.0
            used_backend = "auto"
            last_pts: List[Dict] = []

            # 复用同一个 PitchDetector 实例跨多次 run，避免 robust 模式反复
            # 加载 ONNX/torch 模型（单次加载即数百 ms~数 s，是卡顿主因之一）
            det = pitch.PitchDetector(
                model_size=cfg.model_size, conf_thresh=cfg.conf_thresh,
                hop=cfg.hop, intra_op_threads=cfg.intra_op_threads,
                backend="auto", sr=sr,
                fmin=getattr(cfg, "fmin", 50.0), fmax=getattr(cfg, "fmax", 1100.0),
                preferred_backend=cfg.preferred_backend,
                inference_timeout=cfg.inference_timeout)
            for k in range(n_runs):
                if progress_cb:
                    progress_cb("pitch", f"音高检测 ({k+1}/{n_runs})…",
                                0.12 + 0.72 * (k + 1) / n_runs)
                t0 = time.time()
                notes, pts = _segment_once(y, sr, cfg, k, det)
                t_pitch_total += time.time() - t0
                used_backend = det.used_backend
                last_pts = pts
                runs.append(notes)

            t_pitch = t_pitch_total

            if n_runs > 1:
                notes, merge_info = _consensus(runs, cfg)
            else:
                notes = runs[0]
                merge_info = None

            t0 = time.time()
            if progress_cb:
                progress_cb("parse", "节拍/调式/音符解析…", 0.92)
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
                "name": midi_name(int(n["midi"])),
            } for n in notes]

            return {
                "jianpu": jianpu, "bpm": round(float(bpm), 1),
                "key": {"tonic": key_name[0], "mode": key_name[1]},
                "note_count": len(notes), "duration_sec": round(total_dur, 2),
                "backend": used_backend, "notes": notes_out,
                "confidence": round(float(merge_info["confidence"]) if merge_info else _conf(runs[0]), 2),
                "robust_runs": n_runs,
                "robust_kept": merge_info["kept"] if merge_info else len(runs[0]),
                "perf": {"preprocess_ms": round(t_pre * 1000, 1),
                         "pitch_ms": round(t_pitch * 1000, 1),
                         "parse_ms": round(t_parse * 1000, 1),
                         "pitch_frames": len(last_pts)},
                "source": source.get("source", ""),
            }
        except Exception as e:
            traceback.print_exc()
            raise

    # ---------- 便捷入口（面向 CLI / demo） ----------
    def run(self, audio_path: Optional[str] = None, record_seconds: int = 0,
            out_xml: Optional[str] = None, ms_score: Optional[str] = None,
            out_sheet: Optional[str] = None,
            progress_cb=None) -> Dict:
        """一键识别：文件 / 现场录音二选一，可选导出 musicxml / 标准歌谱图片。

        返回与 recognize 一致的结构化结果，并默认附带 score_image_path（如可用）。
        progress_cb 可选，用于实时进度。
        """
        if record_seconds and record_seconds > 0:
            import sounddevice as sd  # 延迟导入，避免无音频设备环境报错
            y = capture.record(seconds=record_seconds, sr=self.cfg.sr)
            source = {"kind": "record", "data": _dump_wav(y, self.cfg.sr),
                      "cfg": self.cfg, "source": f"现场录音{record_seconds}s"}
        elif audio_path:
            with open(audio_path, "rb") as f:
                data = f.read()
            source = {"kind": "file", "data": data, "cfg": self.cfg,
                      "source": os.path.basename(audio_path)}
        else:
            raise ValueError("需提供 audio_path 或 record_seconds")

        res = self.recognize(source, progress_cb=progress_cb)

        if out_xml:
            self._export_xml(res, out_xml, ms_score)

        # 默认生成规范歌谱图片（PNG），置于 exports/ 目录
        sheet_path = out_sheet or self._default_sheet_path(audio_path or "record")
        try:
            generated = score_sheet.export_score(
                notes=res["notes"],
                key=res["key"],
                bpm=res["bpm"],
                output_path=sheet_path,
                title=os.path.splitext(os.path.basename(audio_path or "未命名旋律"))[0],
            )
            res["score_image_path"] = generated
        except Exception as e:
            # 图片生成失败不应阻断主流程（如服务器无字体）
            res["score_image_path"] = None
            res["score_image_error"] = str(e)

        return res

    def _default_sheet_path(self, audio_path: str) -> str:
        base = os.path.splitext(os.path.basename(audio_path or "record"))[0] or "未命名旋律"
        exports_dir = os.path.join(ROOT, "app", "exports")
        os.makedirs(exports_dir, exist_ok=True)
        ts = time.strftime("%Y%m%d_%H%M%S")
        return os.path.join(exports_dir, f"{base}_标准歌谱_{ts}.png")

    def _export_xml(self, res: Dict, out_xml: str, ms_score: Optional[str]):
        try:
            score.to_musicxml(
                res["notes"],
                float(res["bpm"]),
                (res["key"]["tonic"], res["key"]["mode"]),
                fp=out_xml,
            )
        except Exception as e:
            print(f"[warn] musicxml 导出失败: {e}")

    @staticmethod
    def print_summary(res: Dict) -> None:
        key = res.get("key", {})
        perf = res.get("perf", {})
        print("\n================ 旋律转谱结果 ================")
        print(f"  音高后端     : {res.get('backend')}")
        print(f"  调式         : {key.get('tonic')} {key.get('mode')}")
        print(f"  速度 BPM     : {res.get('bpm')}")
        print(f"  音符数       : {res.get('note_count')}")
        print(f"  时长(秒)     : {res.get('duration_sec')}")
        print(f"  置信度       : {res.get('confidence')}")
        print(f"  稳健识别次数 : {res.get('robust_runs')}  (共识保留 {res.get('robust_kept')})")
        print(f"  耗时(ms)     : 预处理 {perf.get('preprocess_ms')} / "
              f"音高 {perf.get('pitch_ms')} / 解析 {perf.get('parse_ms')} "
              f"(帧 {perf.get('pitch_frames')})")
        print("  ---------- 简谱 ----------")
        print(res.get("jianpu"))
        print("============================================\n")


def _dump_wav(y, sr) -> bytes:
    """把 (y, sr) 打包为 wav 字节（供 record 源统一加载）。"""
    import soundfile as sf
    buf = io.BytesIO()
    sf.write(buf, np.asarray(y, dtype=np.float32), sr, format="WAV")
    return buf.getvalue()
