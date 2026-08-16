# -*- coding: utf-8 -*-
"""经典旋律语料库 + 多音色合成。

数据：十几首公版（无版权风险）经典旋律，每首以 MIDI 音高序列精确标注，
作为「真实识别」的 ground truth（期望音高）。

音色（覆盖 乐器 / 人声 / 纯音乐 三大类，且乐器含钢琴/吉他/弦乐/长笛/风琴/钟声）：
  - 乐器 instrument : piano / guitar / strings / flute / organ / bell
  - 人声 voice      : human_voice（元音共振峰合成）
  - 纯音乐 pure     : pure_sine / pure_triangle（纯净单频/三角波）

每首旋律按 3 类音色各渲染一次 → 几十个样例，满足「不同旋律×不同乐器×不同人声×纯音乐」。

说明：音源为合成（非下载 copyrighted 音频），但识别是真实的——
音高检测层会对其跑真实模型（crepe_onnx / torchcrepe / pyin），验证端到端链路。
"""
import json
import math
import os
from typing import Dict, List, Optional, Tuple

import numpy as np

BEAT = 0.42          # 每拍秒数
GAP = 0.06           # 音符间隙秒（留出静音便于音高检测切分音符边界）
NOISE_SNR = 42.0     # 注入底噪信噪比(dB)，模拟真实录音

# 音名 -> 半音偏移
_SEMI = {'C': 0, 'C#': 1, 'D': 2, 'D#': 3, 'E': 4, 'F': 5,
         'F#': 6, 'G': 7, 'G#': 8, 'A': 9, 'A#': 10, 'B': 11}


def n(name: str, octave: int) -> int:
    """音名+八度 -> MIDI 编号（C4=60）。"""
    return 12 * (octave + 1) + _SEMI[name]


def midi2freq(m: int) -> float:
    return 440.0 * 2.0 ** ((m - 69) / 12.0)


