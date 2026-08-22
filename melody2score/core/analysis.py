# -*- coding: utf-8 -*-
"""音乐解析层：BPM/节拍、调式识别、音符分割，以及颤音/滑音毛刺过滤。"""
from typing import Dict, List, Optional, Tuple

import numpy as np
import librosa


def freq2midi(freq: float):
    if freq <= 0:
        return None
    return int(round(69 + 12 * np.log2(freq / 440.0)))


def _median_filter(x: np.ndarray, win: int, hole_after=None) -> np.ndarray:
    """中值滤波去颤音。四重修正（实测 C 被滤成 C#、边界帧被邻音改判的根因）：

    1) NaN（静音空洞）不参与中值、也不传染邻域——np.median 对含 NaN 的
       窗返回 NaN，会把音符"砍头去尾"；
    2) 窗口不得跨越 NaN——NaN 是音符边界（静音切分点），两侧帧属于不同
       音符；混窗会让边界帧被邻音"投票"改判（实测 C 音符帧混入相邻 D
       帧后中值变 63）。窗口在段内截断；
    3) 窗口不得跨越隐性空洞（hole_after[i] 表示点 i 与 i+1 之间有远超
       正常帧距的时间缝隙——检测器无置信帧的 attack/release 过渡区）。
       显性 NaN 之外，被置信阈值丢弃的帧同样构成音符边界；
    4) 偶数个有效值取下中位（次序统计量，必为实际数据值）——np.median
       取中间两数的均值会产生半音幻影（[60,60,62,62]→61=C#，凭空
       造出不存在的音高；[60,62,62,63]→62.5 四舍五入漂移）。
    """
    out = x.copy()
    half = win // 2
    n = len(x)
    for i in range(n):
        if np.isnan(x[i]):
            continue  # 静音空洞保持 NaN（分段切分点）
        # 段内窗口：向两侧扩展至多 half 帧，遇 NaN 或隐性空洞（音符边界）即停
        lo = i
        while (lo > 0 and not np.isnan(x[lo - 1]) and (i - lo) < half
               and not (hole_after is not None and hole_after[lo - 1])):
            lo -= 1
        hi = i
        while (hi < n - 1 and not np.isnan(x[hi + 1]) and (hi - i) < half
               and not (hole_after is not None and hole_after[hi])):
            hi += 1
        valid = x[lo:hi + 1]  # 段内窗口（无 NaN、未跨空洞）
        if len(valid):
            sv = np.sort(valid)
            out[i] = float(sv[(len(sv) - 1) // 2])  # 下中位：实际数据值
    return out


# 孤立短音保留下限（秒）：两侧均为静音边界的短段（真实弱起/跳音）低于
# min_note_dur 仍保留；低于此值才视为幻音丢弃。
_ISOLATED_FLOOR = 0.05


def segment_notes(pitch_points: List[Dict], min_note_dur: float = 0.1,
                  median_win: int = 5, vocal_mode: bool = False,
                  vad_mask=None, vad_hop_ms: int = 10) -> List[Dict]:
    """把连续 pitch 点切分为音符，并过滤颤音/滑音毛刺。

    流程：
      0)（可选）VAD 掩码过滤 —— 无人声段不产出音符（人声模式核心）；
      1) midi 轮廓中值滤波 —— 去掉颤音与帧间抖动（人声模式窗更大），
         窗口不跨静音/空洞边界（两侧帧属于不同音符）；
      2) 半音量化后按相同音高分段，并标记段间「静音边界」（sep_prev）：
         显性 NaN 空洞（VAD 判静音）+ 隐性帧缺失缝隙（置信度不足被丢弃）；
      3) VAD 切边回补 → 边界伪音清除 → 短段合并（静音边界感知，
         不跨静音合并）；
      4) 最终过滤：常规音符须 > min_note_dur；静音夹持的孤立短音放宽到
         _ISOLATED_FLOOR（弱起/跳音短于 min_note_dur 仍保留）。
    """
    if not pitch_points:
        return []

    # 0) VAD：把静音帧的 pitch 点置 NaN（而非删除），让下方分段在 NaN 处自然切分。
    #    直接删除会把相邻同音间的短静音「粘」成一段（尤其 CREPE 谐波泄漏在静音处
    #    仍解为高 conf 同音时），导致"一闪一闪"被识别成"一——"。置 NaN 可保留
    #    时间空洞，使相邻同音正确断开。
    if vad_mask is not None and len(vad_mask) > 0:
        silent = 0
        for p in pitch_points:
            idx = int(round(p["t"] / (vad_hop_ms / 1000.0)))
            if not (0 <= idx < len(vad_mask)) or vad_mask[idx] == 0:
                p["freq"] = 0.0  # freq2midi(0) -> None，分段时视为切分点
                silent += 1
        # 仅当几乎所有点都被判静音时才整体放弃
        if silent >= 0.9 * len(pitch_points):
            return []

    # 人声模式：颤音更明显，中值窗不足时自动加窗
    if vocal_mode:
        median_win = max(median_win, 7)

    # 隐性空洞检测：两相邻 pitch 点时间差远超正常帧距 → 中间是检测器无
    # 置信帧的缝隙（attack/release 过渡区/瞬态低置信），同样是音符边界。
    # 仅靠显性 NaN 检测不到（这些帧被后端置信阈值直接丢弃、根本不出现）。
    # 阈值取 2.6×帧距：单帧丢弃（2 帧距缝隙）是音内置信度凹陷（实测
    # 生日歌 C1 中部 conf=0.24 帧在扰动阈值 0.28/0.32 下被丢，72ms 缝隙
    # 若判为边界会把长音劈成两段）；真实音符边界（静音 ≥60ms + 起音/
    # 收尾各丢 1 帧）稳定 ≥3 帧距。
    ts = [float(p["t"]) for p in pitch_points]
    if len(ts) >= 3:
        med_hop = float(np.median(np.diff(ts)))
    else:
        med_hop = 0.05
    hole_gap = max(0.03, 2.6 * med_hop)
    hole_after = np.zeros(len(ts), dtype=bool)
    for i in range(len(ts) - 1):
        if ts[i + 1] - ts[i] > hole_gap:
            hole_after[i] = True

    mids = np.array([freq2midi(p["freq"]) for p in pitch_points], dtype=float)
    mids = _median_filter(mids, median_win, hole_after)

    # 半音量化 + 初分段（含静音边界标记）
    # 注意：VAD 静音帧被置 NaN（见上方步骤 0），遇到 NaN 表示此处有「时间空洞」，
    # 应把当前正在累积的音符先收尾（append）再断开，而非直接丢弃——否则相邻同音
    # 间的短静音会把两段都吞掉。连续多个 NaN 时仅在首次断开，避免重复 append。
    # sep_prev：该段与上一段之间是否存在静音边界（NaN 空洞 / 隐性帧缺失缝隙 /
    # 流起始）。合并短段时禁止跨越——静音两侧是两个独立声学事件（生日歌弱起
    # 0.25 拍 C 与前面 0.75 拍 C 之间有真实停顿），合并会把短音吞进长音。
    raw: List[Dict] = []
    cur: Optional[Dict] = None
    hole = True   # 流起始视为边界
    last_t: Optional[float] = None
    for p, m in zip(pitch_points, mids):
        if np.isnan(m):
            if cur is not None:
                raw.append(cur)
                cur = None
            hole = True
            last_t = p["t"]
            continue
        if last_t is not None and (p["t"] - last_t) > hole_gap:
            # 隐性空洞：两有效帧之间无任何置信帧
            if cur is not None:
                raw.append(cur)
                cur = None
            hole = True
        mi = int(round(m))
        if cur is None:
            cur = {"midi": mi, "start": p["t"], "end": p["t"],
                   "sep_prev": hole, "nfr": 1}
        elif cur["midi"] == mi:
            cur["end"] = p["t"]
            cur["nfr"] += 1
        else:
            raw.append(cur)
            cur = {"midi": mi, "start": p["t"], "end": p["t"],
                   "sep_prev": False, "nfr": 1}
        hole = False
        last_t = p["t"]
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
    # 合并短段到音高最近的邻居（不跨静音边界）
    raw = _merge_short(raw, min_note_dur)
    # 最终过滤：常规音符 > min_note_dur；静音夹持的孤立短音 ≥ _ISOLATED_FLOOR
    # 且 ≥2 帧（单帧孤立段是释放尾音/间隙幻音——实测 G 音尾部在 G|F 间隙
    # 中的单帧残响被误留为独立音符；真实短音即使 0.25 拍也有 ≥2 帧）
    notes: List[Dict] = []
    for i, nrec in enumerate(raw):
        dur = nrec["end"] - nrec["start"]
        if dur > min_note_dur:
            notes.append(nrec)
            continue
        sep_prev = (i == 0) or bool(raw[i].get("sep_prev"))
        sep_next = (i == len(raw) - 1) or bool(raw[i + 1].get("sep_prev"))
        if (sep_prev and sep_next and dur >= _ISOLATED_FLOOR
                and nrec.get("nfr", 1) >= 2):
            # 孤立短音：两静音夹持的真实短音（弱起/跳音）。静音夹持意味着
            # 不可能是持续发音内部的毛刺（毛刺与本体时间相连），保留安全。
            notes.append(nrec)
    return notes


def _pad_note_boundaries(notes: List[Dict], pad: float) -> List[Dict]:
    """音符边界对称回补，相邻重叠取中点切分（不改变音高/顺序/静音边界标记）。"""
    if not notes or pad <= 0:
        return notes
    out = [{"midi": n["midi"], "start": n["start"] - pad, "end": n["end"] + pad,
            "sep_prev": bool(n.get("sep_prev")), "nfr": n.get("nfr", 1)}
           for n in notes]
    for i in range(1, len(out)):
        if out[i]["start"] < out[i - 1]["end"]:
            mid = (out[i - 1]["end"] + out[i]["start"]) / 2.0
            out[i - 1]["end"] = mid
            out[i]["start"] = mid
    return out


def _drop_boundary_artifacts(notes: List[Dict], min_note_dur: float) -> List[Dict]:
    """清除边界伪音：短音符 B 夹在两个相同音高 A 之间（A|B|A 且 B 短）。

    B 是帧窗口横跨 A|gap|A 解出的中间伪音高（attack/release 过渡帧、
    谐波泄漏到静音空洞），并非真实音符。直接删除 B：两侧 A 保持各自
    边界，B 占据的时间自然留空为间隔——相邻同音本就被静音空洞分开
    （见 segment_notes 步骤 0），删除伪音后依然是两个独立音符，不会
    误粘合；也不会被 _merge_short 并入邻居而拉长其边界（时值/BPM 失真根因）。

    阈值取 1.5×min_note_dur：伪音（帧窗横跨 60ms 间隙 + 30ms 回补 ≈
    70~120ms）稳定落入；真实经过音/回音（语料最短 0.5 拍 ≈ 210ms）不受影响。
    """
    if len(notes) < 3:
        return notes
    thresh = 1.5 * min_note_dur
    out: List[Dict] = []
    for i, n in enumerate(notes):
        if (0 < i < len(notes) - 1
                and n["midi"] != notes[i - 1]["midi"]
                and notes[i - 1]["midi"] == notes[i + 1]["midi"]
                and (n["end"] - n["start"]) < thresh):
            continue  # A|B|A 边界伪音：跳过（不入 out）
        out.append(n)
    return out


def _merge_short(notes: List[Dict], min_note_dur: float) -> List[Dict]:
    """合并过短段到最合理的邻居（静音边界感知）。

    修复「短音被吞」（实测生日歌·human_voice 音符数 12→10 的根因）：
    旧版把 < min_note_dur 的短段无条件并入邻居——弱起 0.25 拍 C（105ms，
    检测后仅剩单帧）被并入前面 0.75 拍 C。静音两侧是两个独立声学事件，
    禁止合并：

      - 仅可并入「无静音间隔」的邻居：时间直接相连说明同属一个持续
        发音（滑音/颤音毛刺），合并正确（与 sep_prev 标记联动）；
      - 两侧均为静音边界的孤立短段不参与合并，交由最终过滤按
        _ISOLATED_FLOOR 裁决（真实弱起/跳音保留，幻音丢弃）。

    邻居选择（在允许方向内）：两侧都更长时按音高最近（典型颤音毛刺）；
    仅一侧更长选该侧；都不更长退化为音高最近。
    """
    out = list(notes)
    changed = True
    while changed:
        changed = False
        n = len(out)
        for i in range(n):
            dur_i = out[i]["end"] - out[i]["start"]
            if dur_i > min_note_dur:
                continue
            # 允许的合并方向：与该邻居之间无静音边界（sep_prev 记在段间后者上）
            prev_ok = i > 0 and not out[i].get("sep_prev")
            next_ok = i < n - 1 and not out[i + 1].get("sep_prev")
            if not prev_ok and not next_ok:
                continue  # 孤立短段：不跨静音合并

            best: Optional[int]
            if prev_ok and next_ok:
                dl = out[i - 1]["end"] - out[i - 1]["start"]
                dr = out[i + 1]["end"] - out[i + 1]["start"]
                dp = abs(out[i - 1]["midi"] - out[i]["midi"])
                dn = abs(out[i + 1]["midi"] - out[i]["midi"])
                if dl > dur_i and dr > dur_i:
                    best = i - 1 if dp <= dn else i + 1     # 两侧更长：音高最近
                elif dl > dur_i:
                    best = i - 1
                elif dr > dur_i:
                    best = i + 1
                else:
                    best = i - 1 if dp <= dn else i + 1     # 都不更长：音高最近
            elif prev_ok:
                best = i - 1
            else:
                best = i + 1

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
    不可信。因此**优先用音符时长分布拟合 BPM**（O(音符数²) 但常数极小），
    仅当无可用音符或拟合质量差时才回退到 beat_track。

    拟合方法（修复旧版固定拍类网格 {4,2,1.5,1,...,0.125} 的失配）：
    真实一拍时长（如 0.42s）往往不在固定网格上，0.38s 的音符会被硬映射到
    0.125 拍类（3.07≈3 拍），众数落到 0.125 → 480 BPM 超界被拒 → 整体
    退化为 beat_track（实测生日歌 BPM=125，0.75 拍音被量化成 0.5 拍）。
    改为：候选拍值 = 观测时长本身（BPM 合法域 [40,240] → 拍值 [0.25,1.5]s），
    评分 = 所有时长对拍值整数倍（含 0.25/0.5/0.75/1.5 等音乐时值）的平均
    拟合偏差 − 主拍占比加成（多数旋律以四分音符为主，落在 1 拍的音符
    占比高者是正确拍值，抑制半拍/双拍歧义——实测欢乐颂 0.78s 候选以微弱
    误差优势压过正确的 0.38s，靠主拍占比纠正）。
    """
    if notes:
        durs = [max(0.05, float(n["end"] - n["start"])) for n in notes
                if "end" in n and "start" in n]
        durs = [d for d in durs if 0.05 < d < 4.0]
        if durs:
            # 候选一拍时长：观测时长的代表值（BPM 40~240 → 一拍 0.25~1.5s）
            cands = sorted({round(d, 2) for d in durs if 0.25 <= d <= 1.5})
            # 音乐时值（拍）：十六分…全音符（含附点）
            mults = (0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0)
            best_beat: Optional[float] = None
            best_score = best_avg = 0.0
            for beat in cands:
                errs = [min(abs(d / beat - m) for m in mults) for d in durs]
                avg = float(np.mean(errs))
                frac1 = sum(1 for d in durs if abs(d / beat - 1.0) <= 0.15) / len(durs)
                score = avg - 0.5 * frac1  # 主拍占比加成
                if best_beat is None or score < best_score:
                    best_beat, best_score, best_avg = beat, score, avg
            if best_beat is not None and best_avg < 0.12:
                bpm = float(60.0 / best_beat)
                if 40.0 <= bpm <= 240.0:
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
