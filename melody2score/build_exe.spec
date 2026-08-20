# -*- mode: python ; coding: utf-8 -*-
# Melody2Score 桌面 GUI 一键打包配置（Windows / PyQt5 + torch CPU）
# 目录结构：app / audio / core 等数据经 datas 打入 _internal/（PyInstaller 6 onedir），
# exe 落在发行版根目录 Melody2Score.exe。运行期路径统一由 core/paths.py 的
# resource_path() 解析（打包时基于 sys._MEIPASS），保证「内置经典样例」开箱即用。
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
    ("lib", "lib"),                # jianpu-ly 简谱渲染脚本（LilyPond 预处理器）
    ("requirements.txt", "."),
]
# 用 Tree 递归收集 audio/ 下全部样例（144 个 .wav + manifest.json），
# 确保「内置经典样例」一定随包打入 _internal/audio/，开箱即用。
from PyInstaller.building.build_main import Tree  # noqa: E402

audio_tree = Tree(os.path.join(ROOT, "audio"), prefix="audio")
# Tree 元素为 (dest, src[, typecode])；datas 要求 (src, dest_dir)，
# 故逆序取前两项（PyInstaller 6.22 起元素含 typecode 第三项）
datas += [(src, dest) for dest, src, *_ in audio_tree]

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
    name="Melody2Score",
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
