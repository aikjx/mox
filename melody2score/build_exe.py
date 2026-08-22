#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
Melody2Score 企业级一键打包脚本（纯 Python，跨 Windows 平台）
==============================================================

功能
----
将 melody2score 桌面应用打包为「开箱即用」的绿色发行版：
  * 把 Python 解释器 + 全部依赖（PyTorch CPU / librosa / PyQt5 等）
    与内置样例音频（audio/）一起封入独立文件夹；
  * 目标电脑无需安装 Python 或任何第三方库，双击启动脚本即可运行。

用法
----
  python build_exe.py                 # 标准打包（含依赖自检与安装）
  python build_exe.py --no-deps      # 跳过依赖安装（仅重建可执行文件）
  python build_exe.py --clean        # 打包前清空 build/ 与 dist/
  python build_exe.py --spec SPEC    # 指定自定义 .spec 文件

产物
----
  dist/Melody2Score/
      ├─ Melody2Score.exe            # 主程序（PyInstaller 6 置于根目录）
      ├─ _internal/                  # 运行时依赖与数据（audio/core/app 均在此）
      ├─ 启动Melody2Score.bat         # 双击即运行（目标电脑用）
      └─ README.txt

退出码
------
  0  成功
  1  环境/依赖错误
  2  PyInstaller 打包失败
  3  产物校验失败
"""

import argparse
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

# --------------------------------------------------------------------------- #
# 路径与常量
# --------------------------------------------------------------------------- #
HERE = Path(__file__).resolve().parent
SPEC_DEFAULT = HERE / "build_exe.spec"
LOG_FILE = HERE / "build_exe.log"

# 发行版需要随包分发的数据目录 / 文件（相对 HERE）
# 注：真正打入 exe 的是 build_exe.spec 里的 datas（audio 用 Tree 递归收集，
# 保证 144 个 .wav + manifest.json 全部随包）。此处 BUNDLE_DATA 仅作为
# 打包后校验的基准（目录非空检查 + 样例 wav 数量比对）。
BUNDLE_DATA = [
    ("app", "app"),
    ("core", "core"),
    ("lib", "lib"),
    ("audio", "audio"),
    ("requirements.txt", "."),
]

# 目标电脑运行时必须存在的目录（若缺失则自动创建并给提示）
RUNTIME_DIRS = [
    HERE / "app" / "exports",
]


# --------------------------------------------------------------------------- #
# 日志
# --------------------------------------------------------------------------- #
class Logger:
    """同时写终端与日志文件的简易日志器。"""

    def __init__(self, log_path: Path):
        self._log = open(log_path, "a", encoding="utf-8")

    def _stamp(self) -> str:
        return time.strftime("%Y-%m-%d %H:%M:%S")

    def info(self, msg: str):
        line = f"[{self._stamp()}] INFO  {msg}"
        print(line, flush=True)
        self._log.write(line + "\n")
        self._log.flush()

    def warn(self, msg: str):
        line = f"[{self._stamp()}] WARN  {msg}"
        print(line, flush=True)
        self._log.write(line + "\n")
        self._log.flush()

    def error(self, msg: str):
        line = f"[{self._stamp()}] ERROR {msg}"
        print(line, flush=True)
        self._log.write(line + "\n")
        self._log.flush()

    def close(self):
        try:
            self._log.close()
        except Exception:
            pass


log = Logger(LOG_FILE)


# --------------------------------------------------------------------------- #
# 工具函数
# --------------------------------------------------------------------------- #
def run(cmd, **kwargs) -> int:
    """运行子进程，实时透传输出。"""
    log.info(f"$ {' '.join(str(c) for c in cmd)}")
    kwargs.setdefault("encoding", "utf-8")
    kwargs.setdefault("errors", "replace")
    return subprocess.call(list(cmd), **kwargs)


def pip_install(packages) -> int:
    cmd = [sys.executable, "-m", "pip", "install", "-q", *packages]
    return run(cmd)


def check_another_build_running() -> bool:
    """粗略探测是否有其它 pyinstaller 进程在跑，避免产物冲突。"""
    try:
        out = subprocess.run(
            ["tasklist", "/FI", "IMAGENAME eq pyinstaller.exe"],
            capture_output=True, encoding="utf-8", errors="replace",
        ).stdout
        return "pyinstaller.exe" in out
    except Exception:
        return False


def ensure_runtime_dirs():
    """保证运行期输出目录存在。"""
    for d in RUNTIME_DIRS:
        if not d.exists():
            d.mkdir(parents=True, exist_ok=True)
            log.info(f"已创建运行目录: {d}")


# --------------------------------------------------------------------------- #
# 启动器与说明文档（目标电脑使用）
# --------------------------------------------------------------------------- #
LAUNCHER_BAT = r"""@echo off
chcp 65001 >nul
REM ============================================================
REM  Melody2Score 一键启动（绿色版，无需安装 Python）
REM  若被杀软误报，请将本文件夹加入白名单。
REM ============================================================
setlocal
cd /d "%~dp0"
if exist "Melody2Score.exe" (
    start "" "Melody2Score.exe"
) else (
    echo 未找到 Melody2Score.exe，请确认解压完整。
    pause
)
"""

README_TXT = """============================================================
 Melody2Score 绿色发行版（开箱即用）
