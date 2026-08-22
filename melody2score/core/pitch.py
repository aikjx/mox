# -*- coding: utf-8 -*-
"""音高检测层（可插拔后端，首选 crepe_onnx tiny，稳定可降级）。

设计原则（企业级稳定高效）：
  1) 首选后端锁定：auto 模式下以 Config.preferred_backend 为第一优先；
     默认 crepe_onnx(tiny) —— 嵌入式 ONNX 版 CREPE，加载快、CPU 友好、可复现。
  2) 懒加载 + 可用性缓存：每个后端首次 import 后记录「可用/不可用」，
     后续调用不再反复 try/except 探测，避免重复 import 开销与偶发崩溃。
  3) 降级顺序：crepe_onnx → pyin（零额外依赖、稳定） → torchcrepe
     （PyTorch 实现，部分 Windows 环境 onnxruntime 段错误，放最后兜底）。
  4) 单次推理超时保护：防止模型异常卡死拖垮 UI/服务。
  5) 确定性：固定 OMP 线程，保证同一输入相同输出。

统一输出：[{t, freq, conf}, ...]（time 秒 / freq Hz / conf 0~1）。
无论哪个后端，下游解析层拿到的是同一种轮廓，保证流水线一致。
"""
from typing import Dict, List, Optional, Tuple

import numpy as np


