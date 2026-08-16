# -*- coding: utf-8 -*-
"""编排层：串联 采集→预处理→音高→解析→歌谱→输出。"""
import os
from typing import Dict, Optional

from .config import Config
from . import capture, preprocess, pitch, analysis, score


class Melody2Score:
    def __init__(self, config: Optional[Config] = None):
        self.cfg = config or Config()
        self.detector = pitch.PitchDetector(
            self.cfg.model_size, self.cfg.conf_thresh,
            self.cfg.hop, self.cfg.intra_op_threads)

    def run(self, audio_path: Optional[str] = None, record_seconds: int = 0,
            out_xml: Optional[str] = None, ms_score: Optional[str] = None,
            device: Optional[int] = None) -> Dict:
        # 1) 采集
        if audio_path:
            y = capture.load_audio(audio_path, self.cfg.sr)
        else:
            y = capture.record(record_seconds or 5, self.cfg.sr, device=device)
        # 2) 预处理
        y = preprocess.preprocess(y, self.cfg.sr, self.cfg.enable_denoise)
        # 3) 音高检测
        pts = self.detector.detect(y, self.cfg.sr)
        # 4) 音乐解析：音符分割 + BPM + 调式
        notes = analysis.segment_notes(pts, self.cfg.min_note_dur, self.cfg.median_win)
        bpm = analysis.detect_bpm(y, self.cfg.sr, self.cfg.bpm_fallback)
        key_name = analysis.estimate_key(y, self.cfg.sr)
        # 5) 歌谱生成
        s = score.to_musicxml(notes, bpm, key_name, out_xml)
        jianpu = score.to_jianpu(notes, key_name, bpm)
        # 6) 输出（可选 MuseScore 出图）
        if out_xml and ms_score:
            os.system(f'"{ms_score}" --export-to "{out_xml}.png" "{out_xml}"')

        return {
            "stream": s, "jianpu": jianpu, "notes": notes,
            "bpm": bpm, "key": key_name,
        }

    def print_summary(self, res: Dict):
        print("简谱：", res["jianpu"])
        print(f"[info] BPM={res['bpm']:.1f}  Key={res['key'][0]}{res['key'][1]}  "
              f"音符数={len(res['notes'])}")
