# -*- coding: utf-8 -*-
"""企业级旋律/音频播放引擎（零卡顿重构版）。

设计目标（消除"一卡一卡"）：
  - 回调式环形缓冲播放：用 sounddevice.RawOutputStream 的回调，在音频
    后端线程里直接读取环形缓冲喂字节，Python 侧绝不做阻塞式 write，从
    根本上消除"咔哒"断点。
  - 生产者-消费者解耦：合成（生产者线程）与播放（消费者回调）并行，
    合成结果先预取到环形缓冲（预取窗口），播放线程无需等待合成完成。
  - 合成结果缓存：相同音序（midi 序列 + 速度）只合成一次，重复播放
    立即命中缓存，避免重复 CPU 密集计算导致的卡顿。
  - 精准停止：基于 threading.Event 的 stop()，仅停止当前播放器，
    不再调用全局 sd.stop() 误伤其他并发音频流。

对外接口保持与旧版一致：play_score / play_audio / play_file / play_bytes。
"""
import io
import os
import threading
import time
from collections import OrderedDict
from typing import Dict, List, Optional, Tuple

import numpy as np
import sounddevice as sd
import soundfile as sf

try:  # 延迟导入，仅合钢琴音色需要
    from core.synth import synth_piano
except ImportError:  # 非包运行兜底
    import sys
    sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    from core.synth import synth_piano

# ---------------------------------------------------------------------------
# 环形缓冲（线程安全，字节级预取）
# ---------------------------------------------------------------------------
class RingBuffer:
    """固定容量字节环形缓冲。生产者 write()，消费者需以恒定速率 read()。

    采用「已用长度 + 读写游标」实现，避免 bytearray 频繁拼接造成的内存抖动。
    """

    def __init__(self, capacity: int):
        self._cap = capacity
        self._buf = bytearray(capacity)
        self._start = 0          # 下一个可读位置
        self._used = 0           # 已用字节数
        self._lock = threading.Lock()
        self._not_empty = threading.Condition(self._lock)
        self._not_full = threading.Condition(self._lock)

    def capacity(self) -> int:
        return self._cap

    def readable(self) -> int:
        with self._lock:
            return self._used

    def write(self, data: bytes, timeout: float = 5.0) -> int:
        """写入 data，空间不足时阻塞等待（生产者侧预取受控）。返回实际写入字节数。

        批量内存拷贝（memoryview）替代逐字节写入，长音频写入吞吐提升一个数量级，
        避免生产者线程成为合成→播放管线的瓶颈（卡顿根因之一）。
        """
        written = 0
        deadline = time.time() + timeout
        data = memoryview(data)
        with self._not_full:
            while written < len(data):
                if time.time() > deadline:
                    break
                free = self._cap - self._used
                if free == 0:
                    self._not_full.wait(timeout=max(0.001, deadline - time.time()))
                    continue
                chunk = data[written:written + free]
                end = self._start + self._used
                if end + len(chunk) <= self._cap:
                    self._buf[end:end + len(chunk)] = chunk
                else:
                    # 跨越环形边界：拆两段拷贝
                    first = self._cap - end
                    self._buf[end:end + first] = chunk[:first]
                    self._buf[:len(chunk) - first] = chunk[first:]
                self._used += len(chunk)
                written += len(chunk)
                self._not_empty.notify_all()
        return written

    def read(self, n: int) -> bytes:
        """读出最多 n 字节；不足 n 时返回已有全部（消费者侧不阻塞音频线程）。"""
        with self._lock:
            if self._used == 0:
                return b""
            n = min(n, self._used)
            out = bytearray(n)
            end = self._start + n
            if end <= self._cap:
                out[:] = self._buf[self._start:end]
            else:
                first = self._cap - self._start
                out[:first] = self._buf[self._start:]
                out[first:n] = self._buf[:n - first]
            self._start = (self._start + n) % self._cap
            self._used -= n
            self._not_full.notify_all()
            return bytes(out)

    def clear(self):
        with self._not_full:
            self._used = 0
            self._start = 0
            self._not_full.notify_all()