# ---------------------------------------------------------------------------
# 旋律定义：(title_zh, title_en, 音高序列)
# 序列元素为 (midi, beats)，midi<=0 表示休止符(r)占位，仅记时长。
# ---------------------------------------------------------------------------
MELODIES: List[Tuple[str, str, List[Tuple[int, float]]]] = [
    ("小星星", "Twinkle Twinkle Little Star", [
        (n('C', 4), 1), (n('C', 4), 1), (n('G', 4), 1), (n('G', 4), 1),
        (n('A', 4), 1), (n('A', 4), 1), (n('G', 4), 2),
        (n('F', 4), 1), (n('F', 4), 1), (n('E', 4), 1), (n('E', 4), 1),
        (n('D', 4), 1), (n('D', 4), 1), (n('C', 4), 2)]),

    ("欢乐颂", "Ode to Joy", [
        (n('E', 4), 1), (n('E', 4), 1), (n('F', 4), 1), (n('G', 4), 1),
        (n('G', 4), 1), (n('F', 4), 1), (n('E', 4), 1), (n('D', 4), 1),
        (n('C', 4), 1), (n('C', 4), 1), (n('D', 4), 1), (n('E', 4), 1),
        (n('E', 4), 1.5), (n('D', 4), 0.5), (n('D', 4), 2)]),

    ("生日歌", "Happy Birthday", [
        (n('C', 4), 0.75), (n('C', 4), 0.25), (n('D', 4), 1), (n('C', 4), 1),
        (n('F', 4), 1), (n('E', 4), 2),
        (n('C', 4), 0.75), (n('C', 4), 0.25), (n('D', 4), 1), (n('C', 4), 1),
        (n('G', 4), 1), (n('F', 4), 2)]),

    ("两只老虎", "Frère Jacques", [
        (n('C', 4), 1), (n('D', 4), 1), (n('E', 4), 1), (n('C', 4), 1),
        (n('C', 4), 1), (n('D', 4), 1), (n('E', 4), 1), (n('C', 4), 1),
        (n('E', 4), 1), (n('F', 4), 1), (n('G', 4), 2),
        (n('E', 4), 1), (n('F', 4), 1), (n('G', 4), 2)]),

    ("茉莉花", "Jasmine Flower", [
        (n('E', 4), 1), (n('E', 4), 1), (n('G', 4), 1), (n('A', 4), 1),
        (n('C', 5), 1), (n('C', 5), 1), (n('A', 4), 2),
        (n('G', 4), 1), (n('A', 4), 1), (n('G', 4), 1), (n('E', 4), 1),
        (n('D', 4), 2)]),

    ("致爱丽丝", "Für Elise", [
        (n('E', 5), 1), (n('D', 5), 0.5), (n('E', 5), 0.5), (n('D', 5), 0.5),
        (n('E', 5), 0.5), (n('B', 4), 1), (n('D', 5), 1), (n('C', 5), 1),
        (n('A', 4), 2),
        (n('C', 4), 1), (n('E', 4), 1), (n('A', 4), 1), (n('B', 4), 2)]),

    ("雪绒花", "Edelweiss", [
        (n('D', 4), 1), (n('D', 4), 1), (n('B', 3), 1), (n('C#', 4), 1),
        (n('D', 4), 1), (n('E', 4), 1), (n('E', 4), 2),
        (n('D', 4), 1), (n('E', 4), 1), (n('F', 4), 1), (n('E', 4), 1),
        (n('D', 4), 1), (n('C#', 4), 2)]),

    ("友谊地久天长", "Auld Lang Syne", [
        (n('C', 4), 1), (n('F', 4), 1), (n('F', 4), 1), (n('F', 4), 1),
        (n('A', 4), 1), (n('G', 4), 1), (n('F', 4), 1), (n('E', 4), 1),
        (n('F', 4), 1), (n('A', 4), 1), (n('G', 4), 1), (n('F', 4), 2)]),

    ("伦敦大桥", "London Bridge", [
        (n('G', 4), 1), (n('A', 4), 1), (n('G', 4), 1), (n('F', 4), 1),
        (n('E', 4), 1), (n('F', 4), 1), (n('G', 4), 2),
        (n('D', 4), 1), (n('E', 4), 1), (n('F', 4), 2)]),

    ("玛丽的小羊", "Mary Had a Little Lamb", [
        (n('E', 4), 1), (n('D', 4), 1), (n('C', 4), 1), (n('D', 4), 1),
        (n('E', 4), 1), (n('E', 4), 1), (n('E', 4), 2),
        (n('D', 4), 1), (n('D', 4), 1), (n('D', 4), 2)]),

    ("铃儿响叮当", "Jingle Bells", [
        (n('E', 4), 1), (n('E', 4), 1), (n('E', 4), 2),
        (n('E', 4), 1), (n('E', 4), 1), (n('E', 4), 2),
        (n('E', 4), 1), (n('G', 4), 1), (n('C', 4), 1), (n('D', 4), 1),
        (n('E', 4), 2)]),

    ("老麦克唐纳", "Old MacDonald", [
        (n('G', 4), 1), (n('G', 4), 1), (n('G', 4), 1), (n('D', 4), 1),
        (n('E', 4), 1), (n('E', 4), 1), (n('D', 4), 2),
        (n('B', 4), 1), (n('B', 4), 1), (n('A', 4), 1), (n('A', 4), 1),
        (n('G', 4), 2)]),

    ("划船歌", "Row Row Row Your Boat", [
        (n('C', 4), 1), (n('C', 4), 1), (n('C', 4), 1), (n('D', 4), 1),
        (n('E', 4), 2),
        (n('E', 4), 1), (n('D', 4), 1), (n('E', 4), 1), (n('F', 4), 1),
        (n('G', 4), 3)]),

    ("平安夜", "Silent Night", [
        (n('G', 4), 1), (n('A', 4), 1), (n('G', 4), 1), (n('E', 4), 2),
        (n('G', 4), 1), (n('A', 4), 1), (n('G', 4), 1), (n('E', 4), 2),
        (n('C', 4), 1), (n('C', 4), 1), (n('C', 4), 1), (n('C', 4), 1),
        (n('C', 4), 1), (n('D', 4), 1), (n('E', 4), 1), (n('D', 4), 1),
        (n('C', 4), 2)]),

    ("红河谷", "Red River Valley", [
        (n('C', 4), 1), (n('E', 4), 1), (n('G', 4), 1), (n('A', 4), 1),
        (n('G', 4), 1), (n('E', 4), 1), (n('C', 4), 1), (n('D', 4), 1),
        (n('E', 4), 1), (n('G', 4), 1), (n('A', 4), 2)]),

    ("婚礼进行曲", "Wedding March", [
        (n('C', 4), 1), (n('F', 4), 1), (n('G', 4), 1), (n('C', 5), 1),
        (n('G', 4), 1), (n('C', 5), 1), (n('F', 4), 1), (n('G', 4), 1),
        (n('C', 5), 1.5), (n('B', 4), 0.5), (n('C', 5), 2)]),
]

