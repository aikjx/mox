"""Regression checks for lossless playback and acoustic note boundaries."""
import io
import os
import sys
import time
import threading
from pathlib import Path

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import numpy as np
import soundfile as sf


def test_slow_consumer_never_drops_pcm(monkeypatch):
    from app import audio_play as audio
    class Pool:
        def __init__(self):
            self.session = None
        def key(self, *args):
            return args
        def ensure(self, *args, **kwargs):
            return (), True
        def assign(self, key, session):
            self.session = session
        def current(self, key):
            return self.session
    pool = Pool()
    monkeypatch.setattr(audio, "_STREAM_POOL", pool)
    player = audio._ScorePlayer()
    payload = bytes(range(256)) * 80
    # Deliberately oversized first block, then stall beyond the old 250ms timeout.
    player.play(iter([payload]), sr=1000)
    deadline = time.monotonic() + 2
    while pool.session is None and time.monotonic() < deadline:
        time.sleep(.001)
    session = pool.session
    assert session is not None
    time.sleep(.35)
    got = bytearray()
    deadline = time.monotonic() + 4
    while len(got) < len(payload) and time.monotonic() < deadline:
        got.extend(session.ring.read(997))
        time.sleep(.001)
    player.stop()
    assert bytes(got) == payload


def test_cancel_before_prefill_cannot_restart(monkeypatch):
    from app import audio_play as audio
    assigned = []
    monkeypatch.setattr(audio._STREAM_POOL, "ensure", lambda *a, **k: ((), True))
    monkeypatch.setattr(audio._STREAM_POOL, "assign", lambda k, s: assigned.append(s))
    entered, release = threading.Event(), threading.Event()
    def chunks():
        entered.set()
        release.wait(2)
        yield bytes(100)
    player = audio._ScorePlayer()
    player.play(chunks(), sr=16000)
    assert entered.wait(1)
    session = player._session
    player.stop()
    release.set()
    assert session.finished_ev.wait(2)
    assert session not in assigned


def test_decode_fallback_preserves_duration_pitch_and_mono(monkeypatch):
    import librosa
    from core.pipeline import load_audio_bytes
    sr = 48000
    t = np.arange(sr) / sr
    y = np.sin(2 * np.pi * 440 * t).astype(np.float32)
    buf = io.BytesIO()
    sf.write(buf, np.column_stack([y, y * .5]), sr, format="WAV")
    def fail(*args, **kwargs):
        raise RuntimeError("force fallback")
    monkeypatch.setattr(librosa, "load", fail)
    decoded, result_sr = load_audio_bytes(buf.getvalue(), 16000)
    assert result_sr == 16000 and decoded.shape == (16000,)
    peak = np.argmax(abs(np.fft.rfft(decoded)))
    assert abs(peak - 440) <= 1
    assert abs(np.max(decoded) - .75) < .02


def test_fast_repeated_notes_survive_entire_parse_chain():
    from core.analysis import segment_notes
    from core.pipeline import _consensus
    from core.postprocess import postprocess_notes
    from core.config import Config
    points = [{"t": i / 100, "freq": 440., "conf": .99} for i in range(61)]
    notes = segment_notes(points, min_note_dur=.1, onset_times=[.2, .4])
    assert len(notes) == 3
    merged, _ = _consensus([notes, notes, notes], Config())
    final = postprocess_notes(merged)["notes"]
    assert [n["midi"] for n in final] == [69, 69, 69]
    assert np.allclose([n["start"] for n in final], [0, .2, .4])


def test_silence_boundaries_survive_consensus():
    from core.pipeline import _consensus
    from core.config import Config
    notes = [{"midi": 60, "start": 0, "end": .15, "sep_prev": True},
             {"midi": 60, "start": .19, "end": .34, "sep_prev": True}]
    merged, _ = _consensus([notes] * 3, Config())
    assert len(merged) == 2


def test_native_player_controls_and_source_switch():
    from PyQt5.QtWidgets import QApplication
    from app.original_player import OriginalPlayer
    app = QApplication.instance() or QApplication([])
    widget = OriginalPlayer()
    path = Path(__file__).resolve().parents[1] / "audio/m00_instrument_piano.wav"
    widget.set_source(path=str(path))
    assert Path(widget.media.media().canonicalUrl().toLocalFile()).resolve() == path.resolve()
    widget._duration(65000)
    widget._position(23000)
    assert widget.clock.text().startswith("00:23 / ")
    assert widget.seek.maximum() == 65000
    widget.volume.setValue(37)
    assert widget.media.volume() == 37
    widget.set_source(data=path.read_bytes())
    temporary = widget._temporary.fileName()
    assert Path(temporary).exists()
    widget.set_source(path=str(path))
    assert not Path(temporary).exists()
    widget.close()
    widget.deleteLater()
    app.processEvents()


def test_real_high_register_audio_preserves_absolute_pitch():
    from core.pipeline import Melody2Score
    from core.config import Config
    sr = 16000
    parts = []
    for midi in (76, 78, 79):
        t = np.arange(sr // 2) / sr
        envelope = np.minimum(1, t / .02) * np.minimum(1, (.5 - t) / .02)
        parts.extend([(.5 * envelope * np.sin(2 * np.pi * 440 * 2 ** ((midi - 69) / 12) * t)),
                      np.zeros(sr // 10)])
    y = np.concatenate(parts).astype(np.float32)
    cfg = Config(preferred_backend="pyin", robust=False, enable_denoise=False,
                 enable_separation=False, enable_vad=False, vocal_mode=False,
                 enable_onset=False, fmin=200, fmax=1100)
    result = Melody2Score(cfg).recognize({"kind": "array", "y": y, "sr": sr})
    assert [n["midi"] for n in result["notes"]] == [76, 78, 79]
    assert result["octave_shift"] == 0


def test_sample_audio_endpoint_rejects_traversal():
    from app.webui import sample_audio
    from fastapi import HTTPException
    import pytest
    response = sample_audio("m00_instrument_piano.wav")
    assert Path(response.path).is_file()
    with pytest.raises(HTTPException) as error:
        sample_audio("../README.md")
    assert error.value.status_code == 404