============================================================

本文件夹是一个独立的桌面应用，已把 Python 运行环境、全部依赖
（PyTorch CPU / librosa / PyQt5 等）和内置样例音频一并打包。

【在其他电脑运行】
1. 把整个 Melody2Score 文件夹拷贝到目标 Windows 电脑
   （无需安装 Python 或任何第三方库）。
2. 双击「启动Melody2Score.bat」，或直接双击 Melody2Score.exe。

【功能】
- 选择音频文件（wav / mp3 / flac / ogg / m4a）实时转简谱 + 五线谱 + 音高轮廓；
- 内置 144 个经典旋律样例，一键识别；
- 麦克风实时录音识别（需目标电脑有麦克风及声卡驱动）；
- 一键保存 Markdown 报告到 _internal\\app\\exports\\。

【说明】
- 首次启动稍慢（torch 在磁盘上解压载入），属正常现象；
- 不要拆分移动文件夹，_internal\\ 需保持完整；
- 本程序不含任何网络上传行为，可离线使用。

【关于"标准歌谱图片"】
- 简谱图片由第三方引擎 LilyPond（+ jianpu-ly 预处理器）渲染，
  质量优于手绘。jianpu-ly 脚本已随包内置（_internal\\lib\\jianpu-ly.py）。
- 若导出 PNG/PDF/SVG 时提示"缺少 LilyPond"，请在本机安装 LilyPond：
    * Windows：  winget install LilyPond.LilyPond
    * macOS：    brew install lilypond
    * Linux：    sudo apt install lilypond
  安装后请确保 lilypond 在系统 PATH 中，再重新导出即可。
- 若目标电脑暂未安装 LilyPond，程序会自动回退到内置的简谱图片生成，
  功能不受影响（仅排版样式不同）。
