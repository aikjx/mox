# -*- coding: utf-8 -*-
"""音乐解析层：BPM/节拍、调式识别、音符分割，以及颤音/滑音毛刺过滤。"""
from typing import Dict, List, Optional, Tuple

import numpy as np
import librosa


def freq2midi(freq: float):
    if freq <= 0:
        return None
    return int(round(69 + 12 * np.log2(freq / 440.0)))


# 音名（octave_normalize 同步 name 用；pipeline.midi_name 同源）
_NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']


def _midi_name(m: int) -> str:
    return f"{_NOTE_NAMES[m % 12]}{m // 12 - 1}"


def octave_normalize(notes: List[Dict], target_low: int = 48, target_high: int = 72,
                     fold_gap: int = 17) -> Tuple[List[Dict], int]:
    """八度归一化（检测器半频锁定 octave-halving 的企业级修复）。

    症状链路：音高检测器（crepe/pyin）对弱信号/远场哼唱常见半频锁定 →
    部分或全部音符 midi 偏低 1-2 八度 → 简谱满屏低音点（1__ 6___ …）+
    钢琴播放变成超低音轰鸣。简谱以相对音级记谱，绝对八度不承载乐义——
    把越界音域拉回钢琴/人声黄金区（C3–C5）是哼唱转谱的行业标准做法。

    区间语义（宽区间，忠实优先）：
      已处于 [target_low, target_high]（C3..C5，合成钢琴与人声双黄金区）
      的旋律原样保留——识别结果须忠实反映实际音高，正常输入不做无谓
      变调；仅当中位数越出区间时才整体平移 k×12 拉回。

    步骤（确定性，无随机源）：
      1) 整体平移 k×12：仅当音符 midi 中位数越出 [target_low,
         target_high] 时，平移至区间内（halving 典型残差 12-24 半音）；
      2) 孤立离群折叠（始终生效）：|midi - 中位| ≥ fold_gap（17 半音
         ≈ 1.5 八度）的音符 ±12 折回主体。真实旋律音域极少超 1.5 八度
         （茉莉花等极端也只 1 个八度跳进）；混合 halving 的散点由此收敛。

    返回 (新 notes 列表, 整体平移半音数)。不修改输入。
    """
    if not notes:
        return [], 0

    mids = sorted(int(round(float(n.get("midi", 60)))) for n in notes)
    med = float(mids[len(mids) // 2])

    shift = 0
    while med + shift < target_low:
        shift += 12
    while med + shift > target_high:
        shift -= 12

    out = []
    for n in notes:
        m = int(round(float(n.get("midi", 60)))) + shift
        # 孤立离群折叠（相对平移后的中位）
        d = m - (med + shift)
        if abs(d) >= fold_gap:
            m -= 12 * int(round(d / 12.0))
        item = dict(n)
        item["midi"] = int(m)
        if "name" in item:
            item["name"] = _midi_name(int(m))
        out.append(item)
    return out, shift


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


# BPM 全局业务约束（企业级参数化，避免把 88 BPM 识别成 176 这类翻倍 bug）
# 流行歌曲 95% 落在 [60, 140] BPM。极端值（>140 / <60）常是倍频歧义，
# 需要生成倍频簇 (bpm, 2*bpm, bpm/2, bpm/4...) 并按「与音符节奏拟合质量
# + 音区间距合理性 + 流行分布先验」选簇内代表值。
_BPM_MIN_SOFT: float = 50.0    # 软下界：低于此值的原始 BPM 会被强烈倾向翻倍
_BPM_MAX_SOFT: float = 140.0   # 软上界：超过此值的原始 BPM 会被强烈倾向折半
_BPM_MIN_HARD: float = 40.0    # 硬下界：绝对不可能再低于它（极慢速除外）
_BPM_MAX_HARD: float = 160.0   # 硬上界：绝对不可能再超过它（用户明确禁止176等极端值）

# 音乐时值（拍）栅格：十六分、八分、八分附点、四分、四分附点、二分、二分附点、全
_NOTE_MULTS = (0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0)
# 三连音栅格（等距对齐时允许：三连八分 0.333、三连四分 0.666）
_TRIPLET_MULTS = (1/3, 2/3)


def _normalize_to_cluster(bpm: float) -> List[float]:
    """生成 BPM 的倍频簇候选：bpm × {0.25, 0.5, 1, 2, 4}，裁剪到硬约束域。"""
    out: List[float] = []
    for mul in (0.25, 0.5, 1.0, 2.0, 4.0):
        cand = bpm * mul
        if _BPM_MIN_HARD <= cand <= _BPM_MAX_HARD:
            out.append(round(cand, 2))
    # 去重并升序
    return sorted(set(out))


def _fit_notes_to_bpm(durs: List[float], bpm: float) -> Tuple[float, float, float]:
    """给定候选 BPM，量化所有音符时长到合法音乐时值栅格。

    返回 (avg_err, quartile_err, frac_ongrid)：
      - avg_err       : 平均拟合偏差（按拍比差，0=完美）
      - quartile_err  : 75% 分位偏差（抗离群，<0.1 说明整体对齐）
      - frac_ongrid   : 音符「落栅」比例（偏差 ≤ 0.1 拍），越高越好
    """
    beat = 60.0 / bpm
    all_mults = _NOTE_MULTS + _TRIPLET_MULTS
    errs = []
    ongrid = 0
    for d in durs:
        # 最近音乐时值倍数的偏差
        ratio = d / beat
        # 允许 d 跨多个栅格：找最小 |ratio - m|
        best_err = min(abs(ratio - m) for m in all_mults)
        errs.append(best_err)
        if best_err <= 0.1:
            ongrid += 1
    if not errs:
        return 99.0, 99.0, 0.0
    avg = float(np.mean(errs))
    q75 = float(np.percentile(errs, 75))
    return avg, q75, ongrid / len(errs)


def _cluster_prior_soft(bpm: float) -> float:
    """流行曲 BPM 分布先验：高斯中心=95，σ=25，越高越合理。

    把 80~120 赋予最高先验，60 以下 / 130 以上先验快速衰减——
    直接抑制 librosa 常见的 160~200 翻倍输出（无先验下 176 可能
    仅靠音符拟合微弱优势胜出）。
    """
    mu, sigma = 95.0, 28.0
    return float(np.exp(-0.5 * ((bpm - mu) / sigma) ** 2))


def _pick_cluster_representative(cluster: List[float],
                                  durs: List[float]) -> Optional[float]:
    """在倍频簇内选最优代表值：拟合质量 × 先验 × 网格落栅率 加权。

    企业级 tie-break（避免 88→176 / 132→66 的 2×歧义）：
      实际构造中「1拍/2拍/0.5拍 随机组合」在 BPM 和 BPM/2 下的拟合 loss
      都极低（avg_err 差 < 小数第 3 位），用 loss 无法区分。这种情况
      必须靠「行业语义约束」判定：
        (a) 两者 avg_err 都 < 0.01（已高度拟合），且呈近似 2:1 关系；
        (b) 在 [_BPM_MIN_SOFT, _BPM_MAX_SOFT] 软区间内选更靠近中心
            （95）的那个；如两个都在软区间，选偏高者（避免 132→66 误折半，
            但不会把 88→176 拉到超 160 的硬上界外）。
    """
    if not cluster:
        return None
    if len(cluster) == 1:
        return cluster[0]

    scored = []
    for bpm in cluster:
        avg_err, q75_err, ongrid = _fit_notes_to_bpm(durs, bpm)
        prior = _cluster_prior_soft(bpm)
        score = ongrid * 1.0 + prior * 0.8 - q75_err * 2.0
        scored.append((bpm, score, avg_err, q75_err, ongrid))

    # Top-1 得主（loss+先验综合）
    scored.sort(key=lambda t: t[1], reverse=True)
    best_bpm, _, best_avg, _, _ = scored[0]

    # Pass 2：对所有候选两两比较，若满足 2× 歧义 + 都高拟合，按语义 tie-break
    n = len(scored)
    for i in range(n):
        for j in range(i + 1, n):
            bi, _, ai, _, _ = scored[i]
            bj, _, aj, _, _ = scored[j]
            if not (ai < 0.01 and aj < 0.01):
                continue  # 只有都高度拟合才可能是 2× 歧义
            # 是否 2:1 关系
            hi, lo = max(bi, bj), min(bi, bj)
            if abs(hi / lo - 2.0) > 0.05:
                continue
            # 语义 tie-break：
            # 1) 若只有一个落在软区间内，选那个
            hi_in = _BPM_MIN_SOFT <= hi <= _BPM_MAX_SOFT
            lo_in = _BPM_MIN_SOFT <= lo <= _BPM_MAX_SOFT
            if hi_in and not lo_in:
                if hi > best_bpm or not hi_in:
                    best_bpm = hi
                    continue
            if lo_in and not hi_in:
                if lo < best_bpm or not _BPM_MIN_SOFT <= best_bpm <= _BPM_MAX_SOFT:
                    best_bpm = lo
                    continue
            # 2) 两个都在区间内：选更接近先验中心 95 的
            if hi_in and lo_in:
                dist_hi = abs(hi - 95.0)
                dist_lo = abs(lo - 95.0)
                # 对 2× 歧义对（hi ≈ 2·lo），差值 = 1.5·hi − 190；
                # hi ≤ 142 时差 ≤ 23——这一整段都属于「折半误判风险区」，
                # 统一取高值以消除 70↔140、66↔132、60↔120 等流行 BPM 常见误折半。
                # （对 hi < 90 的真·慢速组合不会走到「都在软区间」分支，安全。）
                if abs(dist_hi - dist_lo) <= 23.0:
                    best_bpm = hi
                elif dist_hi < dist_lo:
                    best_bpm = hi
                else:
                    best_bpm = lo
    return float(best_bpm)


def _bpm_from_note_durations(durs: List[float]) -> Tuple[Optional[float], float]:
    """从音符时长分布拟合 BPM：枚举候选一拍时长 + 倍频簇投票。

    返回 (bpm, confidence_01)：拟合差则返回 None 让调用方走 librosa 兜底。
    """
    if not durs:
        return None, 0.0
    # 候选一拍时长：观测时长在 [0.25, 1.5]s 的代表值（对应 BPM 40~240）
    cands0 = sorted({round(d, 3) for d in durs if 0.25 <= d <= 1.5})
    # 候选不足时用中位数等分补齐，避免空集
    if not cands0:
        md = float(np.median(durs)) if durs else 0.5
        cands0 = sorted({round(md * k, 3) for k in (0.25, 0.5, 1.0, 2.0, 4.0)
                         if 0.25 <= md * k <= 1.5})
    if not cands0:
        return None, 0.0

    # Step 1：对每个候选拍长 beat，拟合音符得到「原始 BPM + 拟合质量」
    scored: List[Tuple[float, float, float]] = []  # (bpm, ongrid, q75_err)
    for beat in cands0:
        bpm0 = 60.0 / beat
        avg_err, q75, ongrid = _fit_notes_to_bpm(durs, bpm0)
        scored.append((bpm0, ongrid, q75))

    # Step 2：取 Top-N 原始 BPM（按 ongrid − 2×q75 得分），各扩成倍频簇
    scored.sort(key=lambda t: t[1] - 2.0 * t[2], reverse=True)
    top_raw = [b for b, _, _ in scored[:5]]

    # Step 3：簇合并去重，对每个簇选代表值
    cluster_pool: List[float] = []
    for bpm in top_raw:
        cluster_pool.extend(_normalize_to_cluster(bpm))
    # 把相近（±0.5%）的 BPM 视为同一簇，合并取均值
    cluster_pool.sort()
    merged_clusters: List[float] = []
    for b in cluster_pool:
        if merged_clusters and abs(b - merged_clusters[-1]) / merged_clusters[-1] < 0.01:
            merged_clusters[-1] = (merged_clusters[-1] + b) / 2.0
        else:
            merged_clusters.append(b)
    if not merged_clusters:
        return None, 0.0

    # Step 4：对合并后的候选簇按「音符拟合 + 先验」选最佳
    best = _pick_cluster_representative(merged_clusters, durs)
    if best is None:
        return None, 0.0
    final = best
    # Step 5：软约束纠偏（仅当最佳候选超出软区间时才翻倍/折半尝试）。
    # 真实 BPM = 132 / 140 仍然属于流行快歌范畴（软上界=140），
    # 不能把它们 ÷2 变成 66 / 70 慢歌——之前的条件 "final > _BPM_MAX_SOFT"
    # 会把 132 误判为"过高"强制折半。正确逻辑：final > 软上限时才÷2，
    # final < 软下限时才×2；若已经在软区间内则保留原样。
    # 同时要求：倍速候选的拟合偏差必须严格优于（< 92%）原候选，不能
    # "近似相当"就武断翻/折——避免拟合模糊区域出现 132↔66 的误判。
    if final > _BPM_MAX_SOFT and (final / 2.0) >= _BPM_MIN_SOFT:
        avg_hi, _, _ = _fit_notes_to_bpm(durs, final)
        avg_lo, _, _ = _fit_notes_to_bpm(durs, final / 2.0)
        # 严格更优才折：loss 必须比原候选小 10% 以上
        if avg_hi > 0 and avg_lo < avg_hi * 0.90:
            final = final / 2.0
    if final < _BPM_MIN_SOFT and (final * 2.0) <= _BPM_MAX_SOFT:
        avg_lo, _, _ = _fit_notes_to_bpm(durs, final)
        avg_hi, _, _ = _fit_notes_to_bpm(durs, final * 2.0)
        if avg_lo > 0 and avg_hi < avg_lo * 0.90:
            final = final * 2.0

    # 硬裁剪兜底：企业级不允许再输出 >160 的离谱 BPM（用户明确禁止176）
    final = max(_BPM_MIN_HARD, min(_BPM_MAX_HARD, final))

    # 置信度：网格落栅率 ×0.7 + 先验 ×0.3
    _, _, ongrid = _fit_notes_to_bpm(durs, final)
    conf = float(0.7 * ongrid + 0.3 * _cluster_prior_soft(final))
    return round(final, 2), conf


def _bpm_from_librosa(y: np.ndarray, sr: int,
                       durs: Optional[List[float]] = None) -> Optional[float]:
    """librosa.beat.tempo 兜底：带倍频簇校正，禁止直接输出原始 176。"""
    raw = None
    try:
        y_bt = y[: int(sr * 30)] if len(y) > sr * 30 else y
        if len(y_bt) >= sr:
            tempo, _ = librosa.beat.beat_track(y=y_bt, sr=sr, hop_length=512)
            raw = float(np.atleast_1d(tempo)[0])
    except Exception:
        raw = None
    if raw is None or not np.isfinite(raw) or raw < 20.0 or raw > 400.0:
        return None

    cluster = _normalize_to_cluster(raw)
    if not cluster:
        # 原始完全不在硬区间：推到最近边界（40 或 160）
        return max(_BPM_MIN_HARD, min(_BPM_MAX_HARD, raw))
    if durs:
        return _pick_cluster_representative(cluster, durs)
    # 无音符时按「先验 + 软区间贴近」选
    best_bpm: Optional[float] = None
    best_score = -1e9
    for b in cluster:
        score = _cluster_prior_soft(b)
        # 靠近软区间中心给一点加成
        if _BPM_MIN_SOFT <= b <= _BPM_MAX_SOFT:
            score += 0.1
        if score > best_score:
            best_score, best_bpm = score, b
    return best_bpm


def detect_bpm(y: np.ndarray, sr: int = 16000, fallback: float = 120.0,
                notes: Optional[list] = None) -> float:
    """企业级稳健 BPM 检测（多方法融合 + 倍频簇校正 + 硬范围约束）。

    修复用户核心投诉：「输出 176 BPM 太夸张」——根因是 librosa.tempo
    对混合音频常输出真实 BPM 的二倍频（真实 88 → 176），且无后处理。

    新链路：
      1) 音符时长拟合 → 得到候选 BPM1（带置信度）；
      2) librosa.beat.tempo 兜底 → 得到候选 BPM2；
      3) 两者分别走「倍频簇归一化 + 与音符拟合质量 + 流行曲分布先验」；
      4) 按置信度加权合并，最终强制裁剪到 [40, 160] 硬区间。
    保证不会再输出 160+ 这种脱离流行曲范围的 BPM。
    """
    fallback = float(fallback) if fallback else 120.0

    durs: List[float] = []
    if notes:
        durs = [max(0.05, float(n["end"] - n["start"])) for n in notes
                if "end" in n and "start" in n]
        durs = [d for d in durs if 0.05 < d < 4.0]

    bpm1, conf1 = _bpm_from_note_durations(durs)  # (float|None, float)
    bpm2 = _bpm_from_librosa(y, sr, durs)         # float|None

    # 合并规则：
    #   - 两者都有且相近（±8%）→ 按 conf1 加权平均；
    #   - bpm1 置信高（≥0.55） → 直接采用（音符拟合比 librosa 对人声哼唱更稳）；
    #   - 否则以 bpm2 为准，bpm2 无则回退 bpm1，再无则 fallback。
    final: Optional[float] = None
    if bpm1 is not None and bpm2 is not None:
        rel = abs(bpm1 - bpm2) / max(bpm1, bpm2)
        if rel <= 0.08:
            w = conf1
            final = bpm1 * w + bpm2 * (1.0 - w)
        elif conf1 >= 0.55:
            final = bpm1
        else:
            final = bpm2
    elif bpm1 is not None and conf1 >= 0.4:
        final = bpm1
    elif bpm2 is not None:
        final = bpm2
    elif bpm1 is not None:
        final = bpm1

    if final is None:
        final = fallback

    # 最终硬裁剪 + 四舍五入为整数（行业惯例：BPM 整数输出）
    final = max(_BPM_MIN_HARD, min(_BPM_MAX_HARD, final))
    return float(round(final, 1))


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
        # 平局偏大调：短哼唱/单声部旋律的 K-S 大小调得分常贴近（实测
        # C 大调旋律被误判 A 小调 → 音级全错）。小调须有显著优势
        # （>0.015）才胜出；简谱记谱惯例 1=X 亦默认大调。
        if vmin > best_v + 0.015:
            best_v, best = vmin, (names[i], 'minor')
    return best
