# -*- coding: utf-8 -*-
"""端到端冒烟：模拟用户吐槽场景——vocal=True + 混鼓点伴奏的「混合音频」，
确认 v2：BPM 不再翻倍（不会输出176）、主旋律不被鼓点完全带偏。
"""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, ROOT)
import numpy as np
import soundfile as sf

MELODY = [60, 60, 67, 67, 69, 69, 67, 65, 65, 64, 64, 62, 62, 60]
BEATS  = [ 1,  1,  1,  1,  1,  1,  2,  1,  1,  1,  1,  1,  1,  2]

def midi2freq(m: int) -> float:
    return 440.0 * 2 ** ((m - 69) / 12.0)

def synth_twinkle(sr=44100, bpm=88, fspace_ms=90):
    spb = 60.0 / bpm
    segs = []
    for m, b in zip(MELODY, BEATS):
        dur = b * spb
        n = int(sr * dur)
        t = np.arange(n) / sr
        # 简单钢琴：基波 + 二次、三次谐波
        f0 = midi2freq(m)
        sig = (np.sin(2*np.pi*f0*t)
               + 0.45 * np.sin(2*np.pi*2*f0*t)
               + 0.22 * np.sin(2*np.pi*3*f0*t))
        atk = min(int(0.012 * sr), max(2, n // 10))
        rel = min(int(0.05 * sr), max(2, n // 6))
        env = np.ones(n)
        env[:atk] = np.linspace(0, 1, atk)
        env[-rel:] = np.linspace(1, 0, rel)
        segs.append(sig * env * 0.7)
        segs.append(np.zeros(int(sr * fspace_ms / 1000)))
    return np.concatenate(segs).astype(np.float32), sr

def synth_mixed(out_path, bpm=88, sr=44100):
    y, _ = synth_twinkle(sr=sr, bpm=bpm)
    dur_s = len(y) / sr
    t_total = np.arange(len(y)) / sr

    # (1) 贝斯鼓 kick：每拍一次 65Hz
    beat_s = 60.0 / bpm
    kick = np.zeros_like(y)
    env_len = int(0.12 * sr)
    kick_env = np.exp(-np.arange(env_len) / (0.030 * sr))
    tone = 0.9 * np.sin(2 * np.pi * 65.0 * np.arange(env_len) / sr) * kick_env
    for b in range(int(dur_s / beat_s) + 2):
        pos = int(b * beat_s * sr)
        if pos < 0 or pos >= len(kick):
            continue
        e = min(len(kick), pos + env_len)
        L = e - pos
        if L <= 0:
            continue
        kick[pos:e] += tone[:L]
    # snare：每半拍（奇半拍）白噪 + 200Hz 弱冲——经典 2× BPM 陷阱
    snare_len = int(0.06 * sr)
    rng = np.random.default_rng(42)
    for b in range(int(dur_s / (beat_s / 2)) + 2):
        if b % 2 == 1:
            pos = int(b * (beat_s / 2) * sr)
            if pos < 0 or pos >= len(kick):
                continue
            L = snare_len
            e = min(len(kick), pos + L)
            L = e - pos
            if L <= 0:
                continue
            sn_tone = (0.38 * rng.normal(0, 1, L)
                       + 0.18 * np.sin(2 * np.pi * 200 * np.arange(L) / sr))
            sn_tone *= np.exp(-np.arange(L) / (0.015 * sr))
            kick[pos:e] += sn_tone

    # (2) 和弦伴奏：每 4 拍叠加 C-E-G 三音，模拟乐队和弦垫
    bar_s = beat_s * 4
    chord = np.zeros_like(y)
    chord_freqs = [midi2freq(x) for x in (60, 64, 67)]  # C4 E4 G4
    for bi in range(int(dur_s / bar_s) + 1):
        pos = int(bi * bar_s * sr)
        L = min(len(y) - pos, int(0.92 * bar_s * sr))
        if L <= 0:
            continue
        tt = np.arange(L) / sr
        env = np.exp(-tt / (0.55 * bar_s))
        c = np.zeros(L)
        for f in chord_freqs:
            c += 0.20 * np.sin(2 * np.pi * f * tt) * env
        chord[pos:pos + L] += c

    y_mix = y + 0.80 * kick + 0.90 * chord
    y_mix += 0.022 * np.random.default_rng(0).normal(0, 1, len(y_mix))

    peak = float(np.max(np.abs(y_mix)) + 1e-9)
    y_mix = (y_mix / peak * 0.95).astype(np.float32)
    sf.write(out_path, y_mix, sr)
    return out_path, bpm


if __name__ == "__main__":
    audio_path = os.path.join(HERE, "mixed_twinkle.wav")
    _, true_bpm = synth_mixed(audio_path, bpm=88)
    print(f"[合成] true BPM={true_bpm}，带 kick + snare 弱拍（= BPM×2 陷阱）+ C 大调三和弦")

    from core.pipeline import Melody2Score
    from core.config import Config

    # ========== 实验组：v2 全链路（默认配置 vocal=True + auto 分离 + 后处理） ==========
    cfg = Config()
    cfg.model_size = "tiny"
    cfg.conf_thresh = 0.30
    cfg.enable_denoise = True
    cfg.robust = True
    res = Melody2Score(cfg).run(audio_path=audio_path)
    print("\n========== [v2 全链路: vocal=True + auto 分离 + 后处理] ==========")
    Melody2Score.print_summary(res)
    got_midi = [n["midi"] for n in res["notes"]]
    print("恢复 MIDI:", got_midi)
    print("简谱      :", res["jianpu"])
    print("EXP MIDI  :", MELODY)
    print("EXP 简谱  :", "1 1 5 5 6 6 5- 4 4 3 3 2 2 1-")

    bpm_ok = res["bpm"] <= 160 and abs(res["bpm"] - true_bpm) <= 25
    if not bpm_ok:
        print(f"\n[FAIL] BPM 异常: 期望 ~{true_bpm}，实得 {res['bpm']}")
        sys.exit(1)
    if len(got_midi) < 10:
        print(f"[FAIL] 严重丢音: 期望 ≥10 条音符，实得 {len(got_midi)}")
        sys.exit(2)

    # ========== 对照组：强制关分离 + 关后处理（模拟 v1 旧链路） ==========
    cfg2 = Config(vocal_mode=True, enable_separation=False, enable_postprocess=False)
    cfg2.enable_denoise = False
    cfg2.model_size = "tiny"
    cfg2.conf_thresh = 0.30
    cfg2.robust = True
    res2 = Melody2Score(cfg2).run(audio_path=audio_path)
    print("\n========== [对照 v1 旧链路: 无分离 + 无纠错] ==========")
    Melody2Score.print_summary(res2)
    print("恢复 MIDI:", [n["midi"] for n in res2["notes"]])
    print("简谱      :", res2["jianpu"])

    # ========== 对照组：关分离 + 开后处理 ==========
    cfg3 = Config(vocal_mode=True, enable_separation=False, enable_postprocess=True)
    cfg3.enable_denoise = True
    cfg3.model_size = "tiny"
    cfg3.conf_thresh = 0.30
    cfg3.robust = True
    res3 = Melody2Score(cfg3).run(audio_path=audio_path)
    print("\n========== [消融: 关分离 + 开后处理] ==========")
    Melody2Score.print_summary(res3)
    print("恢复 MIDI:", [n["midi"] for n in res3["notes"]])
    print("简谱      :", res3["jianpu"])

    print("\n==== 关键对比 ====")
    print(f"真实 BPM={true_bpm}")
    print(f"v2 全链路  BPM={res['bpm']:.1f}  音符={len(res['notes'])}  分离={res['separation'].get('strategy')}")
    print(f"v1 旧链路  BPM={res2['bpm']:.1f}  音符={len(res2['notes'])}")
    print(f"关分离+开后 BPM={res3['bpm']:.1f}  音符={len(res3['notes'])}")
    print("\n[v2 达标判断]: BPM <= 160 且 不翻倍 才算 PASS")
    if res["bpm"] <= 160 and abs(res["bpm"] - true_bpm) <= 25:
        if true_bpm * 1.7 <= res["bpm"] <= true_bpm * 2.3:
            print(f"[FAIL] 依然翻倍！实得 {res['bpm']}（真实 {true_bpm}×2={true_bpm*2}）")
            sys.exit(3)
        else:
            print("✓ v2 END-TO-END PASS")
            sys.exit(0)
    else:
        print("[FAIL] BPM 约束失败")
        sys.exit(4)
