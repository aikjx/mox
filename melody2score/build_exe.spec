# -*- mode: python ; coding: utf-8 -*-
# Melody2Score 桌面 GUI 一键打包配置（Windows / PyQt5 + torch CPU）
# 目录结构保持 melody2score/{app,audio,core,...}，exe 落在 app/gui.exe，
# 使 gui.py 中的 ROOT = dirname(dirname(__file__)) 仍能正确指向分发根目录。
import os

ROOT = os.path.abspath(SPECPATH)

block_cipher = None

# 运行时需要但位于函数内/条件导入、PyInstaller 抓不到的模块
hiddenimports = [
    "torch", "torchcrepe", "crepe_onnx", "librosa", "onnxruntime",
    "sounddevice", "soundfile", "music21", "numba", "pyaudio",
    "numpy", "scipy", "soundfile", "PyQt5", "PyQt5.QtCore",
    "PyQt5.QtGui", "PyQt5.QtWidgets",
]

# 随包分发的数据：原样保留目录结构
datas = [
    ("app", "app"),
    ("core", "core"),
    ("audio", "audio"),
    ("requirements.txt", "."),
]
# 把 audio 下所有 .wav / manifest 一并打包（Tree 会递归收集）
from PyInstaller.utils.hooks import collect_data_files  # noqa: E402

a = Analysis(
    [os.path.join(ROOT, "app", "gui.py")],
    pathex=[ROOT],
    binaries=[],
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        # 排除不需要的 torch 后端以缩小体积（仅 CPU 推理）
        "torch.cuda", "torch.cuda.amp", "torch.backends.cudnn",
        "torch.distributed", "torch._C._distributed_c10d",
        "torch.utils.tensorboard",
        # 开发板用的 alsa / 测试框架无关
        "pytest",
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name=os.path.join("app", "Melody2Score"),
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,            # 桌面 GUI：不弹黑窗口
    disable_windowed_traceback=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name="Melody2Score",
)
