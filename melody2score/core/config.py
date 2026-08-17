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

    # ---- 人声模式（识别人唱歌/哼唱，默认开启） ----
    vocal_mode: bool = True         # 人声模式：收窄基频、启用 VAD、加强颤音平滑
    fmin: float = 50.0              # 基频下界（Hz）
    fmax: float = 1100.0            # 基频上界（Hz）
    enable_vad: bool = True         # 人声活动检测，过滤呼吸/停顿/气声假音高
    vad_energy_thresh: float = 0.006    # VAD 能量门限（相对峰值，偏低以抑制谐波泄漏残留）
    vad_centroid_min: float = 200.0     # VAD 谱质心下界
    vad_centroid_max: float = 3500.0    # VAD 谱质心上界
    vad_flatness_max: float = 0.25      # VAD 谱平坦度上界（人声低）
    min_voiced_ms: int = 80         # VAD 最小有声段（短于该值的孤立段视为毛刺）
    robust: bool = True              # 稳健重识别：多次识别取音符级共识，抑制单次偶发错误

    @classmethod
    def pc(cls) -> "Config":
        return cls()

    @classmethod
    def board(cls) -> "Config":
        """开发板调优：关降噪、限 2 核、略放宽最短音符。"""
        return cls(enable_denoise=False, intra_op_threads=2, min_note_dur=0.12)

    @classmethod
    def vocal(cls) -> "Config":
        """人声/哼唱模式调优：把基频范围收紧到人声典型音域，加大颤音平滑。"""
        return cls(vocal_mode=True, fmin=80.0, fmax=1000.0,
                   median_win=7, enable_vad=True, min_note_dur=0.12)
