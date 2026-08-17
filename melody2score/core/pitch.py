# -*- coding: utf-8 -*-
"""音高检测层（可插拔后端）。

后端优先级（auto 模式依次尝试）：
  1) crepe_onnx  —— 架构首选：嵌入式 ONNX 版 CREPE（tiny），开发板用。
  2) torchcrepe  —— 真实 CREPE tiny 模型（PyTorch 实现，pip 可装）。
  3) pyin        —— librosa 概率化 YIN，零额外依赖兜底，真实音高估计（非伪造）。

统一输出：[{t, freq, conf}, ...]（time 秒 / freq Hz / conf 0~1）。
无论哪个后端，下游解析层拿到的是同一种轮廓，保证流水线一致。
"""
from typing import Dict, List, Optional, Tuple

import numpy as np


class PitchDetector:
    BACKENDS = ("crepe_onnx", "torchcrepe", "pyin")

    def __init__(self, model_size: str = "tiny", conf_thresh: float = 0.3,
                 hop: int = 10, intra_op_threads: int = 0,
                 backend: str = "auto", sr: int = 16000,
                 fmin: float = 50.0, fmax: float = 1100.0):
        self.model_size = model_size
        self.conf_thresh = conf_thresh
        self.hop = hop
        self.intra_op_threads = intra_op_threads
        self.backend = backend
        self.sr = sr
        self.fmin = fmin
        self.fmax = fmax
        self._used: Optional[str] = None

    @property
    def used_backend(self) -> Optional[str]:
        return self._used

    # ---- 各后端实现 ----
    def _detect_crepe_onnx(self, y: np.ndarray, sr: int) -> List[Dict]:
        if self.intra_op_threads and self.intra_op_threads > 0:
            import os
            os.environ.setdefault("OMP_NUM_THREADS", str(self.intra_op_threads))
        import crepe_onnx
        time_arr, freq_arr, conf_arr, _ = crepe_onnx.predict(
            y, sr, model_size=self.model_size, step_size=self.hop, verbose=0)
        return [{"t": float(t), "freq": float(f), "conf": float(c)}
                for t, f, c in zip(time_arr, freq_arr, conf_arr)
                if c >= self.conf_thresh and self.fmin <= f <= self.fmax]

    def _detect_torchcrepe(self, y: np.ndarray, sr: int) -> List[Dict]:
        import torch
        import torchcrepe
        audio = torch.tensor(y, dtype=torch.float32).unsqueeze(0)  # [1, T]
        hop_length = max(1, int(round(sr * self.hop / 1000.0)))  # 样本数
        out = torchcrepe.predict(
            audio, sr, model=self.model_size, hop_length=hop_length,
            fmin=self.fmin, fmax=self.fmax, device="cpu", return_periodicity=True)
        pitch, periodicity = out
        f0 = np.asarray(pitch.squeeze().detach().cpu().float().numpy())
        conf = np.asarray(periodicity.squeeze().detach().cpu().float().numpy())
        times = np.arange(len(f0)) * hop_length / sr
        res = []
        for t, f, c in zip(times, f0, conf):
            if np.isnan(f) or f <= 20 or c < self.conf_thresh or f < self.fmin or f > self.fmax:
                continue
            res.append({"t": float(t), "freq": float(f), "conf": float(c)})
        return res

    def _detect_pyin(self, y: np.ndarray, sr: int) -> List[Dict]:
        import librosa
        fmin, fmax = self.fmin, self.fmax
        frame = 2048
        hop_len = max(1, int(round(sr * self.hop / 1000.0)))
        f0, voiced_flag, voiced_prob = librosa.pyin(
            y, fmin=fmin, fmax=fmax, sr=sr, frame_length=frame, hop_length=hop_len)
        times = librosa.times_like(f0, sr=sr, hop_length=hop_len, n_fft=frame)
        res = []
        for t, f, v, p in zip(times, f0, voiced_flag, voiced_prob):
            if f is None or not v or p < self.conf_thresh or f <= 20:
                continue
            res.append({"t": float(t), "freq": float(f), "conf": float(p)})
        return res

    def detect(self, y: np.ndarray, sr: int) -> List[Dict]:
        if self.backend != "auto":
            candidates = [self.backend]
        else:
            candidates = list(self.BACKENDS)
        last_err: Optional[BaseException] = None
        for kind in candidates:
            try:
                if kind == "crepe_onnx":
                    out = self._detect_crepe_onnx(y, sr)
                elif kind == "torchcrepe":
                    out = self._detect_torchcrepe(y, sr)
                else:
                    out = self._detect_pyin(y, sr)
                self._used = kind
                return out
            except Exception as e:  # 该后端不可用 → 试下一个
                last_err = e
                continue
        raise RuntimeError(f"所有音高后端均不可用: {last_err}")
