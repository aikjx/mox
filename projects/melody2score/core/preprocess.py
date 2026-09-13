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
    """Estimate noise only from low-energy frames; preserve active starts.

    If no quiet region is observed, keep the signal instead of guessing noise.
    """
    D = librosa.stft(y, n_fft=512, hop_length=128)
    mag = np.abs(D)
    phase = np.angle(D)
    # A recording may start with a note. Never assume its first 100ms is noise.
    energy = np.mean(mag ** 2, axis=0)
    peak = float(np.max(energy))
    quiet = energy <= min(float(np.quantile(energy, 0.15)), peak * 0.01)
    if peak <= 1e-12 or np.count_nonzero(quiet) < 3:
        return y.astype(np.float32, copy=True)
    noise = np.median(mag[:, quiet], axis=1, keepdims=True)
    floor = 0.1 * mag
    mag_clean = np.maximum(mag - 2.0 * noise, floor)
    return librosa.istft(mag_clean * np.exp(1j * phase), hop_length=128,
                         length=len(y)).astype(np.float32)
