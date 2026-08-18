#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
melody2score_demo.py —— PC 验证原型入口（支持 mp3 直接输入）

分层实现见 core/ 包；本文件只做参数解析与编排调用。
流水线：采集 → 预处理 → 音高(Crepe-ONNX tiny) → 节拍/调式 → 音符解析 → musicxml+简谱

依赖：
    pip install -r requirements.txt
运行：
    python melody2score_demo.py your_song.mp3
    python melody2score_demo.py your_song.mp3 -o out.xml
    python melody2score_demo.py -r 5            # 现场录音 5 秒
"""
import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from core.pipeline import Melody2Score
from core.config import Config


def main():
    ap = argparse.ArgumentParser(description="哼唱旋律转歌谱 Demo（支持 mp3 直接输入）")
    ap.add_argument("input", nargs="?", help="mp3/wav 音频文件路径")
    ap.add_argument("-o", "--out", help="输出 musicxml 路径")
    ap.add_argument("-r", "--record", type=int, default=0, help="现场录音秒数（替代 input）")
    ap.add_argument("--mscore", help="MuseScore 可执行路径，导出 png 用")
    ap.add_argument("--model", default="tiny", choices=["tiny", "small", "full"],
                    help="crepe_onnx 模型大小")
    ap.add_argument("--no-denoise", action="store_true", help="关闭谱减降噪")
    ap.add_argument("--threads", type=int, default=0, help="onnxruntime 单算子线程数")
    ap.add_argument("--no-robust", action="store_true", help="关闭稳健重识别共识（单次识别）")
    args = ap.parse_args()

    if not args.input and not args.record:
        ap.print_help()
        sys.exit(0)

    cfg = Config()
    cfg.model_size = args.model
    cfg.enable_denoise = not args.no_denoise
    cfg.preferred_backend = "crepe_onnx"  # 首选首选：crepe_onnx tiny
    cfg.robust = not args.no_robust
    if args.threads:
        cfg.intra_op_threads = args.threads

    m = Melody2Score(cfg)

    # 实时进度反馈（长音频下避免「卡死」观感）：优先 tqdm，缺失则降级为分段打印
    try:
        from tqdm import tqdm
        pbar = tqdm(total=100, desc="识别中", unit="%", ncols=60)

        def progress_cb(stage, msg, fraction):
            pbar.n = int(fraction * 100)
            pbar.set_description(msg)
            pbar.refresh()

        def _done():
            pbar.n = 100
            pbar.refresh()
            pbar.close()
    except Exception:
        pbar = None

        def progress_cb(stage, msg, fraction):
            print(f"  [{int(fraction*100):3d}%] {msg}")

        def _done():
            pass

    res = m.run(audio_path=args.input, record_seconds=args.record,
                out_xml=args.out, ms_score=args.mscore, progress_cb=progress_cb)
    _done()
    m.print_summary(res)


if __name__ == "__main__":
    main()
