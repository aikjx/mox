# -*- coding: utf-8 -*-
"""音乐解析层：BPM/节拍、调式识别、音符分割，以及颤音/滑音毛刺过滤。"""
from typing import Dict, List, Tuple

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
                  median_win: int = 5) -> List[Dict]:
    """把连续 pitch 点切分为音符，并过滤颤音/滑音毛刺。

    流程：
      1) midi 轮廓中值滤波 —— 去掉颤音与帧间抖动；
      2) 半音量化后按相同音高分段；
      3) 把 < min_note_dur 的短段（滑音尾音/颤音过冲）合并到音高最近的相邻音符；
      4) 过滤仍过短的音符。
    """
    if not pitch_points:
        return []

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


def estimate_key(y: np.ndarray, sr: int = 16000) -> Tuple[str, str]:
    """Krumhansl-Schmuckler 调式模板，对 12 大调 / 12 小调做相关。"""
    chroma = librosa.feature.chroma_cqt(y=y, sr=sr)
    prof = chroma.mean(axis=1)
    prof = prof / (np.linalg.norm(prof) + 1e-9)
    major = np.array([6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88])
    minor = np.array([6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17])
    names = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']
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
