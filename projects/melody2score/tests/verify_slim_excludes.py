# -*- coding: utf-8 -*-
"""精简打包预检（构建门禁）：模拟 frozen 环境缺失被排除的模块，跑完整识别链路。

原理：在 sys.meta_path 头部插入 Blocker，令 spec excludes 清单内模块的
真实 import 抛 ImportError（与 PyInstaller excludes 的运行时效果一致），
随后执行与 gui.py --selftest 相同的识别流程 + mp3 解码 + musicxml 输出 +
简谱渲染 + 合成试听验证。全部通过才允许启动正式构建，避免 6 分钟构建
后才发现依赖误排（预检实测曾抓出 librosa→msgpack/joblib、
music21→requests/PIL 四个隐蔽硬依赖）。

排除清单从 build_exe.spec 用 ast 解析读取（单一事实源，防止两处漂移）。
用法：
    python tests/verify_slim_excludes.py        # 独立运行
    build_exe.py 内部自动调用（构建前置门禁）
"""
import ast
import importlib.abc
import importlib.machinery
import json
import os
import sys
import tempfile
import time
import traceback

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, HERE)


def _excludes_from_spec(spec_path):
    """ast 解析 build_exe.spec，提取 Analysis(excludes=[...]) 清单。"""
    tree = ast.parse(open(spec_path, encoding="utf-8").read())
    for node in ast.walk(tree):
        if isinstance(node, ast.Call) and getattr(node.func, "id", "") == "Analysis":
            for kw in node.keywords:
                if kw.arg == "excludes" and isinstance(kw.value, ast.List):
                    return {e.value for e in kw.value.elts
                            if isinstance(e, ast.Constant) and isinstance(e.value, str)}
    raise RuntimeError("spec 中未找到 Analysis(excludes=[...])")


BLOCKED = _excludes_from_spec(os.path.join(HERE, "build_exe.spec"))


class BlockedLoader(importlib.abc.Loader):
    """真正的 import 触发时抛 ImportError（等价于模块缺失）。"""

    def create_module(self, spec):
        raise ImportError(f"blocked (simulating PyInstaller exclude): {spec.name}")

    def exec_module(self, module):
        raise ImportError(f"blocked (simulating PyInstaller exclude): {module.__name__}")


class Blocker(importlib.abc.MetaPathFinder):
    """模拟 PyInstaller excludes 的运行时语义：

    - `import xxx`（真加载）→ ImportError（模块不在包内）
    - `importlib.util.find_spec('xxx')`（探测式，如 music21 对
      matplotlib 的可选依赖探测）→ 返回非 None spec，不炸。
      （music21/base.py 探测后仅在缺失时打警告，从不 import）
    """

    def find_spec(self, name, path=None, target=None):
        if name.split(".")[0] in BLOCKED or name in BLOCKED:
            return importlib.machinery.ModuleSpec(name, BlockedLoader())
        return None


sys.meta_path.insert(0, Blocker())

ok = True


def check(label, fn):
    global ok
    try:
        r = fn()
        print(f"  [PASS] {label}" + (f" -> {r}" if r is not None else ""))
    except Exception:
        ok = False
        print(f"  [FAIL] {label}")
        traceback.print_exc()


print("=" * 60)
print(f" 精简打包预检（模拟 {len(BLOCKED)} 项 excludes 缺失环境）")
print("=" * 60)

# 1) 完整识别链路（等价 selftest）
from core.config import Config
from core.paths import resource_path
from core import capture
from core.pipeline import Melody2Score

with open(resource_path("audio", "manifest.json"), encoding="utf-8") as f:
    manifest = json.load(f)
item = manifest[0]
wav = resource_path("audio", os.path.basename(item["file"]))
cfg = Config()
t0 = time.time()
y = capture.load_audio(wav, cfg.sr)
res = Melody2Score(cfg).recognize({"kind": "array", "y": y, "sr": cfg.sr, "cfg": cfg})
elapsed = round(time.time() - t0, 2)


def _pipeline_ok():
    notes = res.get("notes", [])
    jianpu = res.get("jianpu") or ""
    assert len(notes) == 14, f"音符数异常: {len(notes)}（期望 14）"
    assert jianpu.startswith("1 1 5 5 6 6 5-"), f"简谱异常: {jianpu[:20]}"
    return f"notes=14 bpm={res.get('bpm')} backend={res.get('backend')} {elapsed}s"


check("完整识别链路（加载→预处理→pyin→解析→简谱）", _pipeline_ok)

# 2) musicxml 输出（music21 依赖验证：排除 matplotlib/sympy/lxml 等后是否可用）
def _musicxml_ok():
    from core import score
    tmp = os.path.join(tempfile.mkdtemp(prefix="mxchk_"), "out.musicxml")
    score.to_musicxml(res["notes"], res.get("bpm", 120),
                      (res["key"]["tonic"], res["key"]["mode"]), fp=tmp)
    mx = open(tmp, encoding="utf-8").read()
    assert "part-list" in mx and "<note" in mx, "musicxml 输出缺失/异常"
    return f"{len(mx)} chars, contains part-list/note"


check("music21 musicxml 输出", _musicxml_ok)

# 3) mp3 解码（排除 av 后由 libsndfile 原生支持）
def _mp3_ok():
    import numpy as np
    import soundfile as sf
    tmp = os.path.join(tempfile.mkdtemp(prefix="mp3chk_"), "t.mp3")
    t = np.arange(16000) / 16000.0
    sf.write(tmp, 0.5 * np.sin(2 * np.pi * 440.0 * t).astype(np.float32), 16000)
    y2 = capture.load_audio(tmp, cfg.sr)
    assert len(y2) > 15000, f"mp3 解码异常: len={len(y2)}"
    os.remove(tmp)
    return f"mp3 decode OK ({len(y2)} samples)"


check("mp3 解码（libsndfile 原生）", _mp3_ok)

# 4) 简谱图片渲染链路（jianpu_render + LilyPond 外部进程）
def _render_ok():
    from core.score_sheet import ScoreSheet, RenderNote
    from core.jianpu_render import render_score_sheet
    sheet = ScoreSheet(
        title="预检", key_tonic="C", key_mode="major", time_sig=(4, 4),
        bpm=120, beats_per_bar=4,
        notes=[RenderNote("C4", 60, 1, 0, 0.0, 1.0, 0),
               RenderNote("D4", 62, 2, 0, 1.0, 1.0, 0),
               RenderNote("E4", 64, 3, 0, 2.0, 1.5, 0),
               RenderNote("F4", 65, 4, 0, 3.5, 0.5, 0)])
    out = os.path.join(tempfile.mkdtemp(prefix="lychk_"), "r.png")
    render_score_sheet(sheet, out, dpi=100)
    assert os.path.getsize(out) > 5000, "渲染产物异常"
    return f"png {os.path.getsize(out)} bytes"


check("简谱图片渲染（jianpu-ly + LilyPond）", _render_ok)

# 5) 音频合成（GUI 试听功能）
def _synth_ok():
    from core.synth import synth_piano
    import numpy as np
    y3 = synth_piano(60, 0.2, 16000)
    assert isinstance(y3, np.ndarray) and len(y3) > 3000, "合成异常"
    return f"{len(y3)} samples"


check("钢琴合成（试听）", _synth_ok)

print("=" * 60)
print(" 结论:", "PASS（可安全精简打包）" if ok
      else "FAIL（完整链路未通过，请根据上方 [FAIL] 项定位）")
sys.exit(0 if ok else 1)
