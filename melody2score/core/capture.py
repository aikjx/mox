# -*- coding: utf-8 -*-
"""采集层：文件加载 + 现场录音（pyaudio / arecord-alsa）。"""
import os
from typing import Optional

import numpy as np
import librosa


def load_audio(path: str, sr: int = 16000) -> np.ndarray:
    """加载音频（mp3/wav/flac 均可），重采样为 16kHz 单声道。"""
    if not os.path.exists(path):
        raise FileNotFoundError(path)
    y, _ = librosa.load(path, sr=sr, mono=True)
    return y.astype(np.float32)


def record(seconds: int = 5, sr: int = 16000, device: Optional[int] = None,
           backend: str = "portaudio") -> np.ndarray:
    """现场录音。backend='portaudio'（默认，开发板即 alsa 后端）；
    backend='arecord' 走系统 arecord 命令，适合无 pyaudio 的精简系统。"""
    if backend == "arecord":
        return _record_arecord(seconds, sr)
    return _record_portaudio(seconds, sr, device)


def _record_portaudio(seconds: int, sr: int, device: Optional[int]) -> np.ndarray:
    import pyaudio
    pa = pyaudio.PyAudio()
    idx = device if device is not None else pa.get_default_input_device_info()['index']
    stream_in = pa.open(format=pyaudio.paInt16, channels=1, rate=sr,
                        input=True, input_device_index=idx, frames_per_buffer=2048)
    n = int(sr * seconds)
    raw = np.empty(n, dtype=np.int16)
    filled = 0
    print(f"[record] 录音 {seconds}s ...")
    while filled < n:
        chunk = stream_in.read(min(2048, n - filled))
        arr = np.frombuffer(chunk, dtype=np.int16)
        raw[filled:filled + len(arr)] = arr
        filled += len(arr)
    stream_in.stop_stream()
    stream_in.close()
    pa.terminate()
    return (raw.astype(np.float32) / 32768.0)


def _record_arecord(seconds: int, sr: int) -> np.ndarray:
    import subprocess
    import tempfile
    import soundfile as sf
    tmp = tempfile.mktemp(suffix=".wav")
    subprocess.run(["arecord", "-d", str(seconds), "-r", str(sr),
                    "-c", "1", "-f", "S16_LE", tmp], check=True)
    y, _ = sf.read(tmp, sr=sr, dtype="float32", always_2d=False)
    os.remove(tmp)
    return np.asarray(y, dtype=np.float32)
