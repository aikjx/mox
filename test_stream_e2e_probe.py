# -*- coding: utf-8 -*-
"""端到端验证：模拟机器人 TCP 推流 + 模拟前端 WS 订阅，验证 live_server 是否送帧。"""
import asyncio
import json
import socket
import struct
import sys
import time

import numpy as np

try:
    import cv2
except ImportError:
    cv2 = None

STREAM_ID = "test_stream_e2e_probe"
HOST = "127.0.0.1"
TCP_PORT = 31111
WS_PORT = 31112


def make_jpeg_with_number(n: int) -> bytes:
    frame = np.full((360, 640, 3), (30, 30, 40), dtype=np.uint8)
    if cv2 is not None:
        cv2.putText(frame, "FRAME-%d" % n, (60, 180), cv2.FONT_HERSHEY_SIMPLEX,
                    2.0, (80 + n * 5 % 160, 220, 220), 3, cv2.LINE_AA)
        ok, buf = cv2.imencode(".jpg", frame, [cv2.IMWRITE_JPEG_QUALITY, 80])
        if ok:
            return buf.tobytes()
    return b"\xff\xd8\xff\xe0" + struct.pack("<I", n) + b"\xff\xd9"


async def main() -> int:
    import aiohttp

    push_sock = socket.create_connection((HOST, TCP_PORT), timeout=5)
    sid = STREAM_ID.encode("utf-8")
    push_sock.sendall(bytes([len(sid)]) + sid)
    print("[tcp] stream registered: %s" % STREAM_ID)

    session = aiohttp.ClientSession()
    ws = await session.ws_connect("ws://%s:%d/ws" % (HOST, WS_PORT))
    await ws.send_str(json.dumps({"stream_id": STREAM_ID}))

    got_first_msg = None
    frame_msgs = 0
    distinct_hashes = set()
    t0 = time.time()

    async def push_frames():
        for i in range(5):
            jpg = make_jpeg_with_number(i)
            push_sock.sendall(struct.pack("<I", len(jpg)) + jpg)
            print("[tcp] pushed frame %d (%d bytes)" % (i, len(jpg)))
            await asyncio.sleep(0.5)

    push_task = asyncio.create_task(push_frames())

    try:
        while time.time() - t0 < 8:
            try:
                msg = await asyncio.wait_for(ws.receive(), timeout=1.0)
            except asyncio.TimeoutError:
                continue
            if msg.type == aiohttp.WSMsgType.TEXT:
                data = json.loads(msg.data)
                if got_first_msg is None:
                    got_first_msg = data
                    print("[ws] first message: %s" % json.dumps(data)[:200])
                if data.get("type") in ("image", "frame"):
                    frame_msgs += 1
                    distinct_hashes.add(str(data.get("hash")))
                    if frame_msgs <= 3 or frame_msgs % 50 == 0:
                        print("[ws] frame msg #%d, hash=%s, payload=%dB" % (
                            frame_msgs, data.get("hash"), len(data.get("data") or "")))
            elif msg.type in (aiohttp.WSMsgType.CLOSED, aiohttp.WSMsgType.ERROR):
                print("[ws] closed/error: %s" % msg)
                break
    finally:
        push_task.cancel()
        dur = time.time() - t0
        print("\n=== RESULT ===")
        print("duration=%.1fs frame_msgs=%d distinct_hashes=%d" % (dur, frame_msgs, len(distinct_hashes)))
        print("first_msg_type=%s" % ((got_first_msg or {}).get("type") if got_first_msg else None))
        rate = frame_msgs / dur if dur else 0
        print("msg_rate=%.0f/s (repeat-storm: %s)" % (rate, "YES" if rate > 20 else "no"))
        try:
            await ws.close()
        except Exception:
            pass
        await session.close()
        push_sock.close()
    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
