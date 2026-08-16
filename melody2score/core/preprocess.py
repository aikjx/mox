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
    """轻量谱减：以开头 0.1s 静音段为噪声底估计，减去幅度谱。"""
    D = librosa.stft(y, n_fft=512, hop_length=128)
    mag = np.abs(D)
    noise_len = max(1, int(0.1 * sr / 128))
    noise = np.mean(mag[:, :noise_len], axis=1, keepdims=True)
    mag = np.maximum(mag - noise, 0.0)
    return librosa.istft(mag * np.exp(1j * np.angle(D)), hop_length=128).astype(np.float32)
