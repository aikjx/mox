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


def test_jianpu_render_backend(tmp_path):
    """验证简谱图片由第三方库 jianpu-ly + LilyPond 渲染（而非 matplotlib）。"""
    from core import jianpu_render

    notes = [
        {"name": "C4", "midi": 60, "start": 0.0, "dur": 0.5, "lyric": "你"},
        {"name": "D4", "midi": 62, "start": 0.5, "dur": 0.5, "lyric": "好"},
        {"name": "E4", "midi": 64, "start": 1.0, "dur": 1.0, "lyric": "呀"},
    ]
    sheet = score_sheet.make_score_sheet(
        notes=notes, key={"tonic": "C", "mode": "major"},
        bpm=120.0, title="Test", time_sig=(4, 4),
    )
    lilypond = jianpu_render.find_lilypond()
    if lilypond is None or not os.path.exists(jianpu_render.JIANPU_LY):
        import pytest
        pytest.skip("LilyPond / jianpu-ly 未安装，跳过第三方渲染后端验证")

    out = tmp_path / "jianpu.png"
    p = jianpu_render.render_score_sheet(sheet, str(out), dpi=150)
    assert os.path.exists(p)
    assert os.path.getsize(p) > 0
    # PNG 应由 lilypond 生成（尺寸与 matplotlib 默认不同，仅做存在性+非空断言）


def test_jianpu_ly_location_packaged():
    """验证 jianpu-ly 脚本可被正确定位（源码 / 打包双模式）。"""
    from core import jianpu_render
    # 源码运行：JIANPU_LY 应指向仓库 lib/jianpu-ly.py
    assert jianpu_render.JIANPU_LY.endswith(
        os.path.join("lib", "jianpu-ly.py")
    )
    # 打包后（sys._MEIPASS）也应位于 _internal/lib/jianpu-ly.py
    if getattr(sys, "frozen", False):
        assert "lib" in jianpu_render.JIANPU_LY.replace("\\", "/")


def test_export_score_requires_lilypond(monkeypatch, tmp_path):
    """LilyPond 缺失时，export_score 应明确报错（不自写渲染兜底）。"""
    from core import jianpu_render

    monkeypatch.setattr(jianpu_render, "find_lilypond", lambda: None)
    notes = [
        {"name": "C4", "midi": 60, "start": 0.0, "dur": 0.5},
        {"name": "E4", "midi": 64, "start": 0.5, "dur": 0.5},
    ]
    out = tmp_path / "sheet.png"
    with pytest.raises(RuntimeError):
        score_sheet.export_score(
            notes=notes, key={"tonic": "C", "mode": "major"},
            bpm=120.0, output_path=str(out), title="NeedLily",
        )


def test_safe_beats_no_downgrade():
    """量化拍数应原样保留，不把附点八分/十六分并入八分。"""
    from core import jianpu_render
    # 附点八分 0.75 -> 0.75（不变 0.5）
    assert abs(score_sheet._quantize_duration(0.75)[0] - 0.75) < 1e-6
    # 十六分 0.25 -> 0.25（不变 0.5）
    assert abs(score_sheet._quantize_duration(0.25)[0] - 0.25) < 1e-6
    assert abs(jianpu_render._safe_beats(0.75) - 0.75) < 1e-6
    assert abs(jianpu_render._safe_beats(0.25) - 0.25) < 1e-6
    assert abs(jianpu_render._safe_beats(0.5) - 0.5) < 1e-6


def test_dur_token_for_canonical_durations():
    """规范时值应映射到 jianpu-ly 原生记号（含附点八分/十六分）。"""
    from core import jianpu_render
    # 附点八分 -> q1. ；十六分 -> s1 ；八分 -> q1 ；四分 -> 1
    assert jianpu_render._dur_token_for(1, "", 0.75) == "q1."
    assert jianpu_render._dur_token_for(1, "", 0.25) == "s1"
    assert jianpu_render._dur_token_for(1, "", 0.5) == "q1"
    assert jianpu_render._dur_token_for(1, "", 1.0) == "1"
    assert jianpu_render._dur_token_for(1, "", 1.5) == "1."
    assert jianpu_render._dur_token_for(0, "", 0.5) == "q0"
    # 三十二分
    assert jianpu_render._dur_token_for(1, "", 0.125) == "d1"
