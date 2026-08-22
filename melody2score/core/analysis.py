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
    """中值滤波去颤音；NaN（静音空洞）不参与中值、也不传染邻域。

    修复：np.median 对含 NaN 的窗返回 NaN，导致每个静音空洞把前后
    win//2 个有效帧全部传染为 NaN——音符被"砍头去尾"只剩 2~3 帧
    （实测 0.5s 音符被切成 0.11s 碎片，时值/ BPM 全线失真）。
    正确语义：仅对窗内有效值取中值；全 NaN 窗保持 NaN（静音空洞本身）。
    """
    out = x.copy()
    half = win // 2
    for i in range(len(x)):
        if np.isnan(out[i]):
            continue  # 静音空洞保持 NaN（分段切分点）
        lo = max(0, i - half)
        hi = min(len(x), i + half + 1)
        seg = x[lo:hi]
        valid = seg[~np.isnan(seg)]
        if len(valid):
            out[i] = float(np.median(valid))
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

    # 0) VAD：把静音帧的 pitch 点置 NaN（而非删除），让下方分段在 NaN 处自然切分。
    #    直接删除会把相邻同音间的短静音「粘」成一段（尤其 CREPE 谐波泄漏在静音处
    #    仍解为高 conf 同音时），导致"一闪一闪"被识别成"一——"。置 NaN 可保留
    #    时间空洞，使相邻同音正确断开。
    if vad_mask is not None and len(vad_mask) > 0:
        hop_ms = 10
        silent = 0
        for p in pitch_points:
            idx = int(round(p["t"] / (hop_ms / 1000.0)))
            if not (0 <= idx < len(vad_mask)) or vad_mask[idx] == 0:
                p["freq"] = 0.0  # freq2midi(0) -> None，分段时视为切分点
                silent += 1
        # 仅当几乎所有点都被判静音时才整体放弃
        if silent >= 0.9 * len(pitch_points):
            return []

    # 人声模式：颤音更明显，中值窗不足时自动加窗
    if vocal_mode:
        median_win = max(median_win, 7)

    mids = np.array([freq2midi(p["freq"]) for p in pitch_points], dtype=float)
    mids = _median_filter(mids, median_win)

    # 半音量化 + 初分段
    # 注意：VAD 静音帧被置 NaN（见上方步骤 0），遇到 NaN 表示此处有「时间空洞」，
    # 应把当前正在累积的音符先收尾（append）再断开，而非直接丢弃——否则相邻同音
    # 间的短静音会把两段都吞掉。连续多个 NaN 时仅在首次断开，避免重复 append。
    raw: List[Dict] = []
    cur = None
    for p, m in zip(pitch_points, mids):
        if np.isnan(m):
            if cur is not None:
                raw.append(cur)
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

    # VAD 切边回补（须在过滤之前）：VAD 以能量门限判有声，attack 爬升段
    # 与指数衰减尾系统性低于门限，且 pyin 帧中心落在静音区的边缘帧会被
    # 整帧判杀 → 音符边界两端被切（实测 0.42s → 0.27~0.32s，BPM 反推
    # 随之落到错误拍类）。回补量取 30ms：足以找回边缘帧并让 0.25 拍
    # 短音符存活，又不至于把边界毛刺养到超过过滤线；无 VAD 不回补。
    if vad_mask is not None and len(vad_mask) > 0:
        raw = _pad_note_boundaries(raw, 0.03)
    # 边界伪音清除：短音符夹在两个相同音高之间（A|B|A 且 B 短），
    # B 是帧窗口横跨 A|gap|A 解出的中间伪音高，截断两侧 A 的边界即可。
    raw = _drop_boundary_artifacts(raw, min_note_dur)
    # 合并短段到音高最近的邻居（处理滑音/颤音毛刺）
    raw = _merge_short(raw, min_note_dur)
    # 过滤过短音符
    notes = [n for n in raw if (n["end"] - n["start"]) > min_note_dur]
    return notes


def _pad_note_boundaries(notes: List[Dict], pad: float) -> List[Dict]:
    """音符边界对称回补，相邻重叠取中点切分（不改变音高/顺序）。"""
    if not notes or pad <= 0:
        return notes
    out = [{"midi": n["midi"], "start": n["start"] - pad, "end": n["end"] + pad}
           for n in notes]
    for i in range(1, len(out)):
        if out[i]["start"] < out[i - 1]["end"]:
            mid = (out[i - 1]["end"] + out[i]["start"]) / 2.0
            out[i - 1]["end"] = mid
            out[i]["start"] = mid
    return out