class PitchDetector:
    # 降级顺序：首选 crepe_onnx，pyin 最稳作第二，torchcrepe 易崩放最后
    FALLBACK_ORDER = ("crepe_onnx", "pyin", "torchcrepe")

    def __init__(self, model_size: str = "tiny", conf_thresh: float = 0.3,
                 hop: int = 10, intra_op_threads: int = 0,
                 backend: str = "auto", sr: int = 16000,
                 fmin: float = 50.0, fmax: float = 1100.0,
                 preferred_backend: str = "crepe_onnx",
                 inference_timeout: float = 60.0):
        self.model_size = model_size
        self.conf_thresh = conf_thresh
        self.hop = hop
        self.intra_op_threads = intra_op_threads
        self.sr = sr
        self.fmin = fmin
        self.fmax = fmax
        self.inference_timeout = inference_timeout
        # backend 显式指定 > 配置首选
        self.preferred_backend = backend if backend != "auto" else preferred_backend
        self._used: Optional[str] = None
        # 后端可用性缓存：None=未探测, True=可用, False=不可用
        self._backend_ok: Dict[str, Optional[bool]] = {
            k: None for k in self.FALLBACK_ORDER
        }

    @property
    def used_backend(self) -> Optional[str]:
        return self._used

    # ---- 后端可用性探测（懒加载 + 缓存） ----
    def _is_backend_ok(self, kind: str) -> bool:
        if self._backend_ok[kind] is not None:
            return self._backend_ok[kind]
        try:
            if kind == "crepe_onnx":
                import crepe_onnx  # noqa: F401
            elif kind == "torchcrepe":
                import torchcrepe  # noqa: F401
            else:  # pyin
                import librosa  # noqa: F401
            ok = True
        except Exception:
            ok = False
        self._backend_ok[kind] = ok
        return ok

    # ---- 各后端实现 ----
    def _detect_crepe_onnx(self, y: np.ndarray, sr: int,
                           conf_thresh: Optional[float] = None) -> List[Dict]:
        thr = self.conf_thresh if conf_thresh is None else conf_thresh
        if self.intra_op_threads and self.intra_op_threads > 0:
            import os
            os.environ.setdefault("OMP_NUM_THREADS", str(self.intra_op_threads))
        import crepe_onnx
        time_arr, freq_arr, conf_arr, _ = crepe_onnx.predict(
            y, sr, model_size=self.model_size, step_size=self.hop, verbose=0)
        return [{"t": float(t), "freq": float(f), "conf": float(c)}
                for t, f, c in zip(time_arr, freq_arr, conf_arr)
                if c >= thr and self.fmin <= f <= self.fmax]

    def _detect_torchcrepe(self, y: np.ndarray, sr: int,
                           conf_thresh: Optional[float] = None) -> List[Dict]:
        thr = self.conf_thresh if conf_thresh is None else conf_thresh
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
            if np.isnan(f) or f <= 20 or c < thr or f < self.fmin or f > self.fmax:
                continue
            res.append({"t": float(t), "freq": float(f), "conf": float(c)})
        return res

    def _detect_pyin(self, y: np.ndarray, sr: int,
                     conf_thresh: Optional[float] = None) -> List[Dict]:
        thr = self.conf_thresh if conf_thresh is None else conf_thresh
        import librosa
        fmin, fmax = self.fmin, self.fmax
        # pyin 为逐帧概率基频追踪，耗时与帧数近似线性。用 2 倍 hop（10→20ms）
        # 平衡速度与短音符精度：3 倍 hop（robust 下 54ms）会让 0.25 拍短音
        # （~105ms）只落 1~2 帧、边界帧再被 VAD 切除后仅剩单帧 → 时长 0
        # 被并入邻音（实测生日歌弱起 C 被吞、音符数 12→10）。2 倍 hop
        # 短音可得 3+ 帧，耗时约 1.5×，精度收益远大于速度代价。
        eff_hop = max(10, int(self.hop * 2))
        frame = 1024                       # 满足 frame_length >= 2*hop_length
        hop_len = max(1, int(round(sr * eff_hop / 1000.0)))
        f0, voiced_flag, voiced_prob = librosa.pyin(
            y, fmin=fmin, fmax=fmax, sr=sr, frame_length=frame, hop_length=hop_len)
        times = librosa.times_like(f0, sr=sr, hop_length=hop_len, n_fft=frame)
        res = []
        for t, f, v, p in zip(times, f0, voiced_flag, voiced_prob):
            if f is None or not v or p < thr or f <= 20:
                continue
            res.append({"t": float(t), "freq": float(f), "conf": float(p)})
        return res

    def _candidates(self) -> List[str]:
        if self.preferred_backend in self.FALLBACK_ORDER:
            # 首选置首，其余按默认降级顺序补齐（去重）
            return [self.preferred_backend] + [
                k for k in self.FALLBACK_ORDER if k != self.preferred_backend
            ]
        return list(self.FALLBACK_ORDER)

    def detect(self, y: np.ndarray, sr: int,
               conf_thresh: Optional[float] = None) -> List[Dict]:
        """检测音高轮廓，自动首选 + 优雅降级。

        conf_thresh 可按次调用覆盖（稳健模式多次 run 的扰动阈值）；
        缺省用构造时的 self.conf_thresh。
        返回 [{t, freq, conf}]；若所有后端均不可用抛 RuntimeError。
        """
        last_err: Optional[BaseException] = None
        for kind in self._candidates():
            if not self._is_backend_ok(kind):
                continue  # 跳过已知不可用后端，不重复探测
            try:
                if kind == "crepe_onnx":
                    out = self._run_with_timeout(self._detect_crepe_onnx, y, sr,
                                                 conf_thresh=conf_thresh)
                elif kind == "torchcrepe":
                    out = self._run_with_timeout(self._detect_torchcrepe, y, sr,
                                                 conf_thresh=conf_thresh)
                else:
                    out = self._run_with_timeout(self._detect_pyin, y, sr,
                                                 conf_thresh=conf_thresh)
                self._used = kind
                return out
            except Exception as e:
                # 该后端本次失败：标记不可用，下次跳过，继续降级
                self._backend_ok[kind] = False
                last_err = e
                continue
        raise RuntimeError(f"所有音高后端均不可用（首选={self.preferred_backend}）: {last_err}")

    def _run_with_timeout(self, fn, y, sr, **kw) -> List[Dict]:
        """超时保护：用子线程包装推理，超时则抛出异常触发降级。"""
        import threading
        import queue

        q: "queue.Queue" = queue.Queue(maxsize=1)
        exc: List[BaseException] = []

        def _worker():
            try:
                q.put(("ok", fn(y, sr, **kw)))
            except BaseException as e:  # 捕获一切，含段错误前的异常
                exc.append(e)
                q.put(("err", e))

        th = threading.Thread(target=_worker, daemon=True)
        th.start()
        th.join(self.inference_timeout)
        if th.is_alive():
            # 无法强制杀线程，但标记为不可用，避免反复卡死
            raise TimeoutError(
                f"后端推理超时（>{self.inference_timeout}s），已降级")
        if exc:
            raise exc[0]
        status, val = q.get()
        if status == "err":
            raise val
        return val
