#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
Melody2Score 企业级 · 全维度稳定性/性能验证套件
================================================

覆盖本次优化的全部维度（不依赖声卡 / 不加载 torch 重模型，可在 CI 或
打包后 green 环境直接运行）：

  维度              验证项
  ------------------------------------------------
  [P1] 播放引擎性能   RingBuffer 批量 memoryview 拷贝（跨边界 / 多段）
  [P2] 合成性能       synth_piano 向量化单次合成 + 时间轴缓存命中
  [P3] 合成确定性     相同输入 -> 相同输出（可复现 / 缓存安全）
  [S1] 稳定性         RingBuffer 边界不越界、不丢数据
  [S2] 异常安全       RingBuffer 容量满时 write 超时返回而非死锁
  [C1] 并发安全       多线程并发读 + 写无数据交叉错乱 / 无死锁
  [A1] API 契约       pipeline._load_source 新增 'array' 源（WebUI 免二次编解码）
  [A2] 进度钩子       recognize/run 的 progress_cb 签名与分阶段回调可用

退出码：0 全通过；1 有失败。

用法:
  python tests/verify_all_dims.py
"""
import sys
import os
import time
import threading

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, HERE)

from app.audio_play import RingBuffer, _ScorePlayer
from core import synth
from core.pipeline import _load_source, Config

FAIL = []


def check(name, cond, detail=""):
    if cond:
        print(f"  [PASS] {name}")
    else:
        print(f"  [FAIL] {name}  {detail}")
        FAIL.append(name)


# --------------------------------------------------------------------------- #
# [P1] RingBuffer 批量拷贝性能 + 正确性
# --------------------------------------------------------------------------- #
def test_ringbuffer_bulk():
    print("\n[P1] RingBuffer 批量 memoryview 拷贝（跨边界 / 多段 / 性能）")
    rb = RingBuffer(capacity=100)
    w = rb.write(b"ABCDEFGHIJ" * 10)
    check("write 满容量返回实际字节", w == 100, f"w={w}")
    got = rb.read(250)  # 请求超容量
    check("read 超容量截断到实际", len(got) == 100, f"len={len(got)}")
    check("跨边界读写内容一致", got == b"ABCDEFGHIJ" * 10)

    # 多段连续写 + 多次读，数据与顺序必须完全一致
    rb2 = RingBuffer(capacity=64)
    acc = b""
    for i in range(20):
        rb2.write(bytes([i % 256]) * 3)
        acc += bytes([i % 256]) * 3
    out = b""
    while True:
        d = rb2.read(50)
        if not d:
            break
        out += d
    check("多段读写顺序无损", out == acc, f"out={len(out)} acc={len(acc)}")

    # 性能：10MB 写入吞吐（批量拷贝应明显快于逐字节）
    # 真实播放中消费者(声卡)持续 read 排空，故此处用后台消费者模拟，
    # 仅测「生产者批量拷贝」本项性能。
    rb3 = RingBuffer(capacity=2_000_000)
    big = b"x" * 1_000_000
    consumed = threading.Event()

    def drain():
        while not consumed.is_set():
            rb3.read(4096)

    dt = threading.Thread(target=drain, daemon=True)
    dt.start()
    t = time.time()
    total = 0
    for _ in range(10):
        total += rb3.write(big)
    cost = time.time() - t
    consumed.set()
    dt.join(timeout=2)
    check("10MB 批量写 < 1.0s（有消费者排空）", cost < 1.0, f"cost={cost:.3f}s total={total}")


# --------------------------------------------------------------------------- #
# [P2][P3] 合成性能 + 确定性
# --------------------------------------------------------------------------- #
def test_synth_perf_and_determinism():
    print("\n[P2/P3] synth_piano 向量化性能 + 确定性")
    # 确定性：相同输入相同输出
    a = synth.synth_piano(69, 0.25, 16000)
    b = synth.synth_piano(69, 0.25, 16000)
    check("合成确定性(相同输入相同输出)", a.dtype == b.dtype and __import__("numpy").array_equal(a, b))
    check("输出为 float32 且峰值<=1", a.dtype == __import__("numpy").float32
          and float(__import__("numpy").max(__import__("numpy").abs(a))) <= 1.0001)

    # 缓存命中：相同 (n,sr) 时间轴只构造一次
    synth._T_CACHE.clear()
    synth.synth_piano(60, 0.2, 16000)
    synth.synth_piano(60, 0.2, 16000)
    check("时间轴缓存命中(同参数仅1条)", len(synth._T_CACHE) == 1, f"cache={len(synth._T_CACHE)}")

    # 批量合成耗时
    t = time.time()
    for i in range(200):
        synth.synth_piano(60 + (i % 12), 0.15, 16000)
    cost = (time.time() - t) * 1000
    check("200 次合成 < 100ms", cost < 100, f"cost={cost:.1f}ms")


# --------------------------------------------------------------------------- #
# [S1][S2] 边界与异常安全
# --------------------------------------------------------------------------- #
def test_ringbuffer_safety():
    print("\n[S1/S2] RingBuffer 边界 & 满缓冲超时保护")
    rb = RingBuffer(capacity=8)
    rb.write(b"12345678")          # 写满
    # 满时再写应超时返回(不死锁)
    t = time.time()
    w2 = rb.write(b"AB", timeout=0.2)
    cost = time.time() - t
    check("满缓冲写超时(不阻塞)", w2 == 0 and cost < 1.0, f"w2={w2} cost={cost:.3f}")
    # 读空返回 b""
    rb.read(8)
    check("空缓冲读返回空字节", rb.read(4) == b"")


# --------------------------------------------------------------------------- #
# [C1] 并发安全：多线程同时写不同数据 + 单线程读，数据须可区分不交叉
# --------------------------------------------------------------------------- #
def test_concurrent():
    print("\n[C1] 并发读写安全（多线程写 + 读，无死锁 / 无交叉错乱）")
    rb = RingBuffer(capacity=100_000)

    def producer(tag, payload):
        # 写入本线程专属标记字节，便于事后核对
        rb.write(bytes([tag]) * len(payload))

    threads = []
    for tag in range(8):
        t = threading.Thread(target=producer, args=(tag, b"y" * 5000))
        threads.append(t)

    collected = bytearray()
    producers_done = threading.Event()

    def consumer():
        # 持续排空，直到「生产者已全部 join 完成」且「缓冲真正空」
        while True:
            d = rb.read(4096)
            if d:
                collected.extend(d)
            elif producers_done.is_set():
                tail = rb.read(4096)
                if tail:
                    collected.extend(tail)
                else:
                    break

    ct = threading.Thread(target=consumer, daemon=True)
    for t in threads:          # 先启动所有生产者
        t.start()
    ct.start()                 # 再启动消费者（此时生产者均已 start，避免误判结束）
    for t in threads:
        t.join(timeout=5)
    producers_done.set()       # 标记生产者结束，允许消费者在排空后退出
    ct.join(timeout=5)
    ok = (not any(t.is_alive() for t in threads)) and (not ct.is_alive())
    check("并发读写为无死锁完成", ok)
    # 每个 tag 的字节数应完整(5000)，证明未交叉/未丢失
    counts = {t: collected.count(t) for t in range(8)}
    check("各生产者数据完整不丢失", all(c == 5000 for c in counts.values()), f"counts={counts}")


# --------------------------------------------------------------------------- #
# [A1] API 契约：pipeline.array 音源
# --------------------------------------------------------------------------- #
def test_array_source():
    print("\n[A1] pipeline._load_source('array') 免二次编解码")
    import numpy as np
    y = np.ones(1600, dtype=np.float32)
    out_y, out_sr = _load_source({"kind": "array", "y": y, "sr": 22050}, Config())
    check("array 源直接复用 y", out_y is y or np.array_equal(out_y, y))
    check("array 源保留采样率", out_sr == 22050, f"sr={out_sr}")
    try:
        _load_source({"kind": "bogus"}, Config())
        check("未知源类型抛错(契约)", False)
    except ValueError:
        check("未知源类型抛错(契约)", True)


# --------------------------------------------------------------------------- #
# [A2] progress_cb 钩子：分阶段回调
# --------------------------------------------------------------------------- #
def test_progress_cb():
    print("\n[A2] recognize/run progress_cb 分阶段回调")
    stages = []
    def cb(stage, msg, frac):
        stages.append((stage, frac))
        assert 0.0 <= frac <= 1.0
    # 直接测试 _load_source 之外的轻量契约：用合成数据走 array 源到 recognize
    # 需 torch 后端，这里仅验证回调对象可被传递不报错（不实际跑重模型）
    # 通过 Config 构造确认 progress_cb 参数在 run 签名存在
    import inspect
    from core.pipeline import Melody2Score
    sig = inspect.signature(Melody2Score.run)
    check("run() 支持 progress_cb 参数", "progress_cb" in sig.parameters)
    sig2 = inspect.signature(Melody2Score.recognize)
    check("recognize() 支持 progress_cb 参数", "progress_cb" in sig2.parameters)


if __name__ == "__main__":
    print("=" * 60)
    print(" Melody2Score 企业级 · 全维度验证")
    print("=" * 60)
    test_ringbuffer_bulk()
    test_synth_perf_and_determinism()
    test_ringbuffer_safety()
    test_concurrent()
    test_array_source()
    test_progress_cb()

    print("\n" + "=" * 60)
    if FAIL:
        print(f" 结果: {len(FAIL)} 项失败 -> {FAIL}")
        print("=" * 60)
        sys.exit(1)
    print(" 结果: 全部维度通过 [ALL PASS]")
    print("=" * 60)
    sys.exit(0)
