# -*- coding: utf-8 -*-
"""路径解析助手：在「源码运行」与「PyInstaller 打包」两种模式下，统一定位随包资源。

背景
----
melody2score 打包为绿色版时（见 build_exe.py / build_exe.spec），
`audio/`（内置样例）、`core/`、`app/` 等数据目录会被 PyInstaller 放进
`dist/Melody2Score/_internal/`（onedir 模式；PyInstaller 会把该目录暴露为
`sys._MEIPASS`）。

旧代码用 `ROOT = dirname(dirname(abspath(__file__)))` 去解析 `audio/manifest.json`：
  - 源码运行：__file__ = .../melody2score/core/paths.py → ROOT = melody2score/ ✓
  - 打包运行：__file__ = .../_internal/core/paths.pyc → ROOT = _internal/ ，
    但 exe 在 dist/Melody2Score/，audio 也在 _internal/ 下 → 实际解析出的路径仍
    指向 _internal/，且样例加载需基于 `_MEIPASS` 定位，否则会出现「无样例」。

本模块统一提供：
  - resource_root(): 随包数据所在根目录（源码=工程根；打包=sys._MEIPASS）。
  - resource_path(*parts): 定位到随包资源的绝对路径。
"""
import os
import sys


def _frozen() -> bool:
    """是否为 PyInstaller 打包运行环境。"""
    return getattr(sys, "frozen", False)


def resource_root() -> str:
    """随包数据（audio/ 等）所在根目录。

    打包时返回 sys._MEIPASS（PyInstaller onedir = dist/Melody2Score/_internal/）；
    源码运行返回工程根（melody2score/）。
    """
    if _frozen():
        mp = getattr(sys, "_MEIPASS", None)
        if mp:
            return str(mp)
    return os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def resource_path(*parts: str) -> str:
    """定位随包资源的绝对路径，如 resource_path('audio', 'manifest.json')。"""
    return os.path.join(resource_root(), *parts)


def is_frozen() -> bool:
    """是否运行在打包后的可执行文件中。"""
    return _frozen()