def _merge_short(notes: List[Dict], min_note_dur: float) -> List[Dict]:
    """合并过短段到最合理的邻居。

    合并策略（比单纯「音高最近」更稳）：
      - 短段优先并入**时间上紧邻**且**时长更长**的邻居（颤音/滑音毛刺通常
        夹在两个长音之间，应并入相邻长音而非误并入另一个真实短音）；
      - 仅当两侧都更长时，才在音高最近者中选；若被长段夹在中间且两侧均
        比它长，则按音高最近合并。
    """
    out = list(notes)
    changed = True
    while changed:
        changed = False
        n = len(out)
        for i in range(n):
            dur_i = out[i]["end"] - out[i]["start"]
            if dur_i <= min_note_dur:
                prev_longer = out[i - 1]["end"] - out[i - 1]["start"] > dur_i if i > 0 else False
                next_longer = out[i + 1]["end"] - out[i + 1]["start"] > dur_i if i < n - 1 else False

                # 两侧都更长：按音高最近合并（典型颤音毛刺）
                if prev_longer and next_longer:
                    best = i - 1 if abs(out[i - 1]["midi"] - out[i]["midi"]) <= abs(out[i + 1]["midi"] - out[i]["midi"]) else i + 1
                elif prev_longer:
                    best = i - 1
                elif next_longer:
                    best = i + 1
                elif i > 0 or i < n - 1:
                    # 两侧都更短/等长：退化为音高最近
                    best, best_d = None, 1e9
                    if i > 0:
                        d = abs(out[i - 1]["midi"] - out[i]["midi"])
                        if d < best_d:
                            best_d, best = d, i - 1
                    if i < n - 1:
                        d = abs(out[i + 1]["midi"] - out[i]["midi"])
                        if d < best_d:
                            best_d, best = d, i + 1
                else:
                    out.pop(i)
                    changed = True
                    break

                if best is not None:
                    nb = out[best]
                    nb["start"] = min(nb["start"], out[i]["start"])
                    nb["end"] = max(nb["end"], out[i]["end"])
                    out.pop(i)
                    changed = True
                    break
    return out


def detect_bpm(y: np.ndarray, sr: int = 16000, fallback: float = 120.0,
                notes: Optional[list] = None) -> float:
    """稳健 BPM 检测。

    性能：librosa.beat.beat_track 内部做 STFT + 动态规划节拍追踪，对短音频
    也常耗时数秒（实测 7.6s 音频 ~4.3s），且对哼唱/合成音轨返回的 tempo 往往
    不可信。因此**优先用音符时长反推 BPM**（O(音符数)、瞬间完成），仅当无
    可用音符时才回退到 beat_track，进一步失败再落 fallback。

    修复「哼唱/合成音轨 BPM=0 导致五线谱退化」的根因。
    """
    # 首选：从音符时长反推 BPM。
    # 关键修正：音乐里长音/二分/全音符/附点的时长会远大于一拍，"一拍 ≈ 一音符"
    # 的中位假设会把它们误判成极慢 BPM（如整拍长音 → 60/2=30）。规范要求按
    # "最常见拍类音符"（四分/八分等单拍级）反推：统计每个音符时长落在哪个
    # 拍类（1拍/半拍/2拍…），取众数拍类的时长作为"一拍"基准。
    if notes:
        durs = [max(0.05, float(n["end"] - n["start"])) for n in notes
                if "end" in n and "start" in n]
        durs = [d for d in durs if 0.05 < d < 4.0]
        if durs:
            # 候选拍类时长（秒）：四分/八分/二分/附点四分/十六分/附点八分/全音符
            candidate_beats = [4.0, 2.0, 1.5, 1.0, 0.75, 0.5, 0.25, 0.125]
            # 把每个音符时长映射到"最贴近的整数拍"，得到候选一拍时长
            beat_candidates = []
            for d in durs:
                # 找使 d/beat 最接近 {1,2,3,4,...} 整数拍的 beat 候选
                best_beat, best_err = 1.0, 1e9
                for beat in candidate_beats:
                    for mult in (1, 2, 3, 4):
                        err = abs(d / beat - mult)
                        if err < best_err:
                            best_err, best_beat = err, beat
                beat_candidates.append(best_beat)
            # 取众数拍类对应的时长作为"一拍"基准（最稳健、抗长音污染）
            vals, counts = np.unique(np.round(beat_candidates, 4), return_counts=True)
            one_beat = float(vals[int(np.argmax(counts))])
            if 0.05 < one_beat < 4.0:
                bpm = float(60.0 / one_beat)
                if 30.0 <= bpm <= 300.0:
                    return bpm

    # 兜底：仅当没有可用音符时才跑昂贵的 beat_track。
    # 限长 30s：beat_track 耗时与音频长度近似线性（实测 9s≈2.7s），
    # 长音频全量计算会拖垮 API 延迟；节拍周期统计取前 30s 已足够。
    raw = None
    try:
        y_bt = y[: int(sr * 30)] if len(y) > sr * 30 else y
        if len(y_bt) >= sr:  # 短于 1s 无节拍可言
            tempo, _ = librosa.beat.beat_track(y=y_bt, sr=sr, hop_length=512)
            raw = float(np.atleast_1d(tempo)[0])
    except Exception:
        raw = None

    if raw is not None and np.isfinite(raw) and 30.0 <= raw <= 300.0:
        return raw

    return float(fallback)


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
