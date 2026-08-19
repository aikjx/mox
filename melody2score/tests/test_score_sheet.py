# -*- coding: utf-8 -*-
"""score_sheet 模块单元测试。"""
import os
import sys
import tempfile

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

from core import score_sheet


def test_midi_to_degree():
    # C 大调：C=1, D=2, E=3, F=4, G=5, A=6, B=7
    tonic = score_sheet._name_to_pitch_class("C")
    assert score_sheet._midi_to_degree(60, tonic) == 1  # C4
    assert score_sheet._midi_to_degree(64, tonic) == 3  # E4
    assert score_sheet._midi_to_degree(67, tonic) == 5  # G4


def test_quantize_duration():
    assert score_sheet._quantize_duration(1.0)[1] == "quarter"
    assert score_sheet._quantize_duration(0.5)[1] == "eighth"
    assert score_sheet._quantize_duration(2.0)[1] == "half"


def test_make_score_sheet():
    notes = [
        {"name": "C4", "midi": 60, "start": 0.0, "dur": 0.5},
        {"name": "D4", "midi": 62, "start": 0.5, "dur": 0.5},
        {"name": "E4", "midi": 64, "start": 1.0, "dur": 0.5},
        {"name": "C4", "midi": 60, "start": 1.5, "dur": 0.5},
    ]
    sheet = score_sheet.make_score_sheet(
        notes=notes,
        key={"tonic": "C", "mode": "major"},
        bpm=120.0,
        title="测试旋律",
        time_sig=(4, 4),
    )
    assert sheet.title == "测试旋律"
    assert sheet.key_tonic == "C"
    assert sheet.beats_per_bar == 4
    assert len(sheet.notes) == len(notes)
    assert sheet.notes[0].degree == 1
    assert sheet.notes[1].degree == 2


def test_export_score(tmp_path):
    notes = [
        {"name": "C4", "midi": 60, "start": 0.0, "dur": 0.5},
        {"name": "E4", "midi": 64, "start": 0.5, "dur": 0.5},
        {"name": "G4", "midi": 67, "start": 1.0, "dur": 0.5},
        {"name": "C5", "midi": 72, "start": 1.5, "dur": 0.5},
    ]
    for fmt in ("png", "pdf", "svg"):
        out = tmp_path / f"sheet.{fmt}"
        path = score_sheet.export_score(
            notes=notes,
            key={"tonic": "C", "mode": "major"},
            bpm=120.0,
            output_path=str(out),
            title="Test",
        )
        assert os.path.exists(path)
        assert os.path.getsize(path) > 0


def test_export_score_empty(tmp_path):
    out = tmp_path / "empty.png"
    path = score_sheet.export_score(
        notes=[],
        key={"tonic": "C", "mode": "major"},
        bpm=120.0,
        output_path=str(out),
        title="Empty",
    )
    assert os.path.exists(path)
