# -*- coding: utf-8 -*-
"""全局可调参数。PC 与开发板共用，板端通过 Config.board() 取调优副本。"""
from dataclasses import dataclass
from typing import Optional


@dataclass
class Config:
    # ---- 音高检测（架构首选：crepe_onnx tiny，稳定可复现） ----
    sr: int = 16000                 # 采样率（Crepe 输入即 16k，不必再降）
    preferred_backend: str = "crepe_onnx"  # 首选后端（auto 时以此为第一优先）
    conf_thresh: float = 0.32       # 音高置信度阈值（v2: 提高到 0.32，抑制低置信噪声帧）
    model_size: str = "small"       # crepe_onnx 模型（v2: 改为 small 以提高精度，CPU 友好）
    hop: int = 10                   # 音高帧移（毫秒），越大越快、精度略降
    min_note_dur: float = 0.1       # 最短音符（秒），过滤过短噪声音符
    median_win: int = 5             # midi 轮廓中值滤波窗，去颤音/抖动
    enable_denoise: bool = True     # 是否做谱减降噪（开发板可关以省内存）
    intra_op_threads: int = 0       # onnxruntime 单算子线程数（0=默认）
    inference_timeout: float = 90.0 # 单次音高推理超时（秒），v2: small 模型稍慢
    bpm_fallback: float = 100.0     # BPM 兜底（v2: 从 120 调到流行曲中值 100）

    # ---- 声源分离（前置步骤，对带伴奏的 mp3/混合音至关重要） ----
    # v2 默认规则（兼顾纯钢琴/器乐与带伴奏人声两种核心场景）：
    #   enable_separation=None 表示「跟随 vocal_mode」：人声模式自动开，
    #   纯乐器模式自动关。用户可显式 True/False 覆盖该规则。
    enable_separation: Optional[bool] = None  # 跟随 vocal_mode
    separation_strategy: str = "auto"  # auto | demucs | hpss | none（auto=有Demucs用Demucs否则HPSS）

    # ---- 人声模式（识别人唱歌/哼唱，默认开启） ----
    vocal_mode: bool = True         # 人声模式：收窄基频、启用 VAD、加强颤音平滑
    fmin: float = 70.0              # v2: 基频下界 Hz（从 50 提到 70，滤除贝斯低频泄漏）
    fmax: float = 1050.0            # v2: 基频上界 Hz（从 1100 略降）
    enable_vad: bool = True         # 人声活动检测，过滤呼吸/停顿/气声假音高
    vad_energy_thresh: float = 0.006    # VAD 能量门限（相对峰值，偏低以抑制谐波泄漏残留）
    vad_centroid_min: float = 250.0     # v2: 谱质心下界 Hz（从 200 提）
    vad_centroid_max: float = 3400.0    # 谱质心上界 Hz
    vad_flatness_max: float = 0.22      # v2: 谱平坦度上界（下调，人声更低）
    min_voiced_ms: int = 80         # VAD 最小有声段（短于该值的孤立段视为毛刺）
    robust: bool = True             # 稳健重识别：多次识别取音符级共识，抑制单次偶发错误

    # ---- Onset 起音检测（同音连续音分割增强） ----
    # 仅靠音高变化无法区分"1 1"这类同音连续音，
    # 启用能量包络起音检测后，在能量上升处强制切分，识别准确率显著提升。
    # 对纯正弦波/合成音效果极佳；对真实人声/乐器也有明显改善。
    enable_onset: bool = True       # 启用起音检测辅助音符分割
    onset_threshold_db: float = -30.0  # 起音能量噪声底（dB）
    onset_min_gap_s: float = 0.08      # 最小起音间隔（秒），避免颤音伪起音

    # ---- MIDI 后处理纠错层（v2 新增，默认全开） ----
    enable_postprocess: bool = True # 音域/跳音/短音/长音/同音合并 全局纠错

    @classmethod
    def pc(cls) -> "Config":
        return cls()

    @classmethod
    def board(cls) -> "Config":
        """开发板调优：关降噪/分离、限 2 核、略放宽最短音符、tiny 模型提速。"""
        return cls(enable_denoise=False, enable_separation=False,
                   intra_op_threads=2, min_note_dur=0.12,
                   model_size="tiny", inference_timeout=60.0)

    @classmethod
    def vocal(cls) -> "Config":
        """人声/哼唱模式：收窄到人声典型音域，加大颤音平滑，开启分离与纠错。"""
        return cls(vocal_mode=True, fmin=80.0, fmax=1000.0,
                   median_win=7, enable_vad=True, min_note_dur=0.12,
                   enable_separation=True, enable_postprocess=True)