"""


def write_launcher_and_readme(dist: Path):
    bat = dist / "启动Melody2Score.bat"
    bat.write_text(LAUNCHER_BAT, encoding="utf-8")
    (dist / "README.txt").write_text(README_TXT, encoding="utf-8")
    log.info(f"已生成启动器: {bat.name} 与 README.txt")


# --------------------------------------------------------------------------- #
# 主流程
# --------------------------------------------------------------------------- #
def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Melody2Score 企业级一键打包")
    parser.add_argument("--no-deps", action="store_true", help="跳过依赖安装")
    parser.add_argument("--clean", action="store_true", help="打包前清空 build/ 与 dist/")
    parser.add_argument("--spec", default=str(SPEC_DEFAULT), help="自定义 .spec 文件")
    args = parser.parse_args(argv)

    spec = Path(args.spec).resolve()
    if not spec.exists():
        log.error(f"未找到 spec 文件: {spec}")
        return 1

    log.info("=" * 60)
    log.info(" Melody2Score 企业级一键打包开始")
    log.info("=" * 60)
    log.info(f"Python: {sys.version.splitlines()[0]}")
    log.info(f"平台:   {sys.platform}")
    log.info(f"工作目录: {HERE}")

    # 1) 环境自检
    if sys.platform != "win32":
        log.warn("当前非 Windows 平台，将生成对应平台可执行文件（无法在 Windows 以外生成 .exe）。")
    if check_another_build_running():
        log.error("检测到已有 pyinstaller 进程在运行，请等待其结束后再打包，避免产物冲突。")
        return 1

    ensure_runtime_dirs()

    # 2) 依赖（构建依赖 + 运行依赖）
    if not args.no_deps:
        log.info("[1/4] 安装构建/运行依赖 ...")
        if pip_install(["-U", "pyinstaller"]) != 0:
            log.error("安装 pyinstaller 失败。")
            return 1
        if pip_install(["-r", str(HERE / "requirements.txt"), "pyqt5"]) != 0:
            log.warn("部分运行依赖安装失败，尝试继续（若已安装则无碍）。")
    else:
        log.info("[1/4] 跳过依赖安装（--no-deps）。")

    # 3) 清理
    if args.clean:
        log.info("[2/4] 清空旧构建 ...")
        for d in (HERE / "build", HERE / "dist"):
            if d.exists():
                shutil.rmtree(d, ignore_errors=True)
    else:
        log.info("[2/4] 保留旧构建缓存以加速（如需全新构建请加 --clean）。")

    # 4) PyInstaller
    log.info("[3/4] 运行 PyInstaller（torch 较大，请耐心等待 5-15 分钟）...")
    cmd = [sys.executable, "-m", "PyInstaller", str(spec), "--noconfirm"]
    if args.clean:
        cmd.append("--clean")
    rc = run(cmd)
    if rc != 0:
        log.error(f"PyInstaller 打包失败（退出码 {rc}）。")
        return 2

    # 5) 产物校验与收尾
    log.info("[4/4] 校验产物并生成启动器 ...")
    dist = HERE / "dist" / "Melody2Score"
    # spec 将 exe 置于发行版根目录（Melody2Score.exe），启动器须与此一致
    exe = dist / "Melody2Score.exe"
    if not exe.exists():
        log.error(f"未找到主程序: {exe}")
        return 3

    # 确认关键数据已打入（PyInstaller 6 将数据放在 _internal/ 下）
    internal = dist / "_internal"
    missing = []
    for rel, _ in BUNDLE_DATA:
        src = HERE / rel
        if src.is_dir():
            dst = internal / rel
            if not dst.exists() or not any(dst.iterdir()):
                missing.append(rel)
    if missing:
        log.warn(f"以下数据目录未正确打入: {missing}（请检查 spec 的 datas 配置）")

    # 关键：校验简谱渲染脚本 lib/jianpu-ly.py 已随包打入（第三方渲染后端依赖）
    # is_file() 防目录假阳性（同 audio 校验）
    jianpu_src = HERE / "lib" / "jianpu-ly.py"
    jianpu_dst = internal / "lib" / "jianpu-ly.py"
    if jianpu_src.is_file() and not jianpu_dst.is_file():
        log.error("lib/jianpu-ly.py 未打入发行版，简谱图片将无法用第三方引擎渲染。")
        return 4
    else:
        log.info("简谱渲染脚本 jianpu-ly.py 校验通过。")

    # 关键：校验「内置经典样例」音频确实随包打入，不达标直接报错
    # 注意必须用 is_file() 过滤：曾因 glob 匹配到与 wav 同名的【目录】而
    # 产生假阳性（spec 旧版 Tree 解包错误把目标文件路径当目录用）。
    audio_src = HERE / "audio"
    audio_dst = internal / "audio"
    src_wavs = sorted(p for p in audio_src.glob("*.wav") if p.is_file()) if audio_src.exists() else []
    src_manifest = audio_src / "manifest.json"
    bundled_wavs = sorted(p for p in audio_dst.glob("*.wav") if p.is_file()) if audio_dst.exists() else []
    if not src_wavs:
        log.warn("源码 audio/ 下未找到任何 .wav 样例，将打包一个无样例的发行版。")
    elif len(bundled_wavs) < len(src_wavs):
        log.error(
            f"内置样例打包不完整：源码 {len(src_wavs)} 个 .wav 文件，"
            f"发行版仅 {len(bundled_wavs)} 个。请检查 spec 中 audio 的 Tree/ datas 配置。"
        )
        return 4
    elif src_manifest.is_file() and not (audio_dst / "manifest.json").is_file():
        log.error("内置样例 manifest.json 未打入发行版（或被打成目录），样例清单将加载失败。")
        return 4
    else:
        log.info(f"内置经典样例校验通过：{len(bundled_wavs)} 个 .wav 文件 + manifest.json 已随包打入。")

    write_launcher_and_readme(dist)

    # 计算体积
    total = sum(f.stat().st_size for f in dist.rglob("*") if f.is_file())
    log.info(f"打包完成！发行版体积: {total / 1024 / 1024 / 1024:.2f} GB")
    log.info(f"产物目录: {dist}")
    log.info("分发方式：把 dist/Melody2Score 整个文件夹压缩发给对方，")
    log.info("          解压后双击「启动Melody2Score.bat」即可运行。")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        log.error("用户中断。")
        sys.exit(130)
    finally:
        log.close()
