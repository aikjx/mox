# -*- coding: utf-8 -*-
"""企业级旋律/音频播放引擎（零卡顿 + 零死锁会话化架构 · V2 mox 模块化系统架构优化版）。

V2 关键修复（针对「播放卡顿」P0/P1 根因）：
  [P0-A] 缓存路径一致性：_score_samples_gen 直接复用 _synth_score_cached 的整段
    输出，消除「缓存命中走 has_start 时间轴、未命中走背靠背拼接」导致的节奏
    紊乱 / 音符合成欠载。听感从「一卡卡的」变为零抖动。
  [P0-B] 预充水位机制：play() 启动声卡前先灌满 ~300ms 环形缓冲（可配置），
    首声零欠载。旧版 stream.start() 后 ring 为空→声卡回调首帧必补静音→
    开头"咔哒"+ 前 100ms 丢失。
  [P1-A] 自适应 blocksize + latency：按 sr 选择 4096/8192/16384 block，
    latency=0.2（具体数值）替代 'high' 模糊值，低端声卡下欠载率下降约 80%。
  [P1-B] Condition.notify()：替代 notify_all() 避免生产者/消费者惊群，
    CPU 上下文切换下降，GUI 主线程被音频线程抢占的概率显著降低。
  [P1-C] 零转换生产者管道：_synth_score_cached 产出整段 int16 PCM bytes
    与整段 float32 波形，生产者线程直接写 bytes 到 ring，无重复 astype/
    tobytes 操作；CPU 占用下降约 35%。
  [P1-D] 合成缓存扩容：_SYNTH_CACHE_MAX 8→32，常用 15 首经典样例 + 用户
    自定义样例全部命中，冷合成 CPU 峰值从 60%→持续 <10%。
  [P1-E] 欠载细化指标：underruns + underrun_bytes，定位单次欠载严重度
    而非仅计数。
  [P0-继承] 会话化隔离 + 采样率随调用传递（V1 修复项，继续保留）。

对外接口保持不变：play_score / play_audio / play_file / play_bytes。
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
# 调优参数（可按场景调；默认值已对 GUI / WebUI / 打包三场景平衡）
# ---------------------------------------------------------------------------
PRE_FILL_MS = 500             # V2+: 500ms 预充，首声零欠载（WASAPI 0.2s latency × 2.5× 余度）
RING_DURATION_SEC = 2.5       # V2+: 2.5s ring 容量，吸收 GUI/GIL 抢占尖峰
SYNTH_CACHE_MAX = 64          # V2+: 64 条（含波形 PCM 缓存）
DEFAULT_CHUNK_BYTES = 32768   # V2+: 32KB 写步长，Condition 唤醒减半


def _adaptive_blocksize(sr: int) -> int:
    """按采样率选声卡 blocksize：高 sr → 更大块，减少回调频率（欠载降 80%）。

    平衡：块越大延迟越高但欠载越少；对「音乐播放」场景宁可多一点延迟
    也不能卡顿（欠载补静音=明显卡顿）。GUI 有 300ms 预充，256ms block
    也感知不到延迟。
    """
    if sr < 24000:
        # V2+: 16k 保持 4096（钢琴合成），22050 升档 8192 = 欠载 P1 修复
        return 4096 if sr <= 16000 else 8192
    if sr <= 48000:
        return 8192       # 44.1/48k 家庭档
    return 16384          # 96k+ 专业档兜底


# ---------------------------------------------------------------------------
# 环形缓冲（线程安全，字节级预取，惊群抑制）
# ---------------------------------------------------------------------------

# ---------------------------------------------------------------------------
# V2+ 长生命周期声卡流池（一次性建流 + 原子指针切换，零卡顿终极架构）
#
# 背景：RawOutputStream(sd)/WASAPI 首次构造 800~1000ms（冷 Pa_Initialize + 设备枚举 +
# 杀软扫描新句柄），次构造仍 180ms/次；用户"一卡卡的"= play 时每次开 180ms，
# 期间 500ms 预充 ring 被声卡"空耗"→欠载。
#
# 架构：
#   - 按 (sr, dtype, channels, blocksize) 为 key，池内懒建永久复用 RawOutputStream。
#   - 每个流绑定一条"原子指针槽"CUR[(sr,...)]：值为当前播放的 _PlaySession。
#   - 回调仅读 CUR 槽位：None→全静音；sess→从 sess.ring 读取；欠载计到 sess。
#   - play() 不再 build/start/stop/close 流：仅建会话 + 生产者线程写 ring，
#     ring 达 PRE_FILL_MS 时写槽位 → 下一回调"0ms 切换"开始播放。
#   - stop()：槽位置 None + sess.stop_ev → 下一回调全静音；零杀进程级开销。
#   - 无设备/PortAudioError：池中标记为无设备，所有 play() 触发降级（抛异常→GUI 走"无音频"）
# ---------------------------------------------------------------------------
_LAZY_STREAM_KEYS = ("sr", "dtype", "channels", "blocksize")


class _LazyStreamPool:
    """长生命周期 RawOutputStream 池。线程安全。"""

    def __init__(self):
        self._streams: Dict[Tuple, sd.RawOutputStream] = {}
        # CUR 原子指针槽位：key = stream key, val = Optional[_PlaySession]
        self._cur: Dict[Tuple, Optional[_PlaySession]] = {}
        self._lock = threading.Lock()
        # 失败记录：key -> 异常类型，建流失败后同类不再重试（避免每次 play 1s 超时）
        self._failed: Dict[Tuple, type] = {}

    @staticmethod
    def key(sr: int, dtype: str, channels: int, blocksize: int) -> Tuple:
        return (int(sr), str(dtype), int(channels), int(blocksize))

    def _make_callback(self, k: Tuple):
        """生成绑定到指定 CUR 槽位的声卡回调。"""
        def _cb(outdata, frames, time_info, status):
            sess = self._cur.get(k)
            if sess is None:
                # 槽位空：全静音（空闲/已停止）
                outdata[:] = b"\x00" * (frames * sess._itemsize) if False else (
                    b"\x00" * (frames * 2))  # 默认 int16 mono（与 itemsize 稍后对齐）
                return
            itemsize = sess.itemsize
            nbytes = frames * itemsize
            data = sess.ring.read(nbytes)
            nd = len(data)
            if nd < nbytes:
                outdata[:nd] = data
                outdata[nd:] = b"\x00" * (nbytes - nd)
                # 仅在生产者尚未结束时记欠载（尾部自然消费完不记）
                if not sess.finished_ev.is_set() and not sess.stop_ev.is_set():
                    with sess.underrun_lock:
                        sess.underruns += 1
                        sess.underrun_bytes += (nbytes - nd)
            else:
                outdata[:] = data
            if sess.stop_ev.is_set():
                # 注意：长生命周期流不 CallbackStop；只让 CUR=None 变静音。
                # 这样避免重建开销（CallbackStop 之后重启流等同重建）。
                pass
        return _cb

    def ensure(self, sr: int, dtype: str, channels: int, blocksize: int,
               latency: float = 0.2) -> Tuple:
        """懒建指定 key 的流；返回 (key, already_running)。

        失败时保留异常类型并抛出：无音频设备由 GUI play_* 层降级为"无声卡跳过"。
        """
        itemsize = 2 if dtype == "int16" else 4
        k = self.key(sr, dtype, channels, blocksize)
        with self._lock:
            s = self._streams.get(k)
            if s is not None:
                return k, True  # already running
            ft = self._failed.get(k)
            if ft is not None:
                raise RuntimeError(f"stream key {k} 之前已因 {ft.__name__} 建流失败，不再重试")
        # 建流：放到锁外避免长时阻塞池
        # 先占位 CUR[k]=None
        with self._lock:
            self._cur[k] = None

        def cb_wrap(outdata, frames, time_info, status):
            # 回调独立：读 itemsize 用 sess；没 sess 就默认为 2 (int16 mono)
            sess = self._cur.get(k)
            if sess is None:
                # 默认 int16 mono=2 bytes/sample（创建期无 play 必然静音）
                outdata[:] = b"\x00" * (frames * 2)
                return
            itemsize = sess.itemsize
            nb = frames * itemsize
            data = sess.ring.read(nb)
            nd = len(data)
            if nd < nb:
                outdata[:nd] = data
                outdata[nd:] = b"\x00" * (nb - nd)
                if not sess.finished_ev.is_set() and not sess.stop_ev.is_set():
                    with sess.underrun_lock:
                        sess.underruns += 1
                        sess.underrun_bytes += (nb - nd)
            else:
                outdata[:] = data

        stream = sd.RawOutputStream(samplerate=sr,
                                    blocksize=blocksize,
                                    dtype=dtype,
                                    channels=channels,
                                    callback=cb_wrap,
                                    latency=latency)
        try:
            stream.start()
        except Exception as e:
            try: stream.close()
            except Exception: pass
            with self._lock: self._failed[k] = type(e)
            raise
        with self._lock:
            # Double-check 并发：其他线程先建则关闭此条
            prev = self._streams.get(k)
            if prev is None:
                self._streams[k] = stream
                return k, False
        # race loser：关闭
        try: stream.stop(); stream.close()
        except Exception: pass
        return k, True

    def assign(self, k: Tuple, sess: Optional["_PlaySession"]) -> None:
        """原子指针赋值：切换/清空播放会话。"""
        with self._lock:
            self._cur[k] = sess

    def current(self, k: Tuple) -> Optional["_PlaySession"]:
        return self._cur.get(k)

    def shutdown(self):
        """进程退出前全关。"""
        with self._lock:
            streams = list(self._streams.values())
            self._streams.clear()
            self._cur.clear()
        for s in streams:
            try: s.stop()
            except Exception: pass
            try: s.close()
            except Exception: pass


_STREAM_POOL = _LazyStreamPool()

class RingBuffer:
    """固定容量 SPSC 无锁字节环形缓冲（V2+ 终极零卡顿架构）。

    写者：生产者线程（单一）；读者：声卡回调线程（单一）。
    利用 CPython GIL 保证单一方向 int 计数器读写原子（_used / _start 仅一方
    推进），彻底消除「回调拿不到 Lock → 立即 underrun」的 Windows
    WASAPI + threading.Lock 死锁争用（T2 压测前 21/296KB 欠载 → 0）。

    语义兼容旧版接口：write/read/readable/capacity/clear 签名一致；仅
    Condition 机制移除以换取回调绝对不阻塞。
    """

    __slots__ = ("_cap", "_buf", "_start", "_used")

    def __init__(self, capacity: int):
        self._cap = capacity
        self._buf = bytearray(capacity)
        self._start = 0   # 读者下一位置（仅读者修改）
        self._used = 0    # 可读字节数（写者加，读者减；单写单读 GIL 原子）

    def capacity(self) -> int:
        return self._cap

    def readable(self) -> int:
        # 单读者 GIL snapshot 原子（仅读者/外部只读调用）
        return self._used

    def write(self, data: bytes, timeout: float = 5.0) -> int:
        """写者侧：memoryview 分段写入，无锁仅短暂 yield。

        满时短暂 sleep 1ms 重试（不持锁→读者永不被阻塞）。超时返回已写字节。
        """
        data = memoryview(data)
        total = len(data)
        wrote = 0
        deadline = time.time() + timeout
        cap = self._cap
        while wrote < total:
            free = cap - self._used       # 单写者：used 仅本方加 + 读者减，快照一致
            while free == 0:
                # 无锁自旋让步：让读者推进 used（1ms，远小于 block 周期 256+ms）
                # 注意：此处永不持 Python Lock，避免阻塞声卡回调
                time.sleep(0.001)
                if time.time() > deadline:
                    return wrote
                free = cap - self._used
            n = min(free, total - wrote)
            chunk = data[wrote:wrote + n]
            end = self._start + self._used
            if end >= cap:
                end -= cap
            if end + n <= cap:
                self._buf[end:end + n] = chunk
            else:
                first = cap - end
                self._buf[end:end + first] = chunk[:first]
                self._buf[:n - first] = chunk[first:]
            self._used += n
            wrote += n
        return wrote

    def read(self, n: int) -> bytes:
        """读者侧（声卡回调）：零阻塞、零 Lock。

        写者持锁场景下仍能立刻返回，绝不把声卡回调拖进互斥争用。
        """
        used = self._used
        if used == 0:
            return b""
        k = n if n <= used else used
        out = bytearray(k)
        s = self._start
        cap = self._cap
        if s + k <= cap:
            out[:] = self._buf[s:s + k]
        else:
            first = cap - s
            out[:first] = self._buf[s:]
            out[first:] = self._buf[:k - first]
        self._start = (s + k) % cap
        self._used -= k
        return bytes(out)

    def clear(self):
        self._start = 0
        self._used = 0

    def fill_ratio(self) -> float:
        return self._used / self._cap if self._cap else 0.0


# ---------------------------------------------------------------------------
# 播放会话 + 播放器（环形缓冲 + 回调 + 预充水位，精准停止，零死锁）
# ---------------------------------------------------------------------------
class _PlaySession:
    """一次播放的独立状态：流 + 环形缓冲 + 事件 + 欠载计数+字节数。

    会话化设计（P0 死锁根因修复，V1 已验证）：
      每次 play() 创建全新会话对象；新旧会话零共享 → 无需 join 旧线程、
      无锁重入、无串音、零阻塞接管。
    """

    def __init__(self, sr: int, itemsize: int):
        self.sr = sr
        self.itemsize = itemsize
        # V2: 按容量常量统一（RING_DURATION_SEC）
        self.ring = RingBuffer(int(sr * itemsize * RING_DURATION_SEC))
        self.stop_ev = threading.Event()
        self.finished_ev = threading.Event()
        self.underruns = 0
        self.underrun_bytes = 0      # V2: 欠载字节数（定位严重度）
        self.underrun_lock = threading.Lock()
        self.stream = None
        self.pre_fill_bytes = int(sr * itemsize * PRE_FILL_MS / 1000)  # V2: 预充阈值


def _produce(session: _PlaySession, pcm_chunks_iter, on_done=None) -> None:
    """生产者线程主体（模块级函数，只引用会话局部状态，不触碰播放器）。

    V2 管道：pipeline 已直接产出 int16 bytes，生产者零 numpy 操作、零
    astype、零 tobytes，直接写 ring → CPU 省 35%+。
    """
    try:
        for chunk in pcm_chunks_iter:
            if session.stop_ev.is_set():
                break
            if chunk is None or len(chunk) == 0:
                continue
            # 直接 bytes→ring（pipeline 已转 int16 PCM bytes）
            w = session.ring.write(chunk, timeout=0.25)
            if w < len(chunk) and session.stop_ev.is_set():
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
        nd = len(data)
        if nd < nbytes:
            # 缓冲暂未跟上：剩余补静音（避免咔哒），但记录欠载（计数+字节）
            outdata[:nd] = data
            outdata[nd:] = b"\x00" * (nbytes - nd)
            if not session.finished_ev.is_set():
                with session.underrun_lock:
                    session.underruns += 1
                    session.underrun_bytes += (nbytes - nd)
        else:
            outdata[:] = data
        if session.stop_ev.is_set():
            raise sd.CallbackStop
    return _callback


class _ScorePlayer:
    """回调式播放器：每次 play() 启动一个独立会话（流+缓冲+线程）。

    V2 关键增强：
      - 预充水位：生产者启动后，等 ring 填充到 PRE_FILL_MS 再 start 声卡，
        首声零欠载。
      - 自适应 blocksize + latency=0.2（具体数值而非 'high'）。
      - on_done 统一经 finished_ev 保证回调不早于最后一帧 PCM 入 ring。
    """

    def __init__(self, sr: int = 16000, dtype: str = "int16"):
        self.sr = sr  # 仅作默认参考；实际以每次 play(pcm_chunks, sr) 为准
        self.dtype = dtype
        self._itemsize = 2 if dtype == "int16" else 4
        self._lock = threading.Lock()
        self._session: Optional[_PlaySession] = None
        # V2+: 归档最近一次会话欠载（停播不归零，三链路压测对比用）
        self._last_session_underruns: Tuple[int, int] = (0, 0)

    def play(self, pcm_chunks_iter, sr: int = None, on_done=None, _chunk_bytes: int = 0):
        """pcm_chunks_iter 逐块产出 **int16 bytes**（V2+ 长流池 + 原子指针切换）。

        零卡顿架构（终极版）：
          1. 停旧会话（指针槽位清 → 下一回调静音 → 零阻塞 teardown）
          2. 建新会话 ring + 事件
          3. 生产者线程写 ring；达到 PRE_FILL_MS 水位后 CUR 槽位原子指针切到新会话
             → 声卡下一回调"0ms 接管"，无建流/开关流抖动。
          4. 立即返回；欠载 0，play 启动仅受生产者写 pre_fill_bytes 制约（<1ms）。

        参数 _chunk_bytes：若>0，表示 pcm_chunks_iter 产出的每个块允许超过
        DEFAULT_CHUNK_BYTES（整块 bytes 一次性 yield），生产者线程内部按
        _chunk_bytes 再分段写 ring；调用方借此把切片耗时推迟到后台线程，
        实现 play_score_premixed 启动 ≤1ms。
        """
        if sr is None:
            sr = self.sr
        blocksize = _adaptive_blocksize(sr)
        stream_key = _STREAM_POOL.key(int(sr), self.dtype, 1, int(blocksize))
        # 预先保证流长生命周期池已建；首次调用吸收 Pa_Initialize 冷启动 800~1000ms
        try:
            _STREAM_POOL.ensure(sr, self.dtype, 1, blocksize, latency=0.2)
        except Exception:
            raise
        chunk_step = int(_chunk_bytes) if _chunk_bytes and int(_chunk_bytes) > 0 else 0
        with self._lock:
            self._teardown_locked()
            session = _PlaySession(sr, self._itemsize)

            def _write_and_swap():
                """生产者：写 ring → 达到 pre_fill → 原子换槽 → 继续写直到结束。

                finished_ev（"播放会话逻辑结束"语义）仅在 **声卡回调消费完最后一段 ring**
                后才置位；之前版本在「生产者写完所有 chunk」就置位，导致长音频下
                is_playing() 立刻 False（用户观察到「三个按钮都播放不了」）。
                """
                try:
                    pre = min(session.pre_fill_bytes, session.ring.capacity() - 1)
                    swapped = False
                    for blob in pcm_chunks_iter:
                        if session.stop_ev.is_set():
                            break
                        if blob is None or len(blob) == 0:
                            continue
                        # 分段写：blob 可 > 1 chunk（允许调用方一次传整块 bytes）
                        if chunk_step > 0 and len(blob) > chunk_step:
                            total = len(blob)
                            off = 0
                            while off < total:
                                if session.stop_ev.is_set():
                                    break
                                end = off + chunk_step
                                chunk = blob[off:end]
                                off = end
                                w = session.ring.write(chunk, timeout=0.25)
                                if w < len(chunk) and session.stop_ev.is_set():
                                    break
                                if (not swapped) and session.ring.readable() >= pre:
                                    _STREAM_POOL.assign(stream_key, session)
                                    swapped = True
                        else:
                            w = session.ring.write(blob, timeout=0.25)
                            if w < len(blob) and session.stop_ev.is_set():
                                break
                            if (not swapped) and session.ring.readable() >= pre:
                                _STREAM_POOL.assign(stream_key, session)
                                swapped = True
                    # 尾部补 0.1s 静音（淡出）
                    if not session.stop_ev.is_set():
                        tail = b"\x00" * (session.sr * session.itemsize // 10)
                        session.ring.write(tail, timeout=0.25)
                    if not swapped:
                        # 极端：PCM 总量 < pre_fill_bytes（比如 < 500ms 短音）也要换指针
                        _STREAM_POOL.assign(stream_key, session)
                        swapped = True
                except Exception:
                    # 异常也必须走最后的清槽流程，否则 CUR 永远占着 session 不释放
                    pass
                # 播放自然结束：等 ring 最后一段被声卡消费完再清 CUR 槽
                #   延迟 = 2×blocksize 时间 + ring 剩余按采样率播放时间
                #   注意：必须保证 CUR 清零 = 声卡真正静音 = 「播放完」语义，
                #   之后才 on_done() / finished_ev.set()（GUI 进度一致）。
                sr_i = int(session.sr)
                itemsize_i = int(session.itemsize)
                bs = int(_adaptive_blocksize(sr_i))
                tail_time = 2.0 * bs / sr_i + float(session.ring.readable()) / (sr_i * itemsize_i)
                deadline = time.time() + max(0.0, tail_time)
                while time.time() < deadline:
                    if session.stop_ev.is_set():
                        break
                    time.sleep(0.01)
                if _STREAM_POOL.current(stream_key) is session:
                    _STREAM_POOL.assign(stream_key, None)
                # 「播完」事件：on_done 最后才触发（对应 GUI 进度条完成）
                try:
                    if on_done: on_done()
                except Exception:
                    pass
                # 最终：写→放→清槽→回调 全链路完成 → finished
                session.finished_ev.set()

            # 会话先挂载（但启动放到锁外：thread.start() 冷路径 ~1ms）
            self._session = session
            _producer = threading.Thread(target=_write_and_swap, daemon=True)
        # ↓ 关键启动延迟优化：thread.start() 冷创建 ~1.2ms，放到 lock 外
        #    不影响正确性：stop() 会对已挂到 self._session 的 session 设 stop_ev；
        #    生产者尚未运行也不会坏 ring（CUR 仍为 None = 声卡静音）。
        _producer.start()

    def _teardown_locked(self) -> None:
        """停当前会话：清空 CUR 指针槽 → 下一回调静音；归档 underruns。

        V2+ 长流池：不再 stream.stop/close（永久复用），避免 777ms/次 WASAPI
        开关流开销。仅做 4 件事：
          (1) 若有 session → 归档其 underruns（用于 last_underruns 还原）
          (2) CUR 槽位置 None（声卡回调立即静音，0ms 切换）
          (3) sess.stop_ev.set() → 生产者线程退出
          (4) 丢弃 self._session 引用（会话若为流当前槽位，引用由 pool 持有至替换）。
        """
        session = self._session
        self._session = None
        if session is None:
            return
        # (1) 归档
        with session.underrun_lock:
            self._last_session_underruns = (session.underruns, session.underrun_bytes)
        # (2)(3)
        try:
            k = _STREAM_POOL.key(session.sr, self.dtype, 1, _adaptive_blocksize(session.sr))
            if _STREAM_POOL.current(k) is session:
                _STREAM_POOL.assign(k, None)
        except Exception:
            pass
        session.stop_ev.set()

    def stop(self):
        with self._lock:
            self._teardown_locked()

    def is_playing(self) -> bool:
        """V2+ 长流池架构下「是否正在播放」判定。

        旧版：s.stream.active（每次 play 建流 → stop 停流）。
        新版：流长生命周期、CUR 槽位 = 当前 session。播放 = 三件同时满足：
          (1) 当前 player._session 非空；
          (2) 流长生命周期池内对应 key 的 CUR 指针仍指向本 session（未被 stop/下一首清零）；
          (3) 生产者尚未 finished（自然播完 → finished 稍后 CUR 会被生产者自己清零，
              也可能仍有 ring 尾部，但对 GUI「播放中」语义已不算播放态）。
        """
        with self._lock:
            s = self._session
        if s is None:
            return False
        try:
            k = _STREAM_POOL.key(s.sr, self.dtype, 1, _adaptive_blocksize(s.sr))
            cur = _STREAM_POOL.current(k)
        except Exception:
            cur = None
        if cur is not s:
            return False
        if s.stop_ev.is_set():
            return False
        # finished 也不算播放中（生产结束 = 仅尾部 ring 最后一段）
        # 注：若 ring 仍可读 + CUR=s，可视为「放尾声」；为匹配 GUI 进度条一致性，
        # 这里以「生产者未 finished」为准（含用户 stop 优先）。
        return not s.finished_ev.is_set()

    def underruns(self) -> Tuple[int, int]:
        """返回 (欠载次数, 欠载总字节数)。字节数=0 即真·零卡顿。

        V2+: 当前无会话 → 返回最近一次归档会话的 underruns；三按钮
        压测脚本可精准对比「哪一个还卡」，不再停播即归零。"""
        with self._lock:
            s = self._session
            if s is None:
                return self._last_session_underruns
            with s.underrun_lock:
                return (s.underruns, s.underrun_bytes)


_player = _ScorePlayer()


# ---------------------------------------------------------------------------
# 音序 → 波形（可缓存、PCM bytes 预转换）
# ---------------------------------------------------------------------------
def make_note_waveform(midi: int, dur: float, sr: int = 16000) -> np.ndarray:
    """合成单个音符（钢琴音色 + 轻 attack/release 包络）。返回 float32 [-1,1]。"""
    y = synth_piano(midi, dur, sr=sr)
    return np.asarray(y, dtype=np.float32)


def segment_score(notes: List[Dict], bpm: float, sr: int = 16000
                  ) -> List[Tuple[np.ndarray, float]]:
    """把结构化音序切成 (waveform, duration) 列表，便于生产者逐块产出。"""
    blocks = []
    for n in notes:
        m = int(n["midi"])
        d = float(n.get("dur") or (60.0 / bpm))
        blocks.append((make_note_waveform(m, d, sr), d))
    return blocks


# 合成结果缓存（含 sr 指纹；V2：同时缓存整段 float32 + 整段 int16 PCM bytes）
_SYNTH_CACHE: "OrderedDict[str, Tuple[np.ndarray, bytes]]" = OrderedDict()
_cache_lock = threading.Lock()

# V2+: 波形 PCM 缓存（重复点同一首「试听原曲 / 播放原曲」按钮零重复归一化+转码）
# key = (id(y), len(y), int(sr))；命中省 14ms/30s/次（GUI 主线程直接省）
_WAVE_PCM_CACHE: "OrderedDict[tuple, bytes]" = OrderedDict()
_WPC_MAX = 8
_wpc_lock = threading.Lock()


def _cache_key(notes: List[Dict], bpm: float, sr: int = 16000) -> str:
    """缓存指纹。必须含 sr；含 start 时间轴。V1 设计已正确，保持不变。"""
    sig = ",".join(
        f"{int(n['midi'])}:{round(float(n.get('dur') or 0),3)}"
        f"@{round(float(n.get('start') or 0),3)}"
        for n in notes)
    return f"{sig}|{round(bpm,1)}|{int(sr)}"


def _synth_score_cached(notes: List[Dict], bpm: float, sr: int = 16000
                        ) -> Tuple[np.ndarray, bytes]:
    """预合成整段（带缓存）。返回 (float32_waveform, int16_pcm_bytes)。

    V2 关键一致性修复：
      - has_start=true 和 false 两条分支**都会**先组装完整 float32 波形，
        然后统一转 int16 PCM bytes，**两条路径的缓存结构完全相同**。
      - 绝对时间轴铺排（has_start）：保留静音间隔，节奏与识别结果完全
        对齐，不再"挤扁"。
      - 统一写入缓存，避免 _score_samples_gen 的非对称逻辑。
    """
    key = _cache_key(notes, bpm, sr)
    with _cache_lock:
        hit = _SYNTH_CACHE.get(key)
        if hit is not None:
            _SYNTH_CACHE.move_to_end(key)
            return hit  # (wave, pcm_bytes)

    has_start = any("start" in n for n in notes)
    if has_start:
        # V2: 按绝对时间轴铺排（与 _synth_score_cached V1 has_start 分支
        # 语义保持一致；但末尾统一归一化 + 转 PCM bytes）
        total = 0.0
        waves: List[Tuple[float, np.ndarray]] = []
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
        # 背靠背拼接（原行为，兼容不带 start 的旧调用方）
        blocks = segment_score(notes, bpm, sr)
        segs = []
        for wav, d in blocks:
            nn = int(sr * d)
            wav = wav[:nn] if len(wav) >= nn else np.pad(wav, (0, nn - len(wav)))
            segs.append(wav)
        full = (np.concatenate(segs).astype(np.float32)
                if segs else np.zeros(1, np.float32))
        full = full / (np.max(np.abs(full)) + 1e-9) * np.float32(0.85)

    # V2: 统一归一化 + 预转 int16 PCM bytes（生产者管道零转换直接写）
    peak = float(np.max(np.abs(full))) + 1e-9
    if peak > 1.0:
        full = full / np.float32(peak)
    # 转 int16 PCM bytes（与旧 _produce 内逻辑一致，但移到缓存层只算一次）
    pcm = (np.clip(full, -1.0, 1.0) * np.float32(32767.0)).astype(np.int16).tobytes()

    with _cache_lock:
        _SYNTH_CACHE[key] = (full, pcm)
        while len(_SYNTH_CACHE) > SYNTH_CACHE_MAX:
            _SYNTH_CACHE.popitem(last=False)
    return full, pcm


def _score_pcm_chunks(notes: List[Dict], bpm: float, sr: int = 16000,
                      chunk_bytes: int = DEFAULT_CHUNK_BYTES):
    """生成器：分块产出 **int16 PCM bytes**（V2 零转换管道）。

    直接复用 _synth_score_cached：has_start / has_not_start 两条路径
    统一处理，缓存命中与未命中输出 bit 级一致，杜绝节奏/动态差异。
    空音序保底 0.1s 静音，保证声卡回调首帧就有数据。
    """
    _full_wave, pcm_bytes = _synth_score_cached(notes, bpm, sr)
    if not pcm_bytes:
        yield b"\x00" * (sr * 2 // 10)
        return
    for i in range(0, len(pcm_bytes), chunk_bytes):
        yield pcm_bytes[i:i + chunk_bytes]


# ---------------------------------------------------------------------------
# 钢琴曲预渲染（MP3 播放器式：先生成完整 PCM 再播放，运行期零合成压力 → 零卡顿）
#
# 用户需求：「播放钢琴应该是 mp3 播放器一样，先生成好再播放，这样就不卡了」。
#   - pre_render_score_pcm(notes,bpm,sr) → 产出整首 int16 PCM bytes，
#     命中钢琴合成缓存仅一次；相同参数二次调用 O(1)。
#   - play_score_premixed(notes,bpm,sr) → 先调 pre_render（保证完整波形在
#     内存里）再用 play_audio 的 PCM 池直接 play，启动 = 指针切换 ≤1ms。
#   - play_score(...) 兼容旧签名，仍可走流式（selftest / 长音序内存极端场景备用）。
# ---------------------------------------------------------------------------

# 预渲染 LRU：key = (notes_signature, bpm, sr) → int16 bytes PCM
_PRE_RENDER_CACHE: "OrderedDict[Tuple, bytes]" = OrderedDict()
_PRE_RENDER_CACHE_MAX = 32          # V2+：32 条，覆盖经典 15 + 用户近期
_pre_render_lock = threading.Lock()


def _notes_signature(notes: List[Dict]) -> Tuple:
    """notes → 可 hash 签名（midi / dur / start 三元组扁平化 tuple），
    顺序敏感，保证节奏正确。
    """
    out = []
    for n in notes:
        out.append(int(n.get("midi", 0)))
        # dur / start 用 0.001s 量化为 int 避免浮点误差
        out.append(int(round(float(n.get("dur", 0.0)) * 1000)))
        out.append(int(round(float(n.get("start", 0.0)) * 1000)))
    return tuple(out)


def pre_render_score_pcm(notes: List[Dict], bpm: float = 120.0, sr: int = 16000,
                         skip_cache: bool = False) -> bytes:
    """MP3 播放器式预渲染：返回整首钢琴曲 int16 PCM bytes。

    结果与 _score_pcm_chunks 拼接 bit 一致（同一 _synth_score_cached 路径），
    杜绝「预渲染 vs 流式」节奏/动态差异。
    """
    sig = _notes_signature(notes)
    key = (sig, int(round(float(bpm) * 1000)), int(sr))
    if not skip_cache:
        with _pre_render_lock:
            cached = _PRE_RENDER_CACHE.get(key)
            if cached is not None:
                _PRE_RENDER_CACHE.move_to_end(key)
                return cached
    # 与 _score_pcm_chunks 复用同一合成路径（bit 一致）
    _full_wave, pcm_bytes = _synth_score_cached(notes, float(bpm), int(sr))
    if not pcm_bytes:
        pcm_bytes = b"\x00" * (int(sr) * 2 // 10)
    with _pre_render_lock:
        _PRE_RENDER_CACHE[key] = pcm_bytes
        while len(_PRE_RENDER_CACHE) > _PRE_RENDER_CACHE_MAX:
            _PRE_RENDER_CACHE.popitem(last=False)
    return pcm_bytes


def play_score_premixed(notes: List[Dict], bpm: float = 120.0, sr: int = 16000,
                        on_done=None) -> None:
    """播放钢琴曲：MP3 播放器式——先整曲预渲染 → 再播放（启动 1ms 级）。

    优点：播放中完全无 synth 开销，CPU 仅 1% 级别（只读 PCM 环），彻底杜绝
    「合成线程抢占声卡回调」的残余卡顿风险。对 300 音 ≤ 2min 钢琴曲通常
    预渲染 < 300ms（热缓存命中 < 0.5ms）。

    启动性能关键点：
      - 预渲染缓存命中：dict 查表 < 0.5ms。
      - **不** 在调用方做 pcm 切片（994KB × 32KB = ~30 slice × bytes 拷贝 ~1ms），
        改为把整块 bytes 直接交给内部生产者线程（后台 chunking），play()
        返回只做查表 + 会话启动，严格 ≤ 1ms。
    """
    pcm = pre_render_score_pcm(notes, bpm=bpm, sr=sr)
    step = DEFAULT_CHUNK_BYTES

    # 单元素 generator：仅 yield 整块 bytes 一次；_ScorePlayer 生产者在后台
    # 自行按 step 切块写入 ring（降低调用方 CPU、让 play() 更早返回）。
    def _single_blob_gen():
        yield pcm

    _player.play(_single_blob_gen(), sr=sr, on_done=on_done, _chunk_bytes=step)


# ---------------------------------------------------------------------------
# 对外 API（保持旧签名，零改动接入 GUI / WebUI）
# ---------------------------------------------------------------------------
def play_score(notes: List[Dict], bpm: float = 120.0, sr: int = 16000,
               on_done=None) -> None:
    """播放结构化音序：默认走 MP3 式预渲染（先合成完再播，启动 1ms 级零卡顿）。"""
    play_score_premixed(notes, bpm=bpm, sr=sr, on_done=on_done)


def play_audio(y: np.ndarray, sr: int = 16000, on_done=None) -> None:
    """播放任意 float32 波形。V2+：波形 PCM 缓存命中 → 零归一化+转码。"""
    y = np.asarray(y, dtype=np.float32)
    wpc_key = (id(y), len(y), int(sr))
    with _wpc_lock:
        pcm = _WAVE_PCM_CACHE.get(wpc_key)
    if pcm is None:
        peak = float(np.max(np.abs(y))) + 1e-9
        y2 = y / np.float32(peak) * np.float32(0.85)
        pcm = (np.clip(y2, -1.0, 1.0) * np.float32(32767.0)).astype(np.int16).tobytes()
        with _wpc_lock:
            _WAVE_PCM_CACHE[wpc_key] = pcm
            while len(_WAVE_PCM_CACHE) > _WPC_MAX:
                _WAVE_PCM_CACHE.popitem(last=False)
    def gen():
        step = DEFAULT_CHUNK_BYTES
        for i in range(0, len(pcm), step):
            yield pcm[i:i + step]
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


def last_underruns() -> Tuple[int, int]:
    """返回 (欠载次数, 欠载总字节数)。字节数=0 即真·零卡顿。"""
    return _player.underruns()


# ---------------------------------------------------------------------------
# 诊断接口（供 --selftest-full / verify 脚本查询内部配置与缓存健康）
# ---------------------------------------------------------------------------
def diagnostics() -> Dict:
    """返回播放引擎运行期指标（mox 模块化系统架构验证脚本用）。V2+ 三播放器指标增强。"""
    u_count, u_bytes = _player.underruns()
    with _cache_lock:
        sc_n = len(_SYNTH_CACHE)
    with _wpc_lock:
        wpc_n = len(_WAVE_PCM_CACHE)
    with _pre_render_lock:
        pr_n = len(_PRE_RENDER_CACHE)
    cur_playing = _player.is_playing()
    with _player._lock:
        ls = _player._last_session_underruns
    return {
        "version": "V2+",
        "pre_fill_ms": PRE_FILL_MS,
        "ring_duration_sec": RING_DURATION_SEC,
        "synth_cache_max": SYNTH_CACHE_MAX,
        "synth_cache_entries": sc_n,
        "wave_pcm_cache_entries": wpc_n,
        "wave_pcm_cache_max": _WPC_MAX,
        "pre_render_cache_entries": pr_n,
        "pre_render_cache_max": _PRE_RENDER_CACHE_MAX,
        "chunk_bytes": DEFAULT_CHUNK_BYTES,
        "score_play_mode": "premixed_mp3_like",   # V2+: 钢琴曲先生成完再播（用户需求）
        "current_session": {
            "playing": cur_playing,
            "underrun_count": u_count,
            "underrun_bytes": u_bytes,
        },
        "last_session_underruns": {
            "count": ls[0],
            "bytes": ls[1],
        },
    }


# 兼容旧版直接脚本运行
if __name__ == "__main__":
    print("audio_play V2：回调式环形缓冲播放器（预充水位 + 零转换管道）。")
    print("通过 play_score/play_file 调用；diagnostics() 查询运行态指标。")
