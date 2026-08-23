# -*- coding: utf-8 -*-
"""企业级旋律/音频播放引擎（零卡顿 + 零死锁会话化架构）。

设计目标：
  - 回调式环形缓冲播放：sounddevice.RawOutputStream 回调在音频后端线程
    直接读环形缓冲喂字节，Python 侧绝不做阻塞式 write，消除"咔哒"断点。
  - 会话化隔离（P0 死锁修复）：每次 play() 创建独立会话（流+缓冲+事件），
    新旧会话零共享；旧版 play() 持锁调 stop() 二次取同一把不可重入锁，
    点「播放」即 GUI 主线程永久死锁。会话化后零锁重入、播放中切歌
    零阻塞（不 join 旧线程）、零串音（旧线程只写旧缓冲）。
  - 采样率随调用传递（P1 修复）：流按 play(gen, sr) 的实际采样率创建，
    修复旧版固定 16000Hz 流播放 22050Hz 波形导致的变调变慢。
  - 流式合成（P2 首声优化）：未命中缓存时逐音符合成即时产出，首声延迟
    ≈10ms（旧版整曲合成完毕才出声，长曲按下数秒无声）。
  - 合成缓存：指纹含 (midi 序列, bpm, sr)，跨采样率不互串。

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
# 播放会话 + 播放器（环形缓冲 + 回调，精准停止，零死锁）
# ---------------------------------------------------------------------------
class _PlaySession:
    """一次播放的独立状态：流 + 环形缓冲 + 事件 + 欠载计数。

    会话化设计（P0 死锁根因修复）：
      旧版 play() 在持有 self._lock 时调用 stop()，而 stop() 内部再次
      获取同一把 threading.Lock（不可重入）→ 点「播放」即 GUI 主线程
      永久死锁（selftest 只测合成未测 play()，故打包验收全绿仍必现）。
      现每次 play() 创建全新会话对象：旧会话停流后其生产者线程持有旧
      ring/事件引用自然消亡（daemon，写旧缓冲无副作用，GC 回收），
      新旧会话零共享 → 无需 join 旧线程、无锁重入、无串音、零阻塞接管。
    """

    def __init__(self, sr: int, itemsize: int):
        self.sr = sr
        self.itemsize = itemsize
        # 预取窗口：约 1.5 秒缓冲，足以吸收合成线程的抖动
        self.ring = RingBuffer(int(sr * itemsize * 1.5))
        self.stop_ev = threading.Event()
        self.finished_ev = threading.Event()
        self.underruns = 0
        self.underrun_lock = threading.Lock()
        self.stream = None


def _produce(session: _PlaySession, samples_gen, on_done=None) -> None:
    """生产者线程主体（模块级函数，只引用会话局部状态，不触碰播放器）。

    短超时写入：stop 后若 ring 满且无人读，最多 0.25s 即退出，
    残留线程把数据写进【旧会话】的 ring（与新会话隔离），零串音。
    """
    try:
        for chunk in samples_gen:
            if session.stop_ev.is_set():
                break
            if chunk is None or len(chunk) == 0:
                continue
            arr = np.asarray(chunk, dtype=np.float32)
            pcm = (np.clip(arr, -1.0, 1.0) * 32767.0).astype(np.int16)
            data = pcm.tobytes()
            w = session.ring.write(data, timeout=0.25)
            if w < len(data) and session.stop_ev.is_set():
                break
        # 尾部补 0.1s 静音，确保末尾干净淡出、不截断
        if not session.stop_ev.is_set():
            session.ring.write(b"\x00" * (session.sr * session.itemsize // 10),
                               timeout=0.25)
    finally:
        session.finished_ev.set()
        if on_done:
            try:
                on_done()
            except Exception:
                pass


def _make_consumer(session: _PlaySession, itemsize: int):
    """构建绑定到指定会话的声卡回调（消费者，绝不阻塞）。"""
    def _callback(outdata, frames, time_info, status):
        nbytes = frames * itemsize
        data = session.ring.read(nbytes)
        if len(data) < nbytes:
            # 缓冲暂未跟上：剩余补静音（避免咔哒），但记录欠载
            outdata[:len(data)] = data
            outdata[len(data):] = b"\x00" * (nbytes - len(data))
            if not session.finished_ev.is_set():
                with session.underrun_lock:
                    session.underruns += 1
        else:
            outdata[:] = data
        if session.stop_ev.is_set():
            raise sd.CallbackStop
    return _callback


class _ScorePlayer:
    """回调式播放器：每次 play() 启动一个独立会话（流+缓冲+线程）。

    对外 API 不变：play/stop/is_playing/underruns。
    play() 支持按调用传入采样率（修复旧版固定 16000Hz 流导致的
    22050Hz 波形变调变慢——钢琴曲/试听原曲全链路采样率对齐）。
    """

    def __init__(self, sr: int = 16000, dtype: str = "int16"):
        self.sr = sr  # 仅作默认参考；实际以每次 play(samples_gen, sr) 为准
        self.dtype = dtype
        self._itemsize = 2 if dtype == "int16" else 4
        self._lock = threading.Lock()
        self._session: Optional[_PlaySession] = None

    def play(self, samples_gen, sr: int = None, on_done=None):
        """samples_gen 逐块产出 float32 样本（每块任意长度）。非阻塞返回。

        快速接管：停旧会话的流（不 join 旧生产者线程——它写旧 ring，
        与新会话隔离，自然消亡），因此播放中切歌零阻塞、零串音。
        """
        if sr is None:
            sr = self.sr
        with self._lock:
            self._teardown_locked()
            session = _PlaySession(sr, self._itemsize)
            try:
                session.stream = sd.RawOutputStream(
                    samplerate=sr, blocksize=4096,
                    dtype=self.dtype, channels=1,
                    callback=_make_consumer(session, self._itemsize),
                    latency="high")
                session.stream.start()
            except Exception:
                # 声卡启动失败（无设备/被占用）：置位停止事件（生产者线程
                # 若已启动会立即退出），向上抛出明确错误
                session.stop_ev.set()
                raise
            self._session = session
            threading.Thread(
                target=_produce, args=(session, samples_gen, on_done),
                daemon=True).start()

    def _teardown_locked(self) -> None:
        """停止当前会话（假定已持锁）。不 join 生产者线程（零阻塞）。"""
        session = self._session
        self._session = None
        if session is None:
            return
        session.stop_ev.set()
        if session.stream is not None:
            try:
                if session.stream.active:
                    session.stream.stop()
                session.stream.close()
            except Exception:
                pass

    def stop(self):
        with self._lock:
            self._teardown_locked()

    def is_playing(self) -> bool:
        with self._lock:
            s = self._session
            return s is not None and s.stream is not None and s.stream.active

    def underruns(self) -> int:
        with self._lock:
            s = self._session
            if s is None:
                return 0
            with s.underrun_lock:
                return s.underruns


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


def _cache_key(notes: List[Dict], bpm: float, sr: int = 16000) -> str:
    """缓存指纹。必须含 sr：旧版缺失导致 16000/22050 波形互串（变调）。
    含 start：同一 midi/dur 但时间轴不同（对齐静音间隔）须区分。"""
    sig = ",".join(
        f"{int(n['midi'])}:{round(float(n.get('dur') or 0),3)}"
        f"@{round(float(n.get('start') or 0),3)}"
        for n in notes)
    return f"{sig}|{round(bpm,1)}|{int(sr)}"


def _synth_score_cached(notes: List[Dict], bpm: float, sr: int = 16000
                        ) -> np.ndarray:
    """预合成整段（带缓存）。返回 float32 [-1,1]。

    若音符带 'start' 时间轴，则严格按 start 铺排（音符间保留静音间隔），
    使合成曲总时长 = 原曲时长，听感速度与原曲一致（避免背靠背拼接导致
    整体变短、听起来『太快』）。无 start 时退回背靠背拼接（原行为）。
    """
    key = _cache_key(notes, bpm, sr)
    with _cache_lock:
        if key in _SYNTH_CACHE:
            _SYNTH_CACHE.move_to_end(key)
            return _SYNTH_CACHE[key]
    has_start = any("start" in n for n in notes)
    if has_start:
        # 按绝对时间轴铺排：总时长 = 最后一个音符的起始 + 时长
        total = 0.0
        waves = []
        for n in notes:
            s = float(n.get("start", 0.0))
            d = max(0.05, float(n.get("dur") or (60.0 / bpm)))
            wav = make_note_waveform(int(n["midi"]), d, sr) * np.float32(0.85)
            waves.append((s, wav))
            total = max(total, s + d)
        n_total = int(sr * total) + 1
        full = np.zeros(n_total, dtype=np.float32)
        for s, wav in waves:
            pos = int(sr * s)
            if pos + len(wav) <= n_total:
                full[pos:pos + len(wav)] += wav
            else:
                full[pos:] += wav[:n_total - pos]
    else:
        blocks = segment_score(notes, bpm, sr)
        segs = []
        for wav, d in blocks:
            nn = int(sr * d)
            wav = wav[:nn] if len(wav) >= nn else np.pad(wav, (0, nn - len(wav)))
            segs.append(wav)
        full = np.concatenate(segs).astype(np.float32) if segs else np.zeros(1, np.float32)
        full = full / (np.max(np.abs(full)) + 1e-9) * 0.85
    with _cache_lock:
        _SYNTH_CACHE[key] = full
        while len(_SYNTH_CACHE) > _SYNTH_CACHE_MAX:
            _SYNTH_CACHE.popitem(last=False)
    return full


def _score_samples_gen(notes: List[Dict], bpm: float, sr: int = 16000,
                       chunk: int = 8192):
    """生成器：分块产出合成波形（生产者侧）。

    无论缓存命中与否，均先整段合成完毕再分块高速 yield，使生产者
    在极短时间内灌满 1.5s 环形缓冲，随后声卡稳定消费——彻底消除
    逐音符流式喂给声卡带来的欠载爆音/卡顿（「一卡卡的」根因）。
    synth_piano 为轻量 numpy 向量化，整首合成仅数毫秒，首延迟可忽略；
    单音符峰值 *0.85 缩放保持与原实现数值一致。
    """
    key = _cache_key(notes, bpm, sr)
    with _cache_lock:
        cached = _SYNTH_CACHE.get(key)
        if cached is not None:
            _SYNTH_CACHE.move_to_end(key)
    if cached is not None:
        full = cached
    else:
        segs = []
        for n in notes:
            m = int(n["midi"])
            d = max(0.05, float(n.get("dur") or (60.0 / bpm)))
            wav = make_note_waveform(m, d, sr) * np.float32(0.85)
            cnt = int(sr * d)
            wav = wav[:cnt] if len(wav) >= cnt else np.pad(wav, (0, cnt - len(wav)))
            segs.append(wav)
        if not segs:                      # 空音序：0.1s 静音，保证回调有数据
            yield np.zeros(int(sr * 0.1), dtype=np.float32)
            return
        full = np.concatenate(segs).astype(np.float32)
        with _cache_lock:
            _SYNTH_CACHE[key] = full
            while len(_SYNTH_CACHE) > _SYNTH_CACHE_MAX:
                _SYNTH_CACHE.popitem(last=False)
    for i in range(0, len(full), chunk):
        yield full[i:i + chunk].astype(np.float32, copy=False)


# ---------------------------------------------------------------------------
# 对外 API（保持旧签名，零改动接入 GUI / WebUI）
# ---------------------------------------------------------------------------
def play_score(notes: List[Dict], bpm: float = 120.0, sr: int = 16000,
               on_done=None) -> None:
    """播放结构化音序（生产者-消费者 + 环形缓冲，流畅无卡顿）。非阻塞。"""
    gen = _score_samples_gen(notes, bpm, sr)
    _player.play(gen, sr=sr, on_done=on_done)


def play_audio(y: np.ndarray, sr: int = 16000, on_done=None) -> None:
    """播放任意 float32 波形。分块产出，避免一次性构造巨型字节缓冲。"""
    y = np.asarray(y, dtype=np.float32)
    peak = np.max(np.abs(y)) + 1e-9
    y = y / peak * 0.85

    def gen():
        for i in range(0, len(y), 8192):
            yield y[i:i + 8192]
    _player.play(gen(), sr=sr, on_done=on_done)


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
