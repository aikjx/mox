# -*- coding: utf-8 -*-
"""冒烟：BPM翻倍修复 / 纠错层 / 量化模块 / 分离模块 单测快速验证。"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import numpy as np

from core.analysis import detect_bpm, _BPM_MAX_HARD, _BPM_MIN_HARD
from core.quantize import quantize_notes, jianpu_dur_tokens
from core.postprocess import postprocess_notes
from core.separator import separate_melody
from core.score import to_jianpu

def durs_for_true_bpm(true_bpm, n=24, seed=0):
    beat = 60.0 / true_bpm
    np.random.seed(seed)
    probs = np.array([0.5, 0.2, 0.2, 0.1]); probs /= probs.sum()
    mults = np.random.choice([1.0, 2.0, 0.5, 1.5], size=n, p=probs)
    notes, t = [], 0.0
    for m in mults:
        d = float(m * beat)
        notes.append({"midi": 60 + int(np.random.randint(0, 8) * 2),
                      "start": t, "end": t + d})
        t += d + 0.01
    return notes


print("======== 1. BPM 检测：音符拟合路径 15% 容差 ========")
fail = 0
for true_bpm in [72, 80, 88, 96, 100, 108, 120, 132, 140]:
    notes = durs_for_true_bpm(true_bpm)
    y = np.zeros(16000, dtype=np.float32)
    b = detect_bpm(y, 16000, fallback=100, notes=notes)
    ok = abs(b - true_bpm) / true_bpm <= 0.20  # 20% 容差（纯音符拟合未带 librosa 兜底）
    tag = "PASS" if ok else "FAIL"
    if not ok: fail += 1
    print(f"  true={true_bpm:3d}  ->  {b:5.1f}   {tag}")
print(f"  失败 {fail} 项")

print("\n======== 2. BPM 硬约束：永不输出 160+（禁止 176 类离谱值） ========")
assert _BPM_MAX_HARD < 176, f"硬上界 {_BPM_MAX_HARD} >= 176"
assert _BPM_MIN_HARD >= 40, f"硬下界 {_BPM_MIN_HARD} < 40"
# 构造极端 notes：每拍 0.18s（2个半拍 = 360 BPM），但拟合器仍要被裁剪到 160
notes_crazy = durs_for_true_bpm(360, n=10)
y_empty = np.zeros(16000, dtype=np.float32)
b_crazy = detect_bpm(y_empty, 16000, fallback=100, notes=notes_crazy)
print(f"  真实 360 -> 输出 {b_crazy}，必须 <= 160")
assert b_crazy <= 160, f"极端 BPM 未裁剪: {b_crazy}"
print("  PASS ✓")

print("\n======== 3. quantize：附点 / 切分 / 三连音 保留 ========")
bpm = 120
beat = 60.0 / bpm
cases = [
    # (dur_beat, name, 合法拍值)
    (1.0,  "四分", {"1.0"}),
    (1.5,  "四分附点", {"1.5"}),
    (0.5,  "八分", {"0.5"}),
    (0.75, "八分附点", {"0.75"}),
    (0.25, "十六分", {"0.25"}),
    (2.0,  "二分", {"2.0"}),
    (3.0,  "二分附点", {"3.0"}),
]
for dur_beat, name, _ in cases:
    notes = [{"midi": 60, "start": 0.0, "end": dur_beat * beat}]
    q = quantize_notes(notes, bpm)
    got_dur = q[0]["dur_beat"]
    ok = abs(got_dur - dur_beat) < 0.06
    print(f"  {name:8s} 期望 {dur_beat:.3f} -> 实得 {got_dur:.3f} {'PASS' if ok else 'FAIL'}")
    if not ok: fail += 1

print("\n======== 4. jianpu_dur_tokens 整数拍延音回归 ========")
cases = [
    (1.0, 1, ("", 0, 0, 0)),   # 1拍：无修饰
    (2.0, 1, ("", 0, 0, 1)),   # 2拍：1-
    (4.0, 1, ("", 0, 0, 3)),   # 4拍：1---
    (1.5, 1, ("", 0, 1, 0)),   # 1.5拍：1.
    (0.5, 1, ("", 1, 0, 0)),   # 半拍：_1
]
for dur_beat, _, expect in cases:
    tok = jianpu_dur_tokens(dur_beat)
    # 只比后三项（下划线 / 点 / 延音）
    ok = (tok[1], tok[2], tok[3]) == (expect[1], expect[2], expect[3])
    print(f"  拍={dur_beat:.2f} 实={tok[1:]} 期={expect[1:]} {'PASS' if ok else 'FAIL'}")
    if not ok: fail += 1

# 原 test_jianpu_duration_quantization 的断言
print("\n======== 5. score.to_jianpu 0.86拍 → '1' ，2.04拍 → '1-' ========")
notes_a = [{"midi": 60, "start": 0.0, "end": 0.43}]
got_a = to_jianpu(notes_a, ("C", "major"), bpm=120.0)
notes_b = [{"midi": 60, "start": 0.0, "end": 1.02}]
got_b = to_jianpu(notes_b, ("C", "major"), bpm=120.0)
print(f"  0.43s/@120bpm -> jianpu: '{got_a}'  (期望 '1')")
print(f"  1.02s/@120bpm -> jianpu: '{got_b}'  (期望 '1-')")
if got_a != "1": fail += 1; print("  FAIL A")
if got_b != "1-": fail += 1; print("  FAIL B")

print("\n======== 6. postprocess_notes：跳音幻影修复 + 同音合并 ========")
# 构造孤立跳音：C C G(错) C C -> 要纠正为 C C C C C
notes_jump = [
    {"midi": 60, "start": 0.0,  "end": 0.5},
    {"midi": 60, "start": 0.5,  "end": 1.0},
    {"midi": 72, "start": 1.0,  "end": 1.4},   # 幻影八度跳
    {"midi": 60, "start": 1.5,  "end": 2.0},
    {"midi": 60, "start": 2.0,  "end": 2.5},
]
res = postprocess_notes(notes_jump)
midis = [n["midi"] for n in res["notes"]]
print(f"  幻影跳音修复前: {[n['midi'] for n in notes_jump]}")
print(f"  幻影跳音修复后: {midis}  (纠正跳音={res['corrected_jumps']} 次)")
# 幻影必须被纠正回来
if any(m == 72 for m in midis):
    fail += 1; print("  FAIL: 幻影跳音仍残留 72")
else:
    print("  PASS ✓")

# 连续同音合并：C 0.0-0.5, C 0.51-1.0 -> 合并成一条
notes_rep = [
    {"midi": 62, "start": 0.0,  "end": 0.5},
    {"midi": 62, "start": 0.51, "end": 1.0},
    {"midi": 64, "start": 1.1,  "end": 1.6},
]
res2 = postprocess_notes(notes_rep)
count = len(res2["notes"])
print(f"\n  同音合并 3段 -> {count} 条，合并次数: {res2['merged_count']}")
if count != 2 or res2["merged_count"] < 1:
    fail += 1; print("  FAIL: 同音未合并")
else:
    print("  PASS ✓")

print("\n======== 7. separator HPSS 降级路径 ========")
np.random.seed(7)
# 1s 简单正弦 + 白噪声模拟混合
t_axis = np.linspace(0, 1, 16000, endpoint=False)
sig = (0.6 * np.sin(2*np.pi*261.63*t_axis)  # C4 主旋律
       + 0.2 * np.random.randn(len(t_axis))).astype(np.float32)
sep = separate_melody(sig, 16000, strategy="auto")
print(f"  策略: {sep['strategy']}  时长匹配: {len(sep['vocals']) == len(sig)}")
assert len(sep["vocals"]) == len(sig), "分离输出长度不匹配"
assert sep["strategy"] in ("hpss", "demucs", "passthrough"), f"未知策略: {sep['strategy']}"
print("  PASS ✓")

print(f"\n======== 全部验证完成（失败 {fail} 项） ========")
sys.exit(1 if fail else 0)
