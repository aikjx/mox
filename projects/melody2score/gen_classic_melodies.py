# -*- coding: utf-8 -*-
"""合成经典旋律数据集：不同旋律 × 不同乐器/人声/纯音乐。

生成到 melody2score/audio/，并写出 audio/manifest.json（含 ground truth 音高）。
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from classic_corpus import generate_all

if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    m = generate_all(here)
    print(f"完成：{len(m)} 个样例 -> {os.path.join(here, 'audio')}")
