# -*- coding: utf-8 -*-
"""人声活动检测（VAD）：区分「有人唱」与「无人声（停顿/呼吸/气声/环境噪）」。

为什么需要 VAD（人唱歌场景特有的难点）：
  - 唱歌有大量气声、呼吸、辅音、转音间隙，F0 检测器（CREPE/pyin）在这些段仍可能
    给出高 conf 的假音高（尤其 pyin 在噪声上也会"硬解"出一个频率）。
  - 仅靠 pitch.conf_thresh 会把呼吸/停顿误判成音符，污染简谱。
  - 人声的谐波结构明显（谱质心集中在低中频、谱平坦度低），与噪声/气声不同。

方法（轻量、无额外模型，CPU 友好）：
  1) 以短时帧计算能量（RMS）—— 低于能量门限判定为静音/远场；
  2) 计算谱质心 + 谱平坦度 —— 人声谐波清晰（质心不过高、平坦度低），
     纯噪声/气声平坦度高；
  3) 对「能量 & 谱质心 & 平坦度」三条件做与运算得到有声帧，再做时间平滑
     （去毛刺 + 最小有声段长度），避免单帧抖动。
输出：与输入 pitch 帧对齐的 0/1 掩码，供 segment_notes 使用。
"""
from typing import List, Dict, Optional

import numpy as np
import librosa


def voice_activity_mask(y: np.ndarray, sr: int = 16000,
                        energy_thresh: float = 0.008,
                        centroid_min: float = 200.0,
                        centroid_max: float = 3500.0,
                        flatness_max: float = 0.25,
                        hop_ms: int = 10,
                        min_voiced_ms: int = 80,
                        frame_ms: int = 25) -> np.ndarray:
    """返回时间帧（与后续 pitch 帧对齐）的 0/1 有声掩码。

    帧采用**起点对齐**（center=False）：第 i 帧对应样本 i*hop，时间 i*hop/sr，
    与 CREPE/pyin 的等间隔时间戳（t = frame*hop/1000）严格对齐，便于按
    idx=round(t/hop) 精确映射。帧数取 ceil(len/hop) 与 pitch 后端一致。
    """
    hop = max(1, int(round(sr * hop_ms / 1000.0)))
    frame = max(1, int(round(sr * frame_ms / 1000.0)))
    if len(y) < frame:
        return np.array([1], dtype=np.int8)  # 极短音频直接当作有声

    # 帧数必须与下游 pitch 后端（CREPE/pyin 的等间隔时间戳）一致：
    # 它们产生约 ceil(len/hop) 帧，而非 floor。沿用 floor 会让末尾若干帧
    # 在 apply_vad 中取 idx>=len(mask) 被静默丢弃 → 整段结尾音符丢失。
    n_frames = int(np.ceil(len(y) / hop))
    if n_frames <= 0:
        return np.array([1], dtype=np.int8)

    # center=False：帧起点对齐，确保与 CREPE 的 t=i*hop/1000 一致
    S = np.abs(librosa.stft(y, n_fft=frame, hop_length=hop, win_length=frame,
                            window="hann", center=False))
    # 把频谱按 n_frames 截断/补齐（与 pitch 帧数严格一致）
    S = S[:, :n_frames]
    if S.shape[1] < n_frames:
        S = np.pad(S, ((0, 0), (0, n_frames - S.shape[1])), mode="edge")
    # 1) 能量（RMS，跨频带）
    energy = np.sqrt(np.mean(S ** 2, axis=0) + 1e-12)
    e_ref = max(float(np.max(energy)), 1e-6)
    energy_n = energy / e_ref

    # 2) 谱质心（人声基频区附近谐波最集中）
    centroid = librosa.feature.spectral_centroid(S=S, sr=sr, hop_length=hop)[0]
    # 3) 谱平坦度（人声低，噪声/气声高）
    flatness = librosa.feature.spectral_flatness(S=S, hop_length=hop)[0]

    voiced = (
        (energy_n > energy_thresh)
        & (centroid >= centroid_min) & (centroid <= centroid_max)
        & (flatness < flatness_max)
    ).astype(np.int8)

    # 时间平滑：去除 < min_voiced_ms 的孤立有声段（呼吸/爆音毛刺）
    voiced = _remove_short(voiced, int(round(min_voiced_ms / hop_ms)))
    return voiced


def _remove_short(mask: np.ndarray, min_len: int) -> np.ndarray:
    if min_len <= 1 or len(mask) < min_len:
        return mask
    out = mask.copy()
    i = 0
    n = len(mask)
    while i < n:
        if mask[i] == 1:
            j = i
            while j < n and mask[j] == 1:
                j += 1
            if (j - i) < min_len:
                out[i:j] = 0
            i = j
        else:
            i += 1
    return out


def apply_vad(pitch_points: List[Dict], mask: np.ndarray,
              frame_times: Optional[np.ndarray] = None,
              hop_ms: int = 10, sr: int = 16000) -> List[Dict]:
    """用有声掩码过滤 pitch 点。

    掩码按帧时间取：每个 pitch 点取其时间戳对应的帧索引；若该帧为无声则丢弃。
    当未提供 frame_times 时，按等间隔（hop_ms）重建帧时间，与 CREPE/pyin
    默认等间隔时间戳一致。
    """
    if mask is None or len(mask) == 0:
        return pitch_points
    if frame_times is None:
        n = len(mask)
        frame_times = np.arange(n) * (hop_ms / 1000.0)

    out = []
    for p in pitch_points:
        t = p["t"]
        idx = int(round(t / (hop_ms / 1000.0)))
        if 0 <= idx < len(mask) and mask[idx] == 1:
            out.append(p)
    return out