# 三大类 × 各自音色
CATEGORY_TIMBRES = {
    "instrument": ["piano", "guitar", "strings", "flute", "organ", "bell"],
    "voice": ["human_voice"],
    "pure": ["pure_sine", "pure_triangle"],
}


# ---------------------------------------------------------------------------
# 音色合成
# ---------------------------------------------------------------------------
def _adsr(n_samples: int, sr: int, attack: float, release: float) -> np.ndarray:
    env = np.ones(n_samples, dtype=np.float64)
    a = int(attack * sr)
    r = int(release * sr)
    if a > 0:
        env[:a] = np.linspace(0, 1, a)
    if r > 0:
        env[-r:] = np.linspace(1, 0, r)
    return env


def _add_harmonics(freq, t, amps):
    out = np.zeros_like(t)
    for k, a in enumerate(amps, start=1):
        out += a * np.sin(2 * np.pi * k * freq * t)
    return out


def render_note(freq: float, n_samples: int, sr: int, timbre: str) -> np.ndarray:
    t = np.arange(n_samples) / sr
    dur = n_samples / sr
    if timbre == "pure_sine":
        sig = np.sin(2 * np.pi * freq * t)
        sig *= _adsr(n_samples, sr, 0.005, 0.02)

    elif timbre == "pure_triangle":
        sig = _add_harmonics(freq, t, [(-1) ** ((k - 1) // 2) / (k * k)
                                       for k in range(1, 10, 2)])
        sig *= _adsr(n_samples, sr, 0.005, 0.02)

    elif timbre == "piano":
        amps = [1.0, 0.6, 0.4, 0.25, 0.15, 0.1, 0.06]
        sig = _add_harmonics(freq, t, amps)
        tau = max(0.05, 0.35 * dur)
        sig *= np.exp(-t / tau)
        sig *= _adsr(n_samples, sr, 0.004, 0.03)

    elif timbre == "guitar":
        # Karplus-Strong 拨弦
        N = max(2, int(round(sr / freq)))
        ring = np.random.uniform(-1.0, 1.0, N)
        y = np.zeros(n_samples)
        for i in range(n_samples):
            y[i] = ring[0]
            avg = 0.5 * (ring[0] + ring[1])
            ring = np.roll(ring, -1)
            ring[-1] = avg
        sig = y
        tau = max(0.08, 0.4 * dur)
        sig *= np.exp(-t / tau)
        sig *= _adsr(n_samples, sr, 0.003, 0.03)

    elif timbre == "strings":
        vibrato = 1 + 0.006 * np.sin(2 * np.pi * 5.5 * t)  # ±半音内颤音
        amps = [1.0 / k for k in range(1, 22)]
        sig = _add_harmonics(freq, t * vibrato, amps)
        sig *= _adsr(n_samples, sr, 0.06, 0.05)

    elif timbre == "flute":
        sig = np.sin(2 * np.pi * freq * t)
        breath = np.random.randn(n_samples) * 0.04
        sig = sig + breath
        sig *= _adsr(n_samples, sr, 0.03, 0.04)

    elif timbre == "organ":
        # 持续音：基频 + 八度 + 十二度（保留清晰基频便于音高追踪）
        parts = [freq, 2 * freq, 3 * freq]
        amps = [1.0, 0.5, 0.35]
        sig = np.zeros_like(t)
        for f, a in zip(parts, amps):
            sig += a * np.sin(2 * np.pi * f * t)
        sig *= _adsr(n_samples, sr, 0.01, 0.02)

    elif timbre == "bell":
        # 有音高的钟声：以基频为主 + 少量非谐泛音（金属感），较长衰减
        parts = [freq, 2.0 * freq, 3.01 * freq, 4.2 * freq]
        amps = [1.0, 0.5, 0.3, 0.15]
        sig = np.zeros_like(t)
        for f, a in zip(parts, amps):
            sig += a * np.sin(2 * np.pi * f * t) * np.exp(-t / (1.0 * dur))
        sig *= _adsr(n_samples, sr, 0.002, 0.05)

    elif timbre == "human_voice":
        # 元音 /a/：锯齿声源(含强基频，保证音高可追踪) + 共振峰着色
        vibrato = 1 + 0.008 * np.sin(2 * np.pi * 5.5 * t)
        src = _add_harmonics(freq, t * vibrato, [1.0 / k for k in range(1, 18)])
        voiced = src.copy()
        for ff, amp in [(800, 0.25), (1150, 0.18), (2900, 0.08)]:
            voiced = voiced + amp * np.sin(2 * np.pi * ff * t)
        sig = voiced
        sig *= _adsr(n_samples, sr, 0.04, 0.05)
    else:
        # 默认纯正弦
        sig = np.sin(2 * np.pi * freq * t)

    # 归一化
    peak = np.max(np.abs(sig)) + 1e-9
    return (sig / peak).astype(np.float32)


def render_melody(seq: List[Tuple[int, float]], timbre: str,
                  sr: int = 16000) -> Tuple[np.ndarray, List[int]]:
    """渲染整首旋律；返回 (波形, 期望音高 MIDI 列表)。"""
    chunks = []
    expected = []
    for midi, beats in seq:
        n_samp = int(round(beats * BEAT * sr))
        gap = int(round(GAP * sr))
        if midi > 0:
            freq = midi2freq(midi)
            chunks.append(render_note(freq, n_samp, sr, timbre))
            expected.append(midi)
        else:
            chunks.append(np.zeros(n_samp, dtype=np.float32))
        if gap > 0:
            chunks.append(np.zeros(gap, dtype=np.float32))
    y = np.concatenate(chunks) if chunks else np.zeros(1, dtype=np.float32)
    # 注入底噪模拟录音
    if NOISE_SNR < 120:
        sig_power = float(np.mean(y ** 2)) + 1e-12
        noise_power = sig_power / (10 ** (NOISE_SNR / 10))
        y = y + np.random.randn(len(y)).astype(np.float32) * np.sqrt(noise_power)
    y = y / (np.max(np.abs(y)) + 1e-9)
    return y.astype(np.float32), expected


def build_manifest(sr: int = 16000) -> List[Dict]:
    """生成所有 (旋律 × 音色) 样例的元数据清单。"""
    manifest = []
    for mi, (zh, en, seq) in enumerate(MELODIES):
        for cat, timbres in CATEGORY_TIMBRES.items():
            for ti, timbre in enumerate(timbres):
                mid = f"m{mi:02d}_{cat}_{timbre}"
                manifest.append({
                    "id": mid,
                    "melody_index": mi,
                    "title_zh": zh,
                    "title_en": en,
                    "category": cat,
                    "timbre": timbre,
                    "sr": sr,
                    "file": f"audio/{mid}.wav",
                    "expected_midi": [m for m, _ in seq if m > 0],
                })
    return manifest


def generate_all(out_dir: str, sr: int = 16000) -> List[Dict]:
    """合成全部音频并写出 manifest.json。返回 manifest。"""
    import soundfile as sf
    audio_dir = os.path.join(out_dir, "audio")
    os.makedirs(audio_dir, exist_ok=True)
    manifest = build_manifest(sr)
    for item in manifest:
        seq = MELODIES[item["melody_index"]][2]
        y, expected = render_melody(seq, item["timbre"], sr)
        item["expected_midi"] = expected
        sf.write(os.path.join(out_dir, item["file"]), y, sr)
    with open(os.path.join(out_dir, "audio", "manifest.json"), "w", encoding="utf-8") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=2)
    return manifest


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    m = generate_all(here)
    print(f"已生成 {len(m)} 个样例音频 -> {os.path.join(here, 'audio')}")
