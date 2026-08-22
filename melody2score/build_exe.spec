# -*- mode: python ; coding: utf-8 -*-
# Melody2Score 桌面 GUI 一键打包配置（Windows / PyQt5 + torch CPU）
# 目录结构：app / audio / core 等数据经 datas 打入 _internal/（PyInstaller 6 onedir），
# exe 落在发行版根目录 Melody2Score.exe。运行期路径统一由 core/paths.py 的
# resource_path() 解析（打包时基于 sys._MEIPASS），保证「内置经典样例」开箱即用。
import os

ROOT = os.path.abspath(SPECPATH)

block_cipher = None

# 运行时需要但位于函数内/条件导入、PyInstaller 抓不到的模块。
# 注意：不含 torch/torchcrepe/crepe_onnx/onnxruntime——这两个音高后端
# 在源码环境就未安装（实测识别链路走 librosa.pyin，8/8 精确匹配），
# pitch.py 的后端探测 try/except 会优雅降级到 pyin，打入纯属死重量。
hiddenimports = [
    "librosa", "sounddevice", "soundfile", "music21", "numba",
    "numpy", "scipy", "PyQt5", "PyQt5.QtCore",
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
# Tree 元素为 (dest, src, typecode)：
#   dest = 包内相对路径（如 "audio/m00_xxx.wav"）
#   src  = 源文件绝对路径
# datas 元素为 (src, dest_dir)：把 src 文件拷入 dest_dir 目录（保留文件名）。
# 旧版误写 (src, dest)——把目标文件路径当目标目录，COLLECT 为每个 wav
# 创建了同名【目录】再往里塞文件 → 发行版样例全部不可用（P0 缺陷）。
# 正确转换：目标目录 = dest 去掉文件名（对嵌套子目录同样成立）。
datas += [(src, os.path.dirname(dest) or ".")
          for dest, src, *_ in audio_tree]

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
        # ============================================================
        # 精简打包：以下均为与本桌面应用无关的死重量。
        # 共享 site-packages 装有 AI 全家桶，torch 生态 hook 会连带收集，
        # 必须显式排除（实测排除后 selftest + 真实音频 8/8 回归全绿）。
        # ============================================================
        # -- 音高检测后端（源码环境未安装 crepe_onnx/torchcrepe 两个包，
        #    实际链路 librosa.pyin；探测失败优雅降级）--
        "torch", "torchvision", "torchaudio", "torchcrepe", "crepe_onnx",
        "onnxruntime",
        # -- 深度学习/AI 框架全家桶 --
        "tensorflow", "keras", "tensorboard", "tensorboard_data_server",
        "paddle", "paddlepaddle",
        "transformers", "tokenizers", "diffusers", "accelerate", "peft",
        "timm", "sentencepiece", "tiktoken", "safetensors",
        "huggingface_hub",
        # -- 视觉/NLP/数据科学（核心链路 librosa+music21 均不依赖；
        #    PIL 除外——music21.lily 顶层探测 PIL.Image，缺失会崩）--
        "cv2", "opencv", "pandas", "sklearn", "scikit-learn",
        "matplotlib",
        "nltk", "jieba", "emoji", "ftfy",
        # -- 多媒体解码（mp3 由 libsndfile>=1.1.0 原生支持，实测 1.2.2）--
        "av",
        # -- 云服务/网络 SDK --
        "boto3", "botocore", "s3transfer",
        "grpc", "google", "protobuf", "opentelemetry",
        "redis", "zmq", "tornado", "aiohttp", "websockets",
        "cryptography", "nacl", "bcrypt",
        # -- Web 框架（webui/enterprise_api 是独立服务部署形态，
        #    不随桌面绿色发行版分发；入口 gui.py 不 import 它们）--
        "fastapi", "starlette", "uvicorn", "pydantic", "anyio",
        "httptools", "watchfiles", "orjson",
        # -- 开发/交互工具 --
        "IPython", "jedi", "parso", "prompt_toolkit", "mypy",
        "Cython", "coverage", "pyreadline3", "nbformat", "jsonschema",
        "altair", "rich", "pytest",
        # -- 杂项（music21 的 musicxml 输出走标准库 ElementTree）--
        "h5py", "sqlalchemy", "lxml", "gmpy2", "sympy",
        "lz4", "jsonpickle",
        "win32", "win32com", "Pythonwin", "pywin32",
        # -- GUI 发行版不用 tkinter --
        "tkinter", "_tkinter",
        # -- PyQt5 无关子模块（本应用仅用 Core/Gui/Widgets）--
        "PyQt5.QtWebEngineWidgets", "PyQt5.QtWebChannel", "PyQt5.QtWebKit",
        "PyQt5.QtWebKitWidgets", "PyQt5.QtQuick", "PyQt5.QtQml",
        "PyQt5.QtQuickWidgets", "PyQt5.QtQuickControls2",
        "PyQt5.QtBluetooth", "PyQt5.QtSerialPort", "PyQt5.QtNfc",
        "PyQt5.QtLocation", "PyQt5.QtPositioning", "PyQt5.QtSensors",
        "PyQt5.QtMultimedia", "PyQt5.QtMultimediaWidgets",
        "PyQt5.QtSql", "PyQt5.QtTest", "PyQt5.QtDesigner", "PyQt5.QtHelp",
        "PyQt5.QtDBus", "PyQt5.QtChart", "PyQt5.QtDataVisualization",
        "PyQt5.QtOpenGL", "PyQt5.QtScript", "PyQt5.QtSvg",
        "PyQt5.QtXml", "PyQt5.QtNetwork", "PyQt5.QtPrintSupport",
        "PyQt5.QtXmlPatterns", "PyQt5.QtWinExtras",
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