# ---------------------------------------------------------------------------
# 单例播放器（环形缓冲 + 回调，精准停止）
# ---------------------------------------------------------------------------
class _ScorePlayer:
    """回调式播放器：生产者在独立线程合成并写入 RingBuffer，回调在音频
    线程读出字节喂给声卡。stop 用 threading.Event 精准控制。
    """

    def __init__(self, sr: int = 16000, dtype: str = "int16"):
        self.sr = sr
        self.dtype = dtype
        self._itemsize = 2 if dtype == "int16" else 4
        # 预取窗口：约 1.5 秒缓冲，足以吸收合成线程的抖动
        self.ring = RingBuffer(int(sr * self._itemsize * 1.5))
        self._stream: Optional[sd.RawOutputStream] = None
        self._stop_ev = threading.Event()
        self._finished_ev = threading.Event()
        self._prod_thread: Optional[threading.Thread] = None
        self._lock = threading.Lock()
        self._underruns = 0        # 提前初始化，避免回调先于 play() 触发时 AttributeError
        self._underrun_lock = threading.Lock()  # 回调线程与主线程共享计数器，须加锁

    # ---- 生产者：合成循环，结果写入环形缓冲 ----
    def _produce(self, samples_gen, synth_chunk_bytes: int, on_done=None):
        try:
            for chunk in samples_gen:
                if self._stop_ev.is_set():
                    break
                if chunk is None or len(chunk) == 0:
                    continue
                arr = np.asarray(chunk, dtype=np.float32)
                pcm = (np.clip(arr, -1.0, 1.0) * 32767.0).astype(np.int16)
                self.ring.write(pcm.tobytes())
            # 尾部补 0.1s 静音，确保末尾干净淡出、不截断
            self.ring.write(b"\x00" * (self.sr * self._itemsize // 10))
        finally:
            self._finished_ev.set()
            if on_done:
                try:
                    on_done()
                except Exception:
                    pass

    # ---- 消费者：声卡回调，绝不阻塞 ----
    def _callback(self, outdata, frames, time_info, status):
        nbytes = frames * self._itemsize
        data = self.ring.read(nbytes)
        if len(data) < nbytes:
            # 缓冲暂未跟上：剩余补静音（避免咔哒），但记录欠载
            outdata[:len(data)] = data
            outdata[len(data):] = b"\x00" * (nbytes - len(data))
            if not self._finished_ev.is_set():
                with self._underrun_lock:
                    self._underruns += 1
        else:
            outdata[:] = data
        if self._stop_ev.is_set():
            raise sd.CallbackStop

    def play(self, samples_gen, on_done=None):
        """samples_gen 逐块产出 float32 样本（每块任意长度）。非阻塞返回。"""
        with self._lock:
            self.stop()  # 先干净地停止上一次（若存在）
            self.ring.clear()
            self._stop_ev.clear()
            self._finished_ev.clear()
            with self._underrun_lock:
                self._underruns = 0

            try:
                self._stream = sd.RawOutputStream(
                    samplerate=self.sr, blocksize=2048,
                    dtype=self.dtype, channels=1, callback=self._callback,
                    latency="low")
                self._stream.start()
            except Exception:
                # 声卡启动失败（无设备/被占用）：清理半成品状态，向上抛出明确错误
                self._stream = None
                self.ring.clear()
                raise
            self._prod_thread = threading.Thread(
                target=self._produce,
                args=(samples_gen, 0, on_done), daemon=True)
            self._prod_thread.start()

    def stop(self):
        with self._lock:
            self._stop_ev.set()
            if self._stream is not None:
                try:
                    # 不再用全局 sd.stop()，仅停止本流
                    if self._stream.active:
                        self._stream.stop()
                    self._stream.close()
                except Exception:
                    pass
                self._stream = None
            if self._prod_thread is not None:
                self._prod_thread.join(timeout=2.0)
                self._prod_thread = None
            self.ring.clear()

    def is_playing(self) -> bool:
        with self._lock:
            return self._stream is not None and self._stream.active

    def underruns(self) -> int:
        with self._underrun_lock:
            return self._underruns


_player = _ScorePlayer()


# ---------------------------------------------------------------------------
# 音序 → 波形（可缓存、可单测）
# ---------------------------------------------------------------------------
def make_note_waveform(midi: int, dur: float, sr: int = 16000) -> np.ndarray:
    """合成单个音符（钢琴音色 + 轻 attack/release 包络）。返回 float32 [-1,1]。"""
    y = synth_piano(midi, dur, sr=sr)
    return np.asarray(y, dtype=np.float32)


def segment_score(notes: List[Dict], bpm: float, sr: int = 16000
                  ) -> List[Tuple[np.ndarray, float]]:
    """把结构化音序切成 (waveform, duration) 列表，便于生产者逐块产出。

    notes: [{midi, dur}]，dur 单位秒（若缺失则用 bpm 算）。返回波形块。
    """
    blocks = []
    for n in notes:
        m = int(n["midi"])
        d = float(n.get("dur") or (60.0 / bpm))
        blocks.append((make_note_waveform(m, d, sr), d))
    return blocks


# 合成结果缓存（midi 序列 + bpm 指纹 → 预合成整段波形）
_SYNTH_CACHE: "OrderedDict[str, np.ndarray]" = OrderedDict()
_SYNTH_CACHE_MAX = 8
_cache_lock = threading.Lock()


def _cache_key(notes: List[Dict], bpm: float) -> str:
    sig = ",".join(f"{int(n['midi'])}:{round(float(n.get('dur') or 0),3)}"
                   for n in notes)
    return f"{sig}|{round(bpm,1)}"


def _synth_score_cached(notes: List[Dict], bpm: float, sr: int = 16000
                        ) -> np.ndarray:
    """预合成整段（带缓存）。返回 float32 [-1,1]。"""
    key = _cache_key(notes, bpm)
    with _cache_lock:
        if key in _SYNTH_CACHE:
            _SYNTH_CACHE.move_to_end(key)
            return _SYNTH_CACHE[key]
    # 合成（CPU 密集）：在调用线程执行，但调用方（生产者线程）已与播放解耦
    blocks = segment_score(notes, bpm, sr)
    segs = []
    for wav, d in blocks:
        n = int(sr * d)
        wav = wav[:n] if len(wav) >= n else np.pad(wav, (0, n - len(wav)))
        segs.append(wav)
    full = np.concatenate(segs).astype(np.float32) if segs else np.zeros(1, np.float32)
    full = full / (np.max(np.abs(full)) + 1e-9) * 0.85  # 统一归一化，防削波
    with _cache_lock:
        _SYNTH_CACHE[key] = full
        while len(_SYNTH_CACHE) > _SYNTH_CACHE_MAX:
            _SYNTH_CACHE.popitem(last=False)
    return full


def _score_samples_gen(notes: List[Dict], bpm: float, sr: int = 16000,
                       chunk: int = 8192):
    """生成器：分块产出合成波形（生产者侧），支持缓存命中即时全量产出。"""
    y = _synth_score_cached(notes, bpm, sr)
    for i in range(0, len(y), chunk):
        yield y[i:i + chunk].astype(np.float32)


# ---------------------------------------------------------------------------
# 对外 API（保持旧签名，零改动接入 GUI / WebUI）
# ---------------------------------------------------------------------------
def play_score(notes: List[Dict], bpm: float = 120.0, sr: int = 16000,
               on_done=None) -> None:
    """播放结构化音序（生产者-消费者 + 环形缓冲，流畅无卡顿）。非阻塞。"""
    gen = _score_samples_gen(notes, bpm, sr)
    _player.play(gen, on_done=on_done)


def play_audio(y: np.ndarray, sr: int = 16000, on_done=None) -> None:
    """播放任意 float32 波形。分块产出，避免一次性构造巨型字节缓冲。"""
    y = np.asarray(y, dtype=np.float32)
    peak = np.max(np.abs(y)) + 1e-9
    y = y / peak * 0.85

    def gen():
        for i in range(0, len(y), 8192):
            yield y[i:i + 8192]
    _player.play(gen(), on_done=on_done)


def play_file(path: str, on_done=None) -> None:
    """播放音频文件（wav/flac/ogg 等 soundfile 支持格式）。"""
    y, sr = sf.read(path, dtype="float32", always_2d=False)
    if y.ndim > 1:
        y = y[:, 0]
    play_audio(y, sr=sr, on_done=on_done)


def play_bytes(data: bytes, sr: int = 16000, on_done=None) -> None:
    """播放音频字节（wav 等）。"""
    y, sr = sf.read(io.BytesIO(data), dtype="float32", always_2d=False)
    if y.ndim > 1:
        y = y[:, 0]
    play_audio(y, sr=sr, on_done=on_done)


def stop() -> None:
    """停止当前播放（精准停止本播放器，不影响其他音频流）。"""
    _player.stop()


def play_raw(y: np.ndarray, sr: int = 16000, on_done=None) -> None:
    """兼容别名：play_audio 的等价入口（GUI 试听原曲用）。"""
    play_audio(y, sr=sr, on_done=on_done)


def is_playing() -> bool:
    return _player.is_playing()


def last_underruns() -> int:
    return _player.underruns()


# 兼容旧版直接脚本运行（若有遗留调用）
if __name__ == "__main__":
    print("audio_play 已重构为回调式环形缓冲播放器；请通过 play_score/play_file 调用。")
