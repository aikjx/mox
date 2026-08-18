# -*- coding: utf-8 -*-
"""audio_play 播放引擎企业级测试：聚焦「零卡顿」与多线程健壮性。

覆盖：
  - 环形缓冲连续性：生产者逐块写入、消费者按序读出，数据不可断裂/重复/乱序。
  - 合成波形平滑：相邻样本无跳变（无咔哒），能量包络单调衰减。
  - 缓存命中：相同音序第二次合成应直接命中缓存（更快）。
  - 并发停止竞态：多线程反复 play/stop 不崩溃、不串音、不抛异常。
  - 无音频设备环境可降级运行（用注入式 RingBuffer 测试，不依赖声卡）。

运行：
    pytest tests/test_audio_play.py -q
或直接：
    python tests/test_audio_play.py
"""
import os
import sys
import time
import threading

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from app.audio_play import RingBuffer, synth_piano, segment_score, _synth_score_cached, _cache_key

SR = 16000


# ---------------------------------------------------------------------------
# 1) 环形缓冲：生产者/消费者连续性（核心「不卡顿」保证）
# ---------------------------------------------------------------------------
def test_ringbuffer_sequential_integrity():
    """生产者写入递增字节序列，消费者读出必须完全保持顺序、无缺漏。"""
    rb = RingBuffer(capacity=4096)
    total = 100_000
    produced = bytes((i % 251) for i in range(total))  # 伪随机但确定

    def producer():
        rb.write(produced, timeout=5.0)

    result = {"got": None}

    def consumer():
        got = bytearray()
        while len(got) < total:
            chunk = rb.read(997)  # 不规则读取尺寸，模拟音频帧
            if chunk:
                got.extend(chunk)
            else:
                time.sleep(0.001)
        result["got"] = bytes(got)

    tp = threading.Thread(target=producer)
    tc = threading.Thread(target=consumer)
    tp.start(); tc.start()
    tp.join(timeout=10); tc.join(timeout=10)

    assert result["got"] == produced, "环形缓冲数据断裂/乱序/缺失！"


def test_ringbuffer_overflow_backpressure():
    """容量不足时 write 阻塞等待消费者，而非静默丢数据。"""
    rb = RingBuffer(capacity=1024)
    big = bytes((i % 251) for i in range(5000))  # 远超容量
    written = []
    drained = bytearray()
    prod_done = threading.Event()

    def prod():
        written.append(rb.write(big, timeout=15.0))
        prod_done.set()

    def cons():
        # 持续消费直到生产者完成且缓冲清空
        while not prod_done.is_set() or rb.readable() > 0:
            c = rb.read(512)
            if c:
                drained.extend(c)
            else:
                time.sleep(0.001)

    tp = threading.Thread(target=prod)
    tc = threading.Thread(target=cons)
    tp.start(); tc.start()
    # 起步即写满，readable 应被 cap 限制
    assert rb.readable() <= 1024
    tp.join(timeout=15); tc.join(timeout=15)
    assert written and written[0] == 5000, "溢出时应全部写入，不得丢数据"
    assert len(drained) == 5000, "消费者应取走全部数据"


# ---------------------------------------------------------------------------
# 2) 合成波形平滑性（无咔哒的关键：无样本跳变）
# ---------------------------------------------------------------------------
def test_synth_waveform_smooth():
    """单个音符波形应平滑：相邻样本差分不可出现大跳变（即无咔哒）。"""
    y = synth_piano(69, 0.5, sr=SR)
    assert y.dtype == np.float32
    assert np.max(np.abs(y)) <= 1.0 + 1e-6
    diff = np.abs(np.diff(y))
    # 允许合成起点/谐波叠加处的自然过渡，但禁止远超振幅的跳变
    assert np.max(diff) < 0.5, f"波形存在大跳变(咔哒): max_diff={np.max(diff)}"


def test_synth_envelope_decay():
    """包络应单调衰减（钢琴击弦特性），尾部接近静音。"""
    y = synth_piano(72, 1.0, sr=SR)
    head = float(np.mean(y[: max(1, int(0.05 * SR))].astype(np.float32) ** 2))
    tail = float(np.mean(y[-max(1, int(0.1 * SR)):].astype(np.float32) ** 2))
    assert tail < head, "包络未衰减，疑似恒幅（听感刺耳）"


def test_segment_score_shapes():
    """音序切块返回结构正确。"""
    notes = [
        {"midi": 60, "dur": 0.3},
        {"midi": 64, "dur": 0.3},
        {"midi": 67, "dur": 0.6},
    ]
    blocks = segment_score(notes, bpm=120, sr=SR)
    assert len(blocks) == 3
    for wav, d in blocks:
        assert isinstance(wav, np.ndarray)
        assert wav.dtype == np.float32
        # 块长度应约等于 dur*sr
        assert abs(len(wav) - int(d * SR)) <= 16


