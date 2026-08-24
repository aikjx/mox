# -*- coding: utf-8 -*-
"""真实识别：对经典旋律数据集跑 core 流水线，采集音高恢复精度。

调用真实 core 层（采集→预处理→音高检测[crepe_onnx/torchcrepe/pyin]→解析），
把识别出的音符序列与 ground truth（manifest.expected_midi）比对，
输出每首的 音高类准确率 / 音符召回率 / 音符精确率，以及按类别、音色的聚合统计。

结果写入 results/classic_results.json。
"""
import json
import os
import sys
from typing import Dict, List

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from core.config import Config
from core import capture, preprocess, pitch, analysis
import classic_corpus as corpus

# 可通过环境变量选择音高后端：crepe_onnx / torchcrepe / pyin / auto（默认 auto）
# 注：本沙箱 torchcrepe 导入段错误，故示例运行用 pyin（librosa 概率化 YIN，真实音高估计）。
BACKEND = os.environ.get("MELODY_BACKEND", "auto")

BEAT = corpus.BEAT
GAP = corpus.GAP
ONSET_TOL = 0.35 * BEAT  # 对齐容差（秒）


def expected_onsets(seq) -> List[float]:
    onsets = []
    t = 0.0
    for midi, beats in seq:
        if midi > 0:
            onsets.append(t)
        t += beats * BEAT + GAP
    return onsets


def align(expected_midi, expected_on, recovered):
    """贪心按起始时间对齐，统计精确匹配与音高类匹配。"""
    rec_by_onset = sorted(recovered, key=lambda x: x["start"])
    matched = 0
    pc_matched = 0
    used = set()
    for midi, eo in zip(expected_midi, expected_on):
        best_i, best_d = None, 1e9
        for i, r in enumerate(rec_by_onset):
            if i in used:
                continue
            d = abs(r["start"] - eo)
            if d < best_d:
                best_d, best_i = d, i
        if best_i is not None and best_d <= ONSET_TOL:
            r = rec_by_onset[best_i]
            used.add(best_i)
            if r["midi"] == midi:
                matched += 1
                pc_matched += 1
            elif (r["midi"] % 12) == (midi % 12):
                pc_matched += 1
    n_exp = len(expected_midi)
    n_rec = len(rec_by_onset)
    return {
        "n_expected": n_exp,
        "n_recovered": n_rec,
        "matched": matched,
        "pc_matched": pc_matched,
        "pitch_class_acc": pc_matched / n_exp if n_exp else 0.0,
        "note_recall": matched / n_exp if n_exp else 0.0,
        "note_precision": matched / n_rec if n_rec else 0.0,
    }


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    manifest_path = os.path.join(here, "audio", "manifest.json")
    if not os.path.exists(manifest_path):
        print("未找到 audio/manifest.json，请先运行 gen_classic_melodies.py")
        sys.exit(1)
    with open(manifest_path, encoding="utf-8") as f:
        manifest = json.load(f)

    cfg = Config()
    det = pitch.PitchDetector(
        model_size=cfg.model_size, conf_thresh=cfg.conf_thresh, hop=cfg.hop,
        intra_op_threads=cfg.intra_op_threads, backend=BACKEND, sr=cfg.sr)

    results = []
    for item in manifest:
        path = os.path.join(here, item["file"])
        try:
            y = capture.load_audio(path, cfg.sr)
        except Exception as e:
            print(f"[warn] 加载失败 {item['id']}: {e}")
            continue
        y = preprocess.preprocess(y, cfg.sr, cfg.enable_denoise)
        pts = det.detect(y, cfg.sr)
        notes = analysis.segment_notes(pts, cfg.min_note_dur, cfg.median_win)
        recovered = [{"midi": int(nt["midi"]), "start": float(nt["start"]),
                      "end": float(nt["end"])} for nt in notes]
        seq = corpus.MELODIES[item["melody_index"]][2]
        exp_midi = item["expected_midi"]
        exp_on = expected_onsets(seq)
        metrics = align(exp_midi, exp_on, recovered)
        results.append({
            "id": item["id"],
            "title_zh": item["title_zh"],
            "title_en": item["title_en"],
            "category": item["category"],
            "timbre": item["timbre"],
            "expected_midi": exp_midi,
            "recovered_midi": [r["midi"] for r in recovered],
            **metrics,
        })
        print(f"{item['id']:<26} {item['title_zh']:<8} "
              f"acc={metrics['pitch_class_acc']*100:5.1f}% "
              f"recall={metrics['note_recall']*100:5.1f}% "
              f"n={metrics['n_recovered']}/{metrics['n_expected']}")

    # 聚合统计
    def aggregate(key_fn):
        agg = {}
        for r in results:
            k = key_fn(r)
            agg.setdefault(k, []).append(r)
        out = {}
        for k, rs in agg.items():
            out[k] = {
                "count": len(rs),
                "mean_pitch_class_acc": float(np.mean([x["pitch_class_acc"] for x in rs])),
                "mean_note_recall": float(np.mean([x["note_recall"] for x in rs])),
                "mean_note_precision": float(np.mean([x["note_precision"] for x in rs])),
            }
        return out

    summary = {
        "backend": det.used_backend,
        "total": len(results),
        "overall": {
            "mean_pitch_class_acc": float(np.mean([x["pitch_class_acc"] for x in results])),
            "mean_note_recall": float(np.mean([x["note_recall"] for x in results])),
            "mean_note_precision": float(np.mean([x["note_precision"] for x in results])),
        },
        "by_category": aggregate(lambda r: r["category"]),
        "by_timbre": aggregate(lambda r: r["timbre"]),
    }

    out_dir = os.path.join(here, "results")
    os.makedirs(out_dir, exist_ok=True)
    with open(os.path.join(out_dir, "classic_results.json"), "w", encoding="utf-8") as f:
        json.dump({"summary": summary, "items": results}, f, ensure_ascii=False, indent=2)

    print("\n=== 汇总（后端: %s）===" % det.used_backend)
    print(f"总样例: {summary['total']}  平均音高类准确率: {summary['overall']['mean_pitch_class_acc']*100:.1f}%  "
          f"平均音符召回: {summary['overall']['mean_note_recall']*100:.1f}%")
    print("\n按类别:")
    for k, v in summary["by_category"].items():
        print(f"  {k:<10} n={v['count']:<3} acc={v['mean_pitch_class_acc']*100:5.1f}% "
              f"recall={v['mean_note_recall']*100:5.1f}%")
    print("\n按音色:")
    for k, v in summary["by_timbre"].items():
        print(f"  {k:<12} n={v['count']:<3} acc={v['mean_pitch_class_acc']*100:5.1f}% "
              f"recall={v['mean_note_recall']*100:5.1f}%")


if __name__ == "__main__":
    main()
