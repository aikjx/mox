# -*- coding: utf-8 -*-
"""音乐解析层：BPM/节拍、调式识别、音符分割，以及颤音/滑音毛刺过滤。"""
from typing import Dict, List, Optional, Tuple

import numpy as np
import librosa


def freq2midi(freq: float):
    if freq <= 0:
        return None
    return int(round(69 + 12 * np.log2(freq / 440.0)))


def _median_filter(x: np.ndarray, win: int) -> np.ndarray:
    out = x.copy()
    for i in range(len(x)):
        lo = max(0, i - win // 2)
        hi = min(len(x), i + win // 2 + 1)
        seg = x[lo:hi]
        if len(seg):
            out[i] = np.median(seg)
    return out


def segment_notes(pitch_points: List[Dict], min_note_dur: float = 0.1,
                  median_win: int = 5, vocal_mode: bool = False,
                  vad_mask=None) -> List[Dict]:
    """把连续 pitch 点切分为音符，并过滤颤音/滑音毛刺。

    流程：
      0)（可选）VAD 掩码过滤 —— 无人声段不产出音符（人声模式核心）；
      1) midi 轮廓中值滤波 —— 去掉颤音与帧间抖动（人声模式窗更大）；
      2) 半音量化后按相同音高分段；
      3) 把 < min_note_dur 的短段（滑音尾音/颤音过冲）合并到音高最近的相邻音符；
      4) 过滤仍过短的音符。
    """
    if not pitch_points:
        return []

    # 0) VAD：拿掉呼吸/停顿/气声假音高
    if vad_mask is not None and len(vad_mask) > 0:
        from .vad import apply_vad
        pitch_points = apply_vad(pitch_points, vad_mask, hop_ms=10,
                                 sr=16000)
        if not pitch_points:
            return []

    # 人声模式：颤音更明显，中值窗不足时自动加窗
    if vocal_mode:
        median_win = max(median_win, 7)

    mids = np.array([freq2midi(p["freq"]) for p in pitch_points], dtype=float)
    mids = _median_filter(mids, median_win)

    # 半音量化 + 初分段
    raw: List[Dict] = []
    cur = None
    for p, m in zip(pitch_points, mids):
        if np.isnan(m):
            cur = None
            continue
        mi = int(round(m))
        if cur is None:
            cur = {"midi": mi, "start": p["t"], "end": p["t"]}
        elif cur["midi"] == mi:
            cur["end"] = p["t"]
        else:
            raw.append(cur)
            cur = {"midi": mi, "start": p["t"], "end": p["t"]}
    if cur is not None:
        raw.append(cur)

    # 合并短段到音高最近的邻居（处理滑音/颤音毛刺）
    raw = _merge_short(raw, min_note_dur)
    # 过滤过短音符
    notes = [n for n in raw if (n["end"] - n["start"]) > min_note_dur]
    return notes


def _merge_short(notes: List[Dict], min_note_dur: float) -> List[Dict]:
    out = list(notes)
    changed = True
    while changed:
        changed = False
        for i, n in enumerate(out):
            if (n["end"] - n["start"]) <= min_note_dur:
                best, best_d = None, 1e9
                if i > 0:
                    d = abs(out[i - 1]["midi"] - n["midi"])
                    if d < best_d:
                        best_d, best = d, i - 1
                if i < len(out) - 1:
                    d = abs(out[i + 1]["midi"] - n["midi"])
                    if d < best_d:
                        best_d, best = d, i + 1
                if best is None:
                    out.pop(i)
                else:
                    nb = out[best]
                    nb["start"] = min(nb["start"], n["start"])
                    nb["end"] = max(nb["end"], n["end"])
                    out.pop(i)
                changed = True
                break
    return out


def detect_bpm(y: np.ndarray, sr: int = 16000, fallback: float = 120.0) -> float:
    try:
        tempo, _ = librosa.beat.beat_track(y=y, sr=sr, hop_length=512)
        return float(np.atleast_1d(tempo)[0])
    except Exception:
        return fallback


def estimate_key(y: np.ndarray, sr: int = 16000,
                 notes: Optional[List[Dict]] = None) -> Tuple[str, str]:
    """Krumhansl-Schmuckler 调式识别（12 大调 / 12 小调）。

    优化（精确 + 高效）：
      - 优先用「音符 MIDI 轮廓」统计音级分布（O(音符数)，免 CQT 重计算）；
      - 仅当无音符时回退到 chroma_stft 对降采样信号做轻量估计（远快于 chroma_cqt）。
    返回 (tonic, mode)。
    """
    major = np.array([6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88])
    minor = np.array([6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17])
    names = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']

    prof = None
    if notes:
        pc = np.zeros(12, dtype=float)
        for i, n in enumerate(notes):
            w = max(0.05, float(n.get("end", 0) - n.get("start", 0)))
            pc[int(round(n["midi"])) % 12] += w   # 按时长加权，主音/属音权重更高
        # 旋律学先验：起始音与终止音强烈倾向主音（tonic）→ 加倍权重，
        # 显著纠正「属音(如 G)被 K-S 误判为主音」的常见错误（如小星星）。
        if notes:
            pc[int(round(notes[0]["midi"])) % 12] += 1.0
            pc[int(round(notes[-1]["midi"])) % 12] += 0.8
        if pc.sum() > 0:
            prof = pc / (np.linalg.norm(pc) + 1e-9)

    if prof is None:
        # 兜底：对 4kHz 降采样信号做 chroma_stft（比 chroma_cqt 快一个数量级）
        try:
            yd = librosa.resample(y, orig_sr=sr, target_sr=4000) if sr > 4000 else y
            chroma = librosa.feature.chroma_stft(y=yd, sr=4000 if sr > 4000 else sr,
                                                 hop_length=2048, n_fft=2048)
            p = chroma.mean(axis=1)
            prof = p / (np.linalg.norm(p) + 1e-9)
        except Exception:
            return ('C', 'major')

    best_v, best = -1.0, ('C', 'major')
    for i in range(12):
        shifted = np.roll(prof, -i)
        vmaj = float(np.dot(shifted, major / np.linalg.norm(major)))
        vmin = float(np.dot(shifted, minor / np.linalg.norm(minor)))
        if vmaj > best_v:
            best_v, best = vmaj, (names[i], 'major')
        if vmin > best_v:
            best_v, best = vmin, (names[i], 'minor')
    return best
