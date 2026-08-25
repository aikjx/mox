# -*- coding: utf-8 -*-
"""端到端自测（企业级严格断言）：合成旋律 → 跑流水线 → 精确恢复验证。

覆盖维度：
  1. 音符序列精确匹配（数量 + 音高逐一对应，不允许丢音/加音）
  2. 时值精确性（1 拍 / 2 拍音符落在容差窗口内）
  3. BPM 精确反推（不再落 fallback，不走昂贵的 beat_track 兜底）
  4. 调式识别正确（C 大调）
  5. 简谱文本正确（数字唱名 + 整数拍延音）
  6. 连续同音可分（62-62、60-60 等，回归共识簇污染 bug）

运行：
    pytest tests/test_pipeline.py -q
或直接：
    python tests/test_pipeline.py
"""
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from core.pipeline import Melody2Score
from core.config import Config

EXPECTED = [60, 60, 67, 67, 69, 69, 67, 65, 65, 64, 64, 62, 62, 60]
EXPECTED_BEATS = [1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 2]
EXPECTED_JIANPU = "1 1 5 5 6 6 5- 4 4 3 3 2 2 1-"


def _ensure_audio():
    base = os.path.dirname(os.path.abspath(__file__))
    wav = os.path.join(base, "twinkle.wav")
    if not os.path.exists(wav):
        import subprocess
        subprocess.run([sys.executable, os.path.join(base, "gen_test_audio.py")], check=True)
    return wav


def _run(audio):
    cfg = Config()
    cfg.enable_denoise = False    # 合成音频本就干净，关降噪加速
    cfg.enable_separation = False # twinkle.wav 是干净纯钢琴合成的单音旋律，
                                  # 已经是分离好的主旋律，再走 HPSS 会伤谐波
                                  # （真实用户 mp3 的带伴奏人声则让默认 None
                                  #  跟随 vocal_mode=True 自动开分离）。
    cfg.model_size = "tiny"       # v1 黄金配置：tiny 模型 + 置信阈值 0.30
    cfg.conf_thresh = 0.30
    m = Melody2Score(cfg)
    return m.run(audio_path=audio)


def test_twinkle_pipeline():
    audio = _ensure_audio()
    res = _run(audio)

    got = [n["midi"] for n in res["notes"]]
    durs = [n["dur"] for n in res["notes"]]
    print("\n[test] 恢复 MIDI:", got)
    print("[test] 音符时长:", [round(d, 2) for d in durs])
    print("[test] 简谱:", res["jianpu"])
    print(f"[test] BPM={res['bpm']:.1f} Key={res['key']}")

    # 1) 音符序列精确匹配：不允许丢音（含连续同音 62-62 / 60-60）
    assert got == EXPECTED, f"音符序列不匹配:\n  期望 {EXPECTED}\n  实际 {got}"

    # 2) 时值精确性：1 拍音符 0.3~0.7s，2 拍音符 0.8~1.3s（VAD 切边容差）
    for d, b in zip(durs, EXPECTED_BEATS):
        if b == 1:
            assert 0.3 <= d <= 0.7, f"1 拍音符时值异常: {d}s"
        else:
            assert 0.8 <= d <= 1.3, f"2 拍音符时值异常: {d}s"

    # 3) BPM 精确反推（真值 120；不得落 fallback 之外的离谱值）
    assert abs(res["bpm"] - 120.0) <= 15.0, f"BPM 偏差过大: {res['bpm']}"

    # 4) 调式正确
    assert res["key"] == {"tonic": "C", "mode": "major"}, f"调式错误: {res['key']}"

    # 5) 简谱文本正确（整数拍延音 + 首调唱名）
    assert res["jianpu"] == EXPECTED_JIANPU, \
        f"简谱不匹配:\n  期望 {EXPECTED_JIANPU}\n  实际 {res['jianpu']}"

    # 6) 置信度与稳健共识健康
    assert res["confidence"] >= 0.9, f"置信度过低: {res['confidence']}"
    assert res["robust_kept"] == len(EXPECTED), \
        f"共识保留数异常: {res['robust_kept']} (期望 {len(EXPECTED)})"


def test_median_filter_nan_isolation():
    """单元回归：中值滤波不得让静音 NaN 传染邻域有效帧。

    修复前的行为：np.median(含 NaN 窗) → NaN，音符被砍头去尾。
    """
    from core.analysis import _median_filter
    x = np.array([60.0, 60.0, 60.0, np.nan, 62.0, 62.0, 62.0])
    out = _median_filter(x, 7)
    # NaN 两侧的有效帧必须保留原值（不得被传染成 NaN）
    assert out[0] == 60.0 and out[1] == 60.0 and out[2] == 60.0, f"左侧传染: {out}"
    assert out[4] == 62.0 and out[5] == 62.0 and out[6] == 62.0, f"右侧传染: {out}"
    assert np.isnan(out[3]), "静音空洞应保持 NaN"


def test_consensus_no_note_swallow():
    """单元回归：跨音符长音不得把相邻同音吞并进同一簇。

    构造：run0 把 62-62 吞成一条长音（吞并型识别错误），
    run1/run2 正确分为两条。共识输出必须是两条 62。
    """
    from core.pipeline import _consensus
    from core.config import Config
    cfg = Config()
    runs = [
        # run0：吞并错误——一条 1.09s 长音
        [{"midi": 62, "start": 7.0, "end": 8.09}],
        # run1：正确——两条 0.5s
        [{"midi": 62, "start": 7.0, "end": 7.5},
         {"midi": 62, "start": 7.59, "end": 8.09}],
        # run2：正确——两条 0.5s
        [{"midi": 62, "start": 7.0, "end": 7.5},
         {"midi": 62, "start": 7.59, "end": 8.09}],
    ]
    merged, info = _consensus(runs, cfg)
    assert len(merged) == 2, f"连续同音被吞并: {merged}"
    # 边界取中位数，不受 run0 长音污染
    assert abs(merged[0]["end"] - 7.5) < 0.05, f"簇边界被长音拉长: {merged[0]}"
    assert abs(merged[1]["start"] - 7.59) < 0.05, f"簇起点异常: {merged[1]}"


def test_jianpu_duration_quantization():
    """单元回归：文本简谱时值量化为整数拍（0.86 拍 → 1 拍而非 3 拍）。"""
    from core.score import to_jianpu
    notes = [{"midi": 60, "start": 0.0, "end": 0.43}]  # 0.43s ≈ 0.86 拍 @120bpm
    out = to_jianpu(notes, ("C", "major"), bpm=120.0)
    assert out == "1", f"0.86 拍应量化为 1 拍（'1'），实际: '{out}'"
    notes2 = [{"midi": 60, "start": 0.0, "end": 1.02}]  # 2.04 拍
    out2 = to_jianpu(notes2, ("C", "major"), bpm=120.0)
    assert out2 == "1-", f"2.04 拍应量化为 2 拍（'1-'），实际: '{out2}'"


if __name__ == "__main__":
    # 无 pytest 时也能直接跑
    audio = _ensure_audio()
    res = _run(audio)
    Melody2Score.print_summary(res)
    print("恢复 MIDI:", [n["midi"] for n in res["notes"]])
    print("\n运行全部断言...")
    test_twinkle_pipeline()
    test_median_filter_nan_isolation()
    test_consensus_no_note_swallow()
    test_jianpu_duration_quantization()
    print("\n全部测试通过 ✓")
