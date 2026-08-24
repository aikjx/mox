# -*- coding: utf-8 -*-
"""开发板调优配置：限核、关降噪、略放宽最短音符。"""
from core.config import Config


def board_config() -> Config:
    return Config(
        sr=16000,
        conf_thresh=0.3,
        model_size="tiny",          # RK3568 可上 small；树莓派 4B 用 tiny
        hop=10,
        min_note_dur=0.12,          # 板端环境噪声大，略放宽
        median_win=5,
        enable_denoise=False,       # 关谱减降噪以省内存/算力
        intra_op_threads=2,         # 限 2 核，避免占满导致系统卡顿
        bpm_fallback=120.0,
    )
