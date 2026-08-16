# -*- coding: utf-8 -*-
"""全局可调参数。PC 与开发板共用，板端通过 Config.board() 取调优副本。"""
from dataclasses import dataclass


@dataclass
class Config:
    sr: int = 16000                 # 采样率（Crepe 输入即 16k，不必再降）
    conf_thresh: float = 0.3        # 音高置信度阈值，低于则判为无声/噪声
    model_size: str = "tiny"        # crepe_onnx 模型：tiny/small/full
    hop: int = 10                   # 音高帧移（毫秒），越大越快、精度略降
    min_note_dur: float = 0.1       # 最短音符（秒），过滤过短噪声音符
    median_win: int = 5             # midi 轮廓中值滤波窗，去颤音/抖动
    enable_denoise: bool = True     # 是否做谱减降噪（开发板可关以省内存）
    intra_op_threads: int = 0       # onnxruntime 单算子线程数（0=默认）
    bpm_fallback: float = 120.0     # BPM 检测失败时的兜底速度

    @classmethod
    def pc(cls) -> "Config":
        return cls()

    @classmethod
    def board(cls) -> "Config":
        """开发板调优：关降噪、限 2 核、略放宽最短音符。"""
        return cls(enable_denoise=False, intra_op_threads=2, min_note_dur=0.12)
