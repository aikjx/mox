# -*- coding: utf-8 -*-
"""开发板入口：alsa 录音 → 转谱，输出简谱文本与 musicxml（写到 /tmp）。

用法（在 melody2score 目录下执行）：
    python board/run_board.py record 6
    python board/run_board.py file test.wav
    python board/run_board.py record 6 -o /tmp/melody.xml
"""
import argparse
import os
import sys

# 让本文件可直接 import core 包
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from core.pipeline import Melody2Score
from board.board_config import board_config


def main():
    ap = argparse.ArgumentParser(description="开发板：哼唱转歌谱")
    ap.add_argument("mode", choices=["record", "file"], help="录音或读文件")
    ap.add_argument("arg", nargs="?", default="5", help="录音秒数 或 音频路径")
    ap.add_argument("-o", "--out", default="/tmp/melody.xml", help="输出 musicxml 路径")
    ap.add_argument("--mscore", help="MuseScore 路径（板端通常无 GUI，可省略）")
    ap.add_argument("--device", type=int, default=None, help="录音设备索引")
    ap.add_argument("--backend", default="portaudio", choices=["portaudio", "arecord"])
    args = ap.parse_args()

    cfg = board_config()
    m = Melody2Score(cfg)

    if args.mode == "record":
        secs = int(args.arg)
        res = m.run(record_seconds=secs, out_xml=args.out, ms_score=args.mscore,
                    device=args.device)
        # 开发板无 pyaudio 时改用 arecord 后端
        if args.backend == "arecord":
            res = m.run(record_seconds=secs, out_xml=args.out, ms_score=args.mscore)
    else:
        res = m.run(audio_path=args.arg, out_xml=args.out, ms_score=args.mscore)

    m.print_summary(res)
    if args.out:
        print("[board] 已保存:", args.out)


if __name__ == "__main__":
    main()