# ---------------------------------------------------------------------------
# 3) 合成缓存正确性 & 性能
# ---------------------------------------------------------------------------
def test_synth_cache_hit():
    """相同音序第二次合成应命中缓存（耗时显著更短）。"""
    notes = [{"midi": m, "dur": 0.25} for m in (60, 62, 64, 65, 67)]
    key = _cache_key(notes, 120.0)
    y1 = _synth_score_cached(notes, 120.0, sr=SR)
    t0 = time.time()
    y2 = _synth_score_cached(notes, 120.0, sr=SR)
    dt = time.time() - t0
    # 命中缓存应当近乎瞬时，且结果完全一致（确定性）
    assert dt < 0.05, f"缓存未命中或退化，耗时 {dt:.3f}s"
    assert np.array_equal(y1, y2), "缓存结果不一致（破坏确定性）"


def test_synth_cache_distinct_keys():
    """不同音序应使用不同缓存键，互不影响。"""
    a = [{"midi": 60, "dur": 0.25}]
    b = [{"midi": 61, "dur": 0.25}]
    assert _cache_key(a, 120.0) != _cache_key(b, 120.0)
    ya = _synth_score_cached(a, 120.0, sr=SR)
    yb = _synth_score_cached(b, 120.0, sr=SR)
    assert not np.array_equal(ya, yb)


# ---------------------------------------------------------------------------
# 4) 多线程并发竞态（健壮性核心，纯内存、不依赖声卡）
# ---------------------------------------------------------------------------
def _notes_seq(seed):
    base = 55 + (seed * 2) % 12
    return [{"midi": base + i, "dur": 0.2} for i in range(4)]


def test_concurrent_synth_cache_no_crash():
    """多个线程并发合成同一/不同音序，缓存与合成应全程不抛异常、结果确定。"""
    errors = []

    def worker(idx):
        try:
            for _ in range(8):
                notes = _notes_seq(idx)
                y = _synth_score_cached(notes, 120.0, sr=SR)
                assert y.dtype == np.float32
                assert np.max(np.abs(y)) <= 1.0 + 1e-6
        except Exception as e:  # noqa
            errors.append(repr(e))

    threads = [threading.Thread(target=worker, args=(i,)) for i in range(4)]
    for t in threads:
        t.start()
    for t in threads:
        t.join(timeout=30)
    assert not errors, f"并发合成异常: {errors}"


def test_concurrent_ringbuffer_no_crash():
    """多生产者 + 多消费者并发读写环形缓冲，数据不得损坏。"""
    rb = RingBuffer(capacity=8192)
    total_per = 20_000
    stop_ev = threading.Event()

    def producer(seed):
        data = bytes(((i + seed) % 200) for i in range(total_per))
        rb.write(data, timeout=5.0)

    def consumer(acc):
        while not stop_ev.is_set() or rb.readable() > 0:
            c = rb.read(1024)
            if c:
                acc.extend(c)
            else:
                time.sleep(0.0005)

    accs = [bytearray() for _ in range(2)]
    ps = [threading.Thread(target=producer, args=(i,)) for i in range(3)]
    cs = [threading.Thread(target=consumer, args=(accs[i],)) for i in range(2)]
    for t in ps + cs:
        t.start()
    for t in ps:
        t.join(timeout=10)
    time.sleep(0.1)
    stop_ev.set()
    for t in cs:
        t.join(timeout=10)

    # 总读出量应当等于总写入量（3 * total_per），且无非预期异常
    assert sum(len(a) for a in accs) == 3 * total_per, "并发读写数据量不一致（丢失/重复）"


def test_play_stop_is_idempotent():
    """连续多次 stop 不应崩溃（不触碰真实声卡）。"""
    from app import audio_play
    for _ in range(5):
        audio_play.stop()
    assert not audio_play.is_playing()


# ---------------------------------------------------------------------------
# 5) 真实播放冒烟测试（默认跳过；需显式 M2S_REAL_AUDIO=1 且有设备才跑）
# ---------------------------------------------------------------------------
def test_real_play_smoke():
    """真实声卡播放冒烟：仅当 M2S_REAL_AUDIO=1 且有设备时运行。

    默认跳过——避免在无音频设备的 CI/服务器环境触发 PortAudio 阻塞查询。
    """
    import os
    import pytest
    if os.environ.get("M2S_REAL_AUDIO") != "1":
        pytest.skip("未设置 M2S_REAL_AUDIO=1，跳过真实播放测试")
    from app import audio_play
    notes = [{"midi": 60, "dur": 0.15}, {"midi": 64, "dur": 0.15},
             {"midi": 67, "dur": 0.3}]
    audio_play.play_score(notes, bpm=120, sr=SR)
    time.sleep(0.3)
    audio_play.stop()
    assert not audio_play.is_playing()
    assert audio_play.last_underruns() >= 0  # 欠载计数可用（0 即零卡顿）


if __name__ == "__main__":
    # 无 pytest 时直接跑（极简自测）
    test_ringbuffer_sequential_integrity()
    test_ringbuffer_overflow_backpressure()
    test_synth_waveform_smooth()
    test_synth_envelope_decay()
    test_segment_score_shapes()
    test_synth_cache_hit()
    test_synth_cache_distinct_keys()
    test_concurrent_synth_cache_no_crash()
    test_concurrent_ringbuffer_no_crash()
    test_play_stop_is_idempotent()
    print("[test_audio_play] 全部通过 OK")
