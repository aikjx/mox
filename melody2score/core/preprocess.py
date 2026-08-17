# -*- coding: utf-8 -*-
"""预处理层：去直流偏移 + 归一化 + 可选谱减降噪。"""
import numpy as np
import librosa


def preprocess(y: np.ndarray, sr: int = 16000, enable_denoise: bool = True) -> np.ndarray:
    y = y - np.mean(y)                       # 去直流偏移
    y = y / (np.max(np.abs(y)) + 1e-9)       # 峰值归一化
    if enable_denoise:
        y = _spectral_subtract(y, sr)
    return y.astype(np.float32)


def _spectral_subtract(y: np.ndarray, sr: int) -> np.ndarray:
    """轻量谱减：以开头 0.1s 静音段为噪声底估计，带过减因子与下限保护。

    数值稳定点：
    - 过减因子(2.0)在噪声较强处多减，抑制残留噪声/谐波毛刺；
    - 下限保护 0.1*mag 避免把有效谐波削成 0（否则 CREPE 置信骤降→假音高）。
    """
    D = librosa.stft(y, n_fft=512, hop_length=128)
    mag = np.abs(D)
    phase = np.angle(D)
    noise_len = max(1, int(0.1 * sr / 128))
    noise = np.mean(mag[:, :noise_len], axis=1, keepdims=True)
    # 噪声底下限，避免过减把谐波彻底抹掉
    floor = 0.1 * mag
    mag_clean = np.maximum(mag - 2.0 * noise, floor)
    return librosa.istft(mag_clean * np.exp(1j * phase), hop_length=128).astype(np.float32)
