# -*- coding: utf-8 -*-
"""Melody2Score 企业级桌面 GUI（PyQt5）。

打开即用：选择音频 / 麦克风录音 / 内置样例 → 后台线程识别 → 直接显示
简谱 + 五线谱 + 量化音高轮廓 + 音符明细，并可一键保存为 Markdown 报告。

运行：python app/gui.py
"""
import io
import json
import math
import os
import re
import shutil
import sys
import time
import traceback
from typing import Dict, List, Optional, Tuple

import numpy as np
import soundfile as sf
from PyQt5.QtCore import QThread, pyqtSignal, Qt, QSize, QTimer, QObject
from PyQt5.QtGui import QPainter, QPen, QColor, QFont, QBrush, QPalette, QPixmap
from PyQt5.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout, QPushButton,
    QLabel, QFileDialog, QComboBox, QSlider, QCheckBox, QTabWidget, QTextEdit,
    QTableWidget, QTableWidgetItem, QHeaderView, QProgressBar, QMessageBox,
    QGroupBox, QLineEdit, QFrame, QSizePolicy, QDialog, QScrollArea)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)
from core.paths import resource_path, is_frozen
from core.config import Config
from core import score_sheet
from app.audio_play import (play_raw, play_score, is_playing,
                            stop as audio_stop)

ACCENT = "#4f9dff"
ACCENT2 = "#36d399"
BG = "#0f1420"
PANEL = "#161d2e"
PANEL2 = "#1d2740"
LINE = "#2a3654"
TEXT = "#e6ecf5"
MUTED = "#8a98b5"


def midi_name(m: int) -> str:
    names = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']
    return f"{names[m % 12]}{m // 12 - 1}"


def load_audio_bytes(data: bytes, sr: int):
    """从字节加载音频并重采样到 sr。

    优先用 librosa（支持 mp3/wav/flac/ogg/m4a 等，底层用 soundfile/audioread），
    失败再回退到 soundfile 直接读（部分环境下的 soundfile 不带 mp3 解码）。
    """
    import librosa
    try:
        y, _ = librosa.load(io.BytesIO(data), sr=sr, mono=True)
        return np.asarray(y, dtype=np.float32), sr
    except Exception:
        y, _ = sf.read(io.BytesIO(data), samplerate=sr, dtype="float32", always_2d=False)
        return np.asarray(y, dtype=np.float32), sr


class _PlayBridge(QObject):
    """播放完成信号桥：on_done 在生产者线程触发，经 Qt 信号跨线程
    自动排队投递到主线程更新 UI（生产者线程直接操作控件不安全）。"""
    finished = pyqtSignal()


_PLAY_BRIDGE = _PlayBridge()


class RecognizeWorker(QThread):
    finished = pyqtSignal(dict)
    error = pyqtSignal(str)

    def __init__(self, source: Dict):
        super().__init__()
        self.source = source  # {kind:'file'|'sample'|'record', data/bytes/name, cfg}

    def run(self):
        try:
            # 统一委托给企业级编排器（core.pipeline.Melody2Score）：
            # 内置 crepe_onnx 首选 + 优雅降级 + 超时保护 + 稳健重识别共识 + 完整计时。
            # （旧版此处的 _conf/_consensus 副本已删除——与 pipeline 内实现
            #  重复且缺少每 run 簇隔离等修复，留着只会误导维护。）
            from core.pipeline import Melody2Score
            result = Melody2Score(self.source["cfg"]).recognize(self.source)
            self.finished.emit(result)
        except Exception as e:
            traceback.print_exc()
            self.error.emit(str(e))


class DecodeWorker(QThread):
    """后台解码 + 重采样原曲（避免 librosa.load 在主线程阻塞导致界面卡死）。"""
    done = pyqtSignal(object, int, str)   # y, sr, status_msg
    failed = pyqtSignal(str)

    def __init__(self, data: bytes, sr: int, status_msg: str):
        super().__init__()
        self.data = data
        self.sr = sr
        self.status_msg = status_msg

    def run(self):
        try:
            y, sr = load_audio_bytes(self.data, self.sr)
            self.done.emit(y, sr, self.status_msg)
        except Exception as e:
            traceback.print_exc()
            self.failed.emit(str(e))



class SheetWorker(QThread):
    """后台生成标准歌谱（png/pdf/svg）（渲染重活移出主线程，避免界面卡顿）。"""
    done = pyqtSignal(str)      # 成功：文件路径
    failed = pyqtSignal(str)    # 失败：完整 traceback（企业级可观测性——
                                # windowed 发行版 stderr 是 NullWriter，
                                # print_exc 会静默丢失，必须随信号带出）

    def __init__(self, res: Dict, title: str, fmt: str = "png"):
        super().__init__()
        self.res = res
        self.title = title
        self.fmt = fmt
        self.outcome = None    # ("ok", path) / ("fail", traceback)——
                                # 直读属性，供诊断模式绕过信号投递时序

    def run(self):
        try:
            import re as _re, time as _time, os as _os
            from core.paths import exports_dir
            export_dir = exports_dir()
            safe = _re.sub(r"[^\w一-鿿-]", "_", self.title or "melody")[:40]
            ts = _time.strftime("%Y%m%d_%H%M%S")
            fpath = _os.path.join(export_dir, f"{safe or 'melody'}_标准歌谱_{ts}.{self.fmt}")
            score_sheet.export_score(
                notes=self.res.get("notes", []),
                key=self.res.get("key", {"tonic": "C", "mode": "major"}),
                bpm=float(self.res.get("bpm", 120)),
                output_path=fpath,
                title=self.title or "未命名旋律",
            )
            self.outcome = ("ok", fpath)
            self.done.emit(fpath)
        except Exception:
            tb = traceback.format_exc()[-2000:]
            self.outcome = ("fail", tb)
            self.failed.emit(tb)
        except BaseException:
            # SystemExit 等非 Exception 逃逸时也必须留痕（否则线程静默消失）
            tb = traceback.format_exc()[-2000:]
            self.outcome = ("fail", tb)
            self.failed.emit(tb)
            raise


# 五线谱：每半音一级；以「底线(第1线)」为基准 midi，向上逐级 +1。
# 高音谱号底线 = E4 (midi 64)；低音谱号底线 = G2 (midi 43)。
_TREBLE_SHARP_STEPS = [10, 5, 12, 7, 14, 9, 16]   # 高音谱号 # 位置(相对底线级数)
_TREBLE_FLAT_STEPS  = [7, 12, 5, 10, 3, 8, 1]
_BASS_SHARP_STEPS   = [3, 8, 1, 6, 11, 4, 9]
_BASS_FLAT_STEPS    = [10, 5, 12, 7, 14, 9, 16]

_MAJOR_SHARPS = {'C':0,'G':1,'D':2,'A':3,'E':4,'B':5,'F#':6,'C#':7,
                 'F':-1,'Bb':-2,'Eb':-3,'Ab':-4,'Db':-5,'Gb':-6,'Cb':-7}
_MINOR_SHARPS = {'A':0,'E':1,'B':2,'F#':3,'C#':4,'G#':5,'D#':6,'A#':7,
                 'D':-1,'G':-2,'C':-3,'F':-4,'Bb':-5,'Eb':-6,'Ab':-7}


def _clef_bottom(lo, hi):
    if hi <= 55:
        return 43          # 低音谱号
    return 64              # 高音谱号（默认）


class StaffView(QWidget):
    """标准五线谱：5 线 + 谱号 + 调号 + 拍号 + 按真实音高落在线/间 + 时值 + 小节线。

    性能：整张谱面离屏渲染到 QPixmap 缓存，只在数据变化或尺寸变化时重绘；
    常规 paintEvent 仅把缓存 blit 到屏幕，避免每次 resize/滚动都重算全部音符绘制。
    """
    def __init__(self):
        super().__init__()
        self.notes = []
        self.key = {"tonic": "C", "mode": "major"}
        self.bpm = 120.0
        self.beats_per_bar = 4
        self.setMinimumHeight(260)
        self._cache = QPixmap()
        self._cache_w = 0
        self._cache_h = 0

    def setData(self, notes, key, bpm=120.0, beats_per_bar=4):
        self.notes = notes or []
        self.key = key or {"tonic": "C", "mode": "major"}
        self.bpm = bpm
        self.beats_per_bar = beats_per_bar
        self._cache = QPixmap()   # 数据变化，强制下次重绘缓存
        self.update()

    def _key_sig(self):
        tonic = self.key.get("tonic", "C")
        mode = self.key.get("mode", "major")
        v = (_MINOR_SHARPS if mode == "minor" else _MAJOR_SHARPS).get(tonic, 0)
        return (v, 0) if v > 0 else (0, -v)

    def _render_to(self, pix: QPixmap):
        """把整张谱面绘制到离屏 pixmap（仅数据/尺寸变化时调用）。"""
        pix.fill(QColor(BG))
        p = QPainter(pix)
        p.setRenderHint(QPainter.Antialiasing)
        W, H = pix.width(), pix.height()

        if not self.notes:
            p.setPen(QColor(MUTED))
            p.setFont(QFont("Sans", 12))
            p.drawText(40, H // 2, "（识别后在此显示标准五线谱）")
            return

        padL, padR = 96, 24
        midi_all = [int(round(n["midi"])) for n in self.notes]
        lo, hi = min(midi_all), max(midi_all)
        clef_bot = _clef_bottom(lo, hi)
        gap = 11
        y0 = H // 2 + 2 * gap     # 底线 y

        # ---- 五条线 ----
        p.setPen(QPen(QColor(LINE), 1))
        for i in range(5):
            p.drawLine(padL, int(y0 - i * gap), W - padR, int(y0 - i * gap))

        # ---- 谱号 ----
        p.setPen(QColor("#cdd6e8"))
        p.setFont(QFont("Serif", 34, QFont.Bold))
        p.drawText(padL - 86, int(y0 + gap * 1.4), "𝄞" if clef_bot == 64 else "𝄢")

        # ---- 调号 ----
        sharps, flats = self._key_sig()
        if clef_bot == 64:
            spos, fpos = _TREBLE_SHARP_STEPS, _TREBLE_FLAT_STEPS
        else:
            spos, fpos = _BASS_SHARP_STEPS, _BASS_FLAT_STEPS
        p.setFont(QFont("Serif", 20, QFont.Bold))
        sx = padL - 44
        if sharps:
            for k in range(sharps):
                step = spos[k]
                ly = y0 - (step // 2) * gap
                p.drawText(int(sx + k * 11), int(ly + 7), "#")
        elif flats:
            for k in range(flats):
                step = fpos[k]
                ly = y0 - (step // 2) * gap
                p.drawText(int(sx + k * 11), int(ly + 7), "♭")

        # ---- 拍号 ----
        p.setFont(QFont("Sans", 18, QFont.Bold))
        p.drawText(padL - 8, int(y0 - 3 * gap - 2), str(self.beats_per_bar))
        p.drawText(padL - 8, int(y0 + 14), "4")

        step_xy = lambda step, x: (x, y0 - (step // 2) * gap)

        def draw_ledgers(step, nx):
            p.setPen(QPen(QColor(LINE), 1))
            if step > 8:                       # 第5线之上
                for k in range(1, (step - 8) // 2 + 1):
                    ly = y0 - (4 + k) * gap
                    p.drawLine(int(nx - 9), int(ly), int(nx + 9), int(ly))
            if step < 0:                       # 底线之下
                for k in range(1, (-step) // 2 + 1):
                    ly = y0 + k * gap
                    p.drawLine(int(nx - 9), int(ly), int(nx + 9), int(ly))

        beat_dur = 60.0 / max(self.bpm, 1.0)
        start_t = self.notes[0]["start"]
        end_t = self.notes[-1]["end"] or (start_t + 1)
        total = max(1e-3, end_t - start_t)
        xOf = lambda t: padL + 30 + (t - start_t) / total * (W - padR - padL - 40)

        note_color = QColor(ACCENT2)
        beat_acc = 0.0
        for n in self.notes:
            m = int(round(n["midi"]))
            step = m - clef_bot
            nx, ny = step_xy(step, int(xOf(n["start"])))
            draw_ledgers(step, nx)
            beats = n["dur"] / beat_dur if beat_dur > 0 else 1.0
            r = 5
            is_whole = beats >= 3.5
            is_half = 1.5 <= beats < 3.5
            p.setPen(QPen(note_color, 1.5))
            if is_whole or is_half:
                p.setBrush(Qt.NoBrush)
            else:
                p.setBrush(QBrush(note_color))
            p.drawEllipse(int(nx - r), int(ny - int(r * 0.7)), int(r * 2), int(r * 1.4))
            if not is_whole:
                stem_top = ny - 26
                p.drawLine(int(nx + r), int(ny - 1), int(nx + r), int(stem_top))
                if beats < 0.75:
                    p.drawLine(int(nx + r), int(stem_top), int(nx + r + 9), int(stem_top + 9))
            if (is_half or (0.75 <= beats < 1.5)) and abs(beats - round(beats) - 0.5) < 0.12:
                p.setBrush(QBrush(note_color))
                p.drawEllipse(int(nx + r + 4), int(ny - 2), 3, 3)
            beat_acc += beats
            if beat_acc >= self.beats_per_bar - 1e-6:
                p.setPen(QPen(QColor(LINE), 1))
                bx = int(xOf(n["end"]) if "end" in n else nx) + 8
                p.drawLine(bx, int(y0), bx, int(y0 - 4 * gap))
                beat_acc = 0.0
        # 终止线
        p.setPen(QPen(QColor(LINE), 2))
        p.drawLine(W - padR, int(y0), W - padR, int(y0 - 4 * gap))
        p.end()

    def paintEvent(self, ev):
        W, H = self.width(), self.height()
        # 缓存失效（无缓存 / 尺寸变化）→ 重建离屏谱面
        if self._cache.isNull() or self._cache_w != W or self._cache_h != H:
            self._cache = QPixmap(W, H)
            self._cache_w, self._cache_h = W, H
            self._render_to(self._cache)
        p = QPainter(self)
        p.drawPixmap(0, 0, self._cache)

class PitchView(QWidget):
    """量化音高轮廓（音符级阶梯）。"""
    def __init__(self):
        super().__init__()
        self.notes = []
        self.setMinimumHeight(220)

    def setData(self, notes):
        self.notes = notes
        self.update()

    def paintEvent(self, ev):
        p = QPainter(self)
        p.fillRect(self.rect(), QColor(BG))
        W, H = self.width(), self.height()
        padT, padB = 14, 14
        if not self.notes:
            return
        minM = min(n["midi"] for n in self.notes) - 2
        maxM = max(n["midi"] for n in self.notes) + 2
        span = max(1, maxM - minM)
        yOf = lambda m: padT + (1 - (m - minM) / span) * (H - padT - padB)
        total = self.notes[-1]["end"] or 1
        xOf = lambda t: (t / total) * (W - 10) + 5

        pen = QPen(QColor(LINE))
        pen.setWidth(1)
        p.setPen(pen)
        for g in range(5):
            y = padT + g * (H - padT - padB) / 4
            p.drawLine(0, int(y), W, int(y))

        pen = QPen(QColor(ACCENT))
        pen.setWidth(2)
        p.setPen(pen)
        p.setBrush(QColor(ACCENT))
        first = True
        for n in self.notes:
            x1, x2, y = xOf(n["start"]), xOf(n["end"]), yOf(n["midi"])
            if first:
                p.drawLine(int(x1), int(y), int(x1), int(y))
                first = False
            p.drawLine(int(x1), int(y), int(x1), int(y))
            p.drawLine(int(x1), int(y), int(x2), int(y))
        for n in self.notes:
            x, y = xOf((n["start"] + n["end"]) / 2), yOf(n["midi"])
            p.drawEllipse(int(x) - 2, int(y) - 2, 4, 4)


class LevelMeter(QWidget):
    """录音电平表（实时音量条 + 呼吸红点动画）。"""
    def __init__(self):
        super().__init__()
        self.setMinimumHeight(64)
        self.level = 0.0
        self.recording = False

    def set_level(self, v: float):
        self.level = max(0.0, min(1.0, v))
        self.update()

    def paintEvent(self, ev):
        p = QPainter(self)
        p.fillRect(self.rect(), QColor(PANEL2))
        W, H = self.width(), self.height()
        # 呼吸红点
        t = time.time()
        pulse = 0.5 + 0.5 * abs(math.sin(t * 4.0)) if self.recording else 0.0
        r = 9
        cx, cy = 22, H // 2
        glow = QColor(255, 70, 70, int(60 + 140 * pulse))
        p.setBrush(QBrush(glow))
        p.setPen(Qt.NoPen)
        p.drawEllipse(cx - r - int(8 * pulse), cy - r - int(8 * pulse),
                      2 * (r + int(8 * pulse)), 2 * (r + int(8 * pulse)))
        p.setBrush(QColor(255, 70, 70))
        p.drawEllipse(cx - r, cy - r, 2 * r, 2 * r)
        # 电平条
        bx, bw = 48, W - 48 - 12
        by, bh = 14, H - 28
        p.setPen(QPen(QColor(LINE)))
        p.setBrush(QColor(BG))
        p.drawRect(bx, by, bw, bh)
        fill = int(bw * self.level)
        grad = QColor(ACCENT2) if self.level > 0.05 else QColor(MUTED)
        if self.level > 0.85:
            grad = QColor("#ffb454")
        p.setBrush(grad)
        p.drawRect(bx, by, fill, bh)
        # 刻度
        p.setPen(QColor(MUTED))
        p.setFont(QFont("Sans", 10))
        p.drawText(bx, by - 2, f"输入电平 {self.level * 100:.0f}%")


class RecDialog(QDialog):
    """录音对话框：3-2-1 准备倒计时 + 实时剩余秒数 + 电平动画。

    录制结束后通过 accept() 释放录制结果（wav 字节），调用方从 .result 读取。
    """
    def __init__(self, secs: int, sr: int, parent=None):
        super().__init__(parent)
        self.secs = secs
        self.sr = sr
        self.result: Optional[Tuple[bytes, "np.ndarray", int]] = None  # (wav_bytes, y, sr)
        self.err: Optional[Exception] = None
        self.frames = []  # 各块 ndarray
        self._stream = None
        self._countdown = 3  # 准备倒计时
        self._elapsed = 0.0
        self.setWindowTitle("🎙️ 录音中…")
        self.setModal(True)
        self.setMinimumSize(480, 240)
        self.setStyleSheet(f"""
            QDialog{{background:{BG};}}
            QWidget{{background:{BG};color:{TEXT};font-family:'Segoe UI','PingFang SC','Microsoft YaHei';font-size:15px;}}
            QLabel{{color:{TEXT};}}
        """)
        v = QVBoxLayout(self)
        v.setSpacing(16)
        v.setContentsMargins(28, 28, 28, 28)
        self.lblTitle = QLabel("准备录音…")
        self.lblTitle.setStyleSheet(f"font-size:20px;font-weight:800;color:{ACCENT};")
        v.addWidget(self.lblTitle)
        self.lblCount = QLabel("")
        self.lblCount.setStyleSheet(f"font-size:54px;font-weight:900;color:{ACCENT2};alignment:AlignCenter;")
        self.lblCount.setAlignment(Qt.AlignCenter)
        v.addWidget(self.lblCount)
        self.lblRemain = QLabel("")
        self.lblRemain.setStyleSheet(f"color:{MUTED};font-size:14px;")
        self.lblRemain.setAlignment(Qt.AlignCenter)
        v.addWidget(self.lblRemain)
        self.meter = LevelMeter()
        v.addWidget(self.meter)
        self.lblTip = QLabel("请对着麦克风清唱 / 哼鸣，保持音量平稳。录音将自动开始与结束。")
        self.lblTip.setStyleSheet(f"color:{MUTED};font-size:12px;")
        self.lblTip.setAlignment(Qt.AlignCenter)
        v.addWidget(self.lblTip)
        self.btnCancel = QPushButton("取消")
        self.btnCancel.setStyleSheet("background:#33405f;color:#e6ecf5;padding:10px 16px;border-radius:9px;")
        self.btnCancel.clicked.connect(self.reject)
        v.addWidget(self.btnCancel)

        # 准备倒计时定时器（每 1s）
        self._timer = QTimer(self)
        self._timer.timeout.connect(self._tick)
        self._timer.start(1000)
        self._tick()  # 立即显示首帧

    def _tick(self):
        if self._countdown > 0:
            self.lblCount.setText(str(self._countdown))
            self.lblTitle.setText("准备录音…")
            self.lblRemain.setText(f"即将开始 {self.secs}s 录音")
            self.meter.recording = False
            self._countdown -= 1
            return
        # 倒计时结束，开始正式录音
        if self._stream is None:
            self._start_recording()
        self._elapsed += 1.0
        left = max(0, self.secs - int(self._elapsed))
        self.lblCount.setText(f"{left}")
        self.lblTitle.setText("🎙️ 录音中…")
        self.lblRemain.setText(f"剩余 {left}s / 共 {self.secs}s")
        if left <= 0:
            self._timer.stop()
            self._finish()

    def _start_recording(self):
        try:
            import sounddevice as sd
            import soundfile as sf
        except Exception as e:  # pragma: no cover
            self.err = e
            self._timer.stop()
            self.reject()
            return
        self._sf = sf
        self.meter.recording = True

        def cb(indata, frames, t, status):
            if status:
                pass
            chunk = np.asarray(indata, dtype=np.float32).reshape(-1).copy()
            self.frames.append(chunk)
            # 实时电平（RMS）
            rms = float(np.sqrt(np.mean(chunk.astype(np.float32) ** 2) + 1e-12))
            lvl = min(1.0, rms * 8.0)  # 放大便于观察
            # 用单次计时器回到主线程刷新（避免子线程直接 paint）
            QTimer.singleShot(0, lambda: self.meter.set_level(lvl))

        try:
            self._stream = sd.InputStream(samplerate=self.sr, channels=1,
                                          dtype="float32", blocksize=2048, callback=cb)
            self._stream.start()
        except Exception as e:  # pragma: no cover
            self.err = e
            self._timer.stop()
            self.reject()

    def _finish(self):
        try:
            if self._stream is not None:
                self._stream.stop()
                self._stream.close()
            if self.frames:
                y = np.concatenate(self.frames, axis=0).astype(np.float32)
            else:
                y = np.zeros(int(self.sr * 0.2), dtype=np.float32)
            buf = io.BytesIO()
            self._sf.write(buf, y, self.sr, format="WAV")
            self.result = (buf.getvalue(), y, self.sr)
        except Exception as e:
            self.err = e
            self.reject()
            return
        self.accept()

    def closeEvent(self, ev):
        if self._stream is not None:
            try:
                self._stream.stop()
                self._stream.close()
            except Exception:
                pass
        super().closeEvent(ev)


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Melody2Score · 哼唱旋律转谱（企业级桌面版）")
        self.setMinimumSize(1680, 1250)
        self.resize(1760, 1280)
        self.current = None
        self.pending_file = None
        self._pending_bytes = None
        self._pending_sample_path = None
        self._auto_sheet_path = None
        self._raw_y = None
        self._raw_sr = 22050
        self._apply_style()
        self._build()
        self._load_samples()

    def _apply_style(self):
        self.setStyleSheet(f"""
            QMainWindow{{background:{BG};}}
            QWidget{{background:{BG};color:{TEXT};font-family:'Segoe UI','PingFang SC','Microsoft YaHei';font-size:15px;}}
            QGroupBox{{border:1px solid {LINE};border-radius:12px;margin-top:14px;padding:14px;font-size:15px;}}
            QGroupBox::title{{color:{MUTED};subcontrol-position:top left;padding:0 8px;font-size:14px;}}
            QPushButton{{background:{ACCENT};color:#06101f;border:none;border-radius:10px;
                padding:12px 18px;font-weight:800;font-size:15px;}}
            QPushButton:hover{{background:#6fb0ff;}}
            QPushButton:disabled{{background:{PANEL2};color:{MUTED};}}
            QComboBox,QLineEdit{{background:{PANEL2};border:1px solid {LINE};border-radius:9px;
                padding:9px;color:{TEXT};font-size:15px;min-height:18px;}}
            QLabel{{color:{TEXT};font-size:15px;}}
            QTableWidget{{background:{PANEL2};gridline-color:{LINE};border:1px solid {LINE};font-size:14px;}}
            QHeaderView::section{{background:{PANEL};color:{MUTED};border:none;padding:8px;font-size:14px;}}
            QTextEdit{{background:#0b1020;border:1px solid {LINE};border-radius:12px;
                color:#dfe9ff;font-family:Consolas,monospace;font-size:16px;}}
            QTabBar::tab{{background:{PANEL2};color:{MUTED};padding:12px 22px;border-radius:9px;margin:3px;font-size:15px;font-weight:700;}}
            QTabBar::tab:selected{{background:{ACCENT};color:#06101f;}}
            QProgressBar{{background:{PANEL2};border:1px solid {LINE};border-radius:7px;text-align:center;font-size:13px;}}
            QProgressBar::chunk{{background:{ACCENT2};border-radius:7px;}}
            QFrame#sep{{background:{LINE};max-height:1px;}}
            QCheckBox{{font-size:15px;spacing:6px;}}
        """)

    def _build(self):
        root = QWidget()
        self.setCentralWidget(root)
        h = QHBoxLayout(root)
        h.setSpacing(22)
        h.setContentsMargins(24, 24, 24, 24)

        # 左侧控制
        left = QFrame()
        left.setFixedWidth(430)
        lv = QVBoxLayout(left)
        lv.setSpacing(16)

        # 输入
        g_in = QGroupBox("输入源")
        iv = QVBoxLayout(g_in)
        self.btnFile = QPushButton("📂 选择音频文件")
        self.btnFile.clicked.connect(self.pick_file)
        rec_row = QHBoxLayout()
        self.btnRec = QPushButton("🎙️ 录音并识别")
        self.btnRec.clicked.connect(self.record)
        self.cbRecSec = QComboBox()
        self.cbRecSec.addItems(["30 秒", "60 秒", "10 秒", "3 秒"])
        self.cbRecSec.setCurrentIndex(0)
        self.cbRecSec.setFixedWidth(80)
        rec_row.addWidget(self.btnRec)
        rec_row.addWidget(self.cbRecSec)
        self.lblFile = QLabel("未选择文件")
        self.lblFile.setStyleSheet(f"color:{MUTED};font-size:12px;")
        iv.addWidget(self.btnFile)
        iv.addLayout(rec_row)
        iv.addWidget(self.lblFile)
        # 试听原曲
        row_aud = QHBoxLayout()
        self.btnPreview = QPushButton("▶ 试听原曲")
        self.btnPreview.setEnabled(False)
        self.btnPreview.clicked.connect(self.preview_original)
        self.btnStop = QPushButton("⏹ 停止")
        self.btnStop.clicked.connect(lambda: audio_stop())
        row_aud.addWidget(self.btnPreview)
        row_aud.addWidget(self.btnStop)
        iv.addLayout(row_aud)
        lv.addWidget(g_in)

        # 参数
        g_p = QGroupBox("参数")
        pv = QVBoxLayout(g_p)
        pv.addWidget(QLabel("音高模型"))
        self.cbModel = QComboBox()
        self.cbModel.addItems(["tiny", "small", "full"])
        pv.addWidget(self.cbModel)
        self.cbVocal = QComboBox()
        self.cbVocal.addItems(["人声模式（唱歌/哼唱）", "器乐/通用模式"])
        pv.addWidget(QLabel("识别场景"))
        pv.addWidget(self.cbVocal)
        pv.addWidget(QLabel("降噪（谱减）"))
        self.cbDenoise = QComboBox()
        self.cbDenoise.addItems(["开启", "关闭（板端省内存）"])
        pv.addWidget(self.cbDenoise)
        # 稳健重识别：多次识别取共识，抑制单次偶发假音高/漏音
        self.cbRobust = QCheckBox("稳健重识别（多次取共识，更准但更慢）")
        self.cbRobust.setChecked(False)   # 默认关闭：识别只跑 1 遍，大幅提速；需更准时再勾选
        pv.addWidget(self.cbRobust)
        pv.addWidget(QLabel("帧移 hop(ms)"))
        self.slHop = QSlider(Qt.Horizontal)
        self.slHop.setRange(5, 30)
        self.slHop.setValue(10)
        self.slHop.setSingleStep(5)
        self.slHop.valueChanged.connect(lambda v: self.lblHop.setText(f"{v} ms"))
        self.lblHop = QLabel("10 ms")
        self.lblHop.setStyleSheet(f"color:{MUTED};")
        pv.addWidget(self.slHop)
        pv.addWidget(self.lblHop)
        lv.addWidget(g_p)

        # 样例
        g_s = QGroupBox("内置经典样例")
        sv = QVBoxLayout(g_s)
        self.sampleCombo = QComboBox()
        sv.addWidget(self.sampleCombo)
        row_s = QHBoxLayout()
        self.btnSample = QPushButton("识别选中样例")
        self.btnSample.clicked.connect(self.run_sample)
        self.btnPlaySample = QPushButton("▶ 播放原曲")
        self.btnPlaySample.clicked.connect(self.play_sample)
        row_s.addWidget(self.btnSample)
        row_s.addWidget(self.btnPlaySample)
        sv.addLayout(row_s)
        self.btnAddSample = QPushButton("➕ 添加样例（导入本地音频）")
        self.btnAddSample.clicked.connect(self.add_sample)
        sv.addWidget(self.btnAddSample)
        lv.addWidget(g_s)

        lv.addStretch(1)
        self.btnSave = QPushButton("💾 保存 Markdown")
        self.btnSave.setEnabled(False)
        self.btnSave.clicked.connect(self.save_md)
        lv.addWidget(self.btnSave)
        # 导出标准歌谱图片
        self.btnExportPng = QPushButton("🖼️ 标准歌谱 PNG")
        self.btnExportPng.setEnabled(False)
        self.btnExportPng.clicked.connect(lambda: self.export_sheet("png"))
        lv.addWidget(self.btnExportPng)
        row_sheet = QHBoxLayout()
        self.btnExportPdf = QPushButton("📄 PDF")
        self.btnExportPdf.setEnabled(False)
        self.btnExportPdf.clicked.connect(lambda: self.export_sheet("pdf"))
        self.btnExportSvg = QPushButton("✏️ SVG")
        self.btnExportSvg.setEnabled(False)
        self.btnExportSvg.clicked.connect(lambda: self.export_sheet("svg"))
        row_sheet.addWidget(self.btnExportPdf)
        row_sheet.addWidget(self.btnExportSvg)
        lv.addLayout(row_sheet)
        # 播放识别后的钢琴曲
        row_play = QHBoxLayout()
        self.btnPlayScore = QPushButton("🎹 播放钢琴曲")
        self.btnPlayScore.setEnabled(False)
        self.btnPlayScore.clicked.connect(self.play_score_audio)
        _PLAY_BRIDGE.finished.connect(self._on_score_play_done)
        row_play.addWidget(self.btnPlayScore)
        row_play.addWidget(self.btnStop)
        lv.addLayout(row_play)
        h.addWidget(left)

        # 右侧
        right = QFrame()
        rv = QVBoxLayout(right)
        rv.setSpacing(10)

        # 指标条
        metric = QHBoxLayout()
        self.mKey = QLabel("—")
        self.mBpm = QLabel("—")
        self.mNotes = QLabel("—")
        self.mConf = QLabel("—")
        self.mBackend = QLabel("")
        self.mBackend.setStyleSheet(f"color:{MUTED};font-size:12px;")
        for w, t in [(self.mKey, "调式"), (self.mBpm, "BPM"), (self.mNotes, "音符数"), (self.mConf, "置信度")]:
            box = QGroupBox(t)
            bv = QVBoxLayout(box)
            w.setStyleSheet(f"font-size:20px;font-weight:800;color:{ACCENT};")
            bv.addWidget(w)
            metric.addWidget(box)
        metric.addWidget(self.mBackend, 1)
        rv.addLayout(metric)

        self.progress = QProgressBar()
        self.progress.setVisible(False)
        rv.addWidget(self.progress)

        self.status = QLabel("请选择音频 / 录音 / 样例开始转谱。")
        self.status.setStyleSheet(f"color:{MUTED};font-size:12px;")
        rv.addWidget(self.status)

        # 标签页
        tabs = QTabWidget()
        # 简谱
        self.jianpu = QTextEdit()
        self.jianpu.setReadOnly(True)
        tabs.addTab(self.jianpu, "简谱")
        # 标准歌谱（图片预览）
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        self.sheetLabel = QLabel("识别完成后自动生成标准歌谱。")
        self.sheetLabel.setAlignment(Qt.AlignCenter)
        self.sheetLabel.setStyleSheet(f"color:{MUTED};background:{PANEL2};")
        scroll.setWidget(self.sheetLabel)
        tabs.addTab(scroll, "标准歌谱")
        # 五线谱
        self.staff = StaffView()
        tabs.addTab(self.staff, "五线谱")
        # 音高
        self.pitch = PitchView()
        tabs.addTab(self.pitch, "音高轮廓")
        # 音符表
        self.table = QTableWidget(0, 5)
        self.table.setHorizontalHeaderLabels(["#", "MIDI", "音名", "起始(s)", "时长(s)"])
        self.table.horizontalHeader().setSectionResizeMode(QHeaderView.Stretch)
        tabs.addTab(self.table, "音符明细")
        # 识别歌曲
        self.songBox = QFrame()
        self.songBox.setMinimumHeight(360)
        svb = QVBoxLayout(self.songBox)
        svb.setSpacing(14)
        svb.setContentsMargins(18, 18, 18, 18)
        self.songHead = QLabel("🎯 旋律识别歌曲（离线曲库匹配）")
        self.songHead.setStyleSheet(f"color:{ACCENT};font-size:17px;font-weight:800;")
        svb.addWidget(self.songHead)
        self.songResult = QTextEdit()
        self.songResult.setReadOnly(True)
        self.songResult.setPlainText("识别完成后，这里会显示「你唱的可能是哪首歌」及匹配度。\n"
                                      "曲库为 15 首公版经典旋律，采用音程 DTW 匹配，对跑调/移调鲁棒。")
        svb.addWidget(self.songResult, 1)
        tabs.addTab(self.songBox, "识别歌曲")
        tabs.setCurrentIndex(1)  # 默认显示「标准歌谱」
        rv.addWidget(tabs, 1)

        self.titleEdit = QLineEdit("未命名旋律")
        self.titleEdit.setPlaceholderText("报告标题（用于 MD 文件名）")
        rv.addWidget(self.titleEdit)

        h.addWidget(right, 1)

    def _cfg(self) -> Config:
        cfg = Config()
        cfg.model_size = self.cbModel.currentText()
        cfg.enable_denoise = self.cbDenoise.currentIndex() == 0
        cfg.robust = self.cbRobust.isChecked()
        cfg.hop = self.slHop.value()
        if self.cbVocal.currentIndex() == 0:
            # 人声模式：收窄基频范围、启用 VAD、加强颤音平滑
            cfg.vocal_mode = True
            cfg.fmin, cfg.fmax = 80.0, 1000.0
            cfg.enable_vad = True
            cfg.median_win = max(cfg.median_win, 7)
            cfg.min_note_dur = max(cfg.min_note_dur, 0.12)
        else:
            cfg.vocal_mode = False
            cfg.fmin, cfg.fmax = 50.0, 1100.0
            cfg.enable_vad = False
        return cfg

    def _load_samples(self):
        """从 audio/manifest.json 载入样例清单并填充下拉框。

        经典样例按旋律去重（每首只显示一种音色），用户自加样例（category=user）
        全部显示。若清单缺失则给出提示，仍允许通过「添加样例」导入首个音频。
        """
        man = resource_path("audio", "manifest.json")
        self.manifest = []
        if not os.path.exists(man):
            # 打包（onedir）模式下若 _MEIPASS 解析失败，回退到 exe 旁的
            # _internal/audio/（PyInstaller 实际放置位置）
            if is_frozen():
                base = os.path.dirname(sys.executable)
                for cand in (
                    os.path.join(base, "_internal", "audio", "manifest.json"),
                    os.path.join(base, "audio", "manifest.json"),
                ):
                    if os.path.exists(cand):
                        man = cand
                        break
        if not os.path.exists(man):
            self.sampleCombo.clear()
            self.sampleCombo.addItem("（暂无样例，可点击下方「添加样例」导入）")
            self.sampleCombo.setEnabled(False)
            self.btnSample.setEnabled(False)
            return
        try:
            with open(man, encoding="utf-8") as f:
                self.manifest = json.load(f)
        except Exception as e:
            self.sampleCombo.clear()
            self.sampleCombo.addItem(f"（样例清单解析失败：{e}）")
            self.sampleCombo.setEnabled(False)
            self.btnSample.setEnabled(False)
            return
        self._reload_sample_combo()

    def _reload_sample_combo(self):
        """按 self.manifest 重新填充样例下拉框。

        经典样例按旋律去重（每首只显示一种音色，减少列表长度）；
        用户自加样例（category=user）每个都显示。条目 data 存相对文件路径，
        后续识别/播放均依据该路径反查清单项，避免「下拉索引≠清单索引」错位。
        """
        self.sampleCombo.clear()
        seen = {}
        for it in self.manifest:
            key = ("user", it["id"]) if it.get("category") == "user" \
                else ("melody", it.get("melody_index"))
            seen.setdefault(key, it)
        has_sample = False
        for it in seen.values():
            label = f"{it.get('title_zh', '')} · {it.get('title_en', '')} · {it.get('timbre', '')}"
            label = label.strip(" · ")
            self.sampleCombo.addItem(label or it["file"], it["file"])
            has_sample = True
        if not has_sample:
            self.sampleCombo.addItem("（暂无样例，可点击下方「添加样例」导入）")
        self.sampleCombo.setEnabled(has_sample)
        self.btnSample.setEnabled(has_sample)

    def _sample_by_file(self, file_rel: str) -> Optional[Dict]:
        """依据相对文件路径反查清单项（若无清单则返回 None）。"""
        if not hasattr(self, "manifest") or not self.manifest:
            return None
        for it in self.manifest:
            if it.get("file") == file_rel:
                return it
        return None

    def pick_file(self):
        path, _ = QFileDialog.getOpenFileName(
            self, "选择音频", "", "音频 (*.wav *.mp3 *.flac *.ogg *.m4a)")
        if not path:
            return
        self.pending_file = path
        self.lblFile.setText(os.path.basename(path))
        self.btnPreview.setEnabled(True)
        # 只读取一次文件字节，识别与试听共用，避免 UI 线程重复 I/O 解码造成卡顿
        try:
            with open(path, "rb") as f:
                self._pending_bytes = f.read()
        except Exception as e:
            self._pending_bytes = None
            self.status.setText(f"文件读取失败：{e}")
            return
        # 试听波形延迟到真正点击「试听原曲」时再解码，避免选文件即阻塞主线程
        self._raw_y = None
        self._raw_sr = None
        self.status.setText(f"已选择：{os.path.basename(path)}，开始识别…")
        self.run_file()

    def record(self):
        """打开录音对话框（含 3-2-1 准备倒计时 + 实时剩余秒数 + 电平动画），
        录音结束后自动识别人声。"""
        worker = getattr(self, "worker", None)
        if worker is not None and worker.isRunning():
            return  # 识别中，忽略重复点击
        try:
            import sounddevice  # noqa: F401
        except Exception as e:
            QMessageBox.critical(self, "录音不可用", f"未安装 sounddevice：{e}")
            return
        secs = [30, 60, 10, 3][self.cbRecSec.currentIndex()]
        sr = self._cfg().sr
        self.btnRec.setEnabled(False)
        self.btnRec.setText("🎙️ 录音中…")
        self.status.setText(f"🎙️ 打开录音… 请稍候（{secs}s）")
        dlg = RecDialog(secs, sr, self)
        dlg.finished.connect(lambda code, d=dlg, secs=secs: self._on_record_done(code, d, secs))
        dlg.open()  # 非模态，主线程不卡

    def _on_record_done(self, code, dlg: "RecDialog", secs: int):
        self.btnRec.setEnabled(True)
        self.btnRec.setText("🎙️ 录音并识别")
        if code != QDialog.Accepted or dlg.result is None:
            err = getattr(dlg, "err", None)
            self.status.setText(f"已取消录音。" + (f"（错误：{err}）" if err else ""))
            return
        wav_bytes, y_rec, sr = dlg.result
        self.pending_file = None
        self._raw_y, self._raw_sr = y_rec, sr
        self.btnPreview.setEnabled(True)
        self.status.setText(f"录音完成（{secs}s），开始识别人声…")
        import io as _io
        self._start({"kind": "record", "data": wav_bytes, "cfg": self._cfg(),
                     "source": f"麦克风录音 {secs}s"})

    def _start(self, source: Dict):
        # 防重入：识别线程仍在跑时忽略重复触发（双击按钮 /
        # 选样例信号重复 / 试听与识别并发等路径都汇聚到此），
        # 避免同一首歌被识别两次。
        worker = getattr(self, "worker", None)
        if worker is not None and worker.isRunning():
            return
        self.btnSample.setEnabled(False)
        self.progress.setVisible(True)
        self.progress.setRange(0, 0)
        self.status.setText("识别中…")
        self.worker = RecognizeWorker(source)
        self.worker.finished.connect(self.on_done)
        self.worker.error.connect(self.on_err)
        self.worker.start()

    def run_file(self):
        if not self.pending_file:
            return
        # 优先复用 pick_file 已读取的字节，避免重复磁盘 I/O
        if getattr(self, "_pending_bytes", None):
            data = self._pending_bytes
        else:
            try:
                with open(self.pending_file, "rb") as f:
                    data = f.read()
            except Exception as e:
                self.status.setText(f"文件读取失败：{e}")
                return
        self._start({"kind": "file", "data": data, "cfg": self._cfg(),
                     "source": os.path.basename(self.pending_file)})

    def run_sample(self):
        if not self.sampleCombo.isEnabled():
            return
        file_rel = self.sampleCombo.currentData()
        # 延迟解码：仅缓存样例路径，切换下拉不再同步解码（避免卡顿）；
        # 真正试听时由 preview_original 解码一次并复用
        item = self._sample_by_file(file_rel)
        self._pending_sample_path = None
        if item:
            path = self._sample_abs_path(item)
            if os.path.exists(path):
                self._pending_sample_path = path
                self.btnPreview.setEnabled(True)
        self._start({"kind": "sample", "name": file_rel, "cfg": self._cfg(),
                     "source": file_rel})

    def _sample_abs_path(self, item: Dict) -> str:
        """把清单项的相对文件路径解析为绝对路径（兼容打包后的 _MEIPASS 布局）。"""
        return resource_path(item["file"])

    def play_sample(self):
        """播放当前选中样例的原曲音频（读取 audio/<id>.wav 并回放）。

        解码 + 重采样（librosa.load）CPU 密集，放到后台 DecodeWorker，
        避免在主线程阻塞导致界面卡死（点播放后长时间无响应）。
        """
        if not self.sampleCombo.isEnabled():
            return
        file_rel = self.sampleCombo.currentData()
        item = self._sample_by_file(file_rel)
        if not item:
            return
        path = self._sample_abs_path(item)
        if not os.path.exists(path):
            QMessageBox.information(self, "播放原曲",
                                    f"未找到样例音频：{item['file']}\n请先运行 gen_classic_melodies.py 生成。")
            return
        try:
            with open(path, "rb") as f:
                data = f.read()
        except Exception as e:
            QMessageBox.critical(self, "播放失败", f"读取样例音频出错：{e}")
            return
        status_msg = f"▶ 试听原曲：{item['title_zh']} · {item['timbre']}"
        # 解码中禁用按钮防连点，并提示进度
        self.btnPlaySample.setEnabled(False)
        self.btnPlaySample.setText("解码中…")
        self.status.setText(f"⏳ 正在解码原曲：{item['title_zh']}")
        w = DecodeWorker(data, 22050, status_msg)

        def _ok(y, sr, msg):
            self.btnPlaySample.setEnabled(True)
            self.btnPlaySample.setText("▶ 播放原曲")
            self._raw_y, self._raw_sr = y, sr
            self.btnPreview.setEnabled(True)
            self.status.setText(msg)
            from app.audio_play import play_raw
            play_raw(y, sr)

        def _err(e):
            self.btnPlaySample.setEnabled(True)
            self.btnPlaySample.setText("▶ 播放原曲")
            QMessageBox.critical(self, "播放失败", f"解码样例音频出错：{e}")

        w.done.connect(_ok)
        w.failed.connect(_err)
        w.start()
        self._decode_worker = w  # 保引用防 GC

    def add_sample(self):
        """添加样例：导入本地音频文件为内置样例。

        把选中的音频拷贝到 audio/ 目录，并追加一条记录到 manifest.json，
        随后刷新下拉框，新样例即可像经典样例一样被「识别选中样例」和
        「播放原曲」。重复文件名会追加时间戳避免覆盖。
        """
        path, _ = QFileDialog.getOpenFileName(
            self, "添加样例（导入本地音频）", "",
            "音频 (*.wav *.mp3 *.flac *.ogg *.m4a)")
        if not path:
            return
        audio_dir = resource_path("audio")
        try:
            os.makedirs(audio_dir, exist_ok=True)
        except Exception as e:
            QMessageBox.critical(self, "添加样例", f"无法创建 audio 目录：{e}")
            return
        base = os.path.splitext(os.path.basename(path))[0]
        ext = os.path.splitext(path)[1] or ".wav"
        ts = time.strftime("%Y%m%d_%H%M%S")
        new_file = f"audio/user_sample_{ts}{ext}"
        dst = resource_path(new_file)
        try:
            shutil.copy2(path, dst)
        except Exception as e:
            QMessageBox.critical(self, "添加样例", f"拷贝音频失败：{e}")
            return
        if not hasattr(self, "manifest") or not isinstance(self.manifest, list):
            self.manifest = []
        user_count = len([x for x in self.manifest if x.get("category") == "user"])
        entry = {
            "id": f"user_{ts}",
            "melody_index": 9000 + user_count,
            "title_zh": base,
            "title_en": "",
            "category": "user",
            "timbre": "自定义",
            "sr": 22050,
            "file": new_file,
            "expected_midi": [],
        }
        self.manifest.append(entry)
        try:
            with open(resource_path("audio", "manifest.json"), "w", encoding="utf-8") as f:
                json.dump(self.manifest, f, ensure_ascii=False, indent=2)
        except Exception as e:
            QMessageBox.warning(self, "添加样例",
                                f"音频已导入，但清单保存失败：{e}\n{dst}")
            return
        self._reload_sample_combo()
        # 选中刚添加的样例，便于立即识别/播放
        new_idx = self.sampleCombo.findData(new_file)
        if new_idx >= 0:
            self.sampleCombo.setCurrentIndex(new_idx)
        self.status.setText(f"已添加样例：{base}")
        self.sampleCombo.setFocus()

    def on_done(self, res: Dict):
        self.current = res
        self.progress.setVisible(False)
        self.btnSample.setEnabled(self.sampleCombo.isEnabled())
        self.btnSave.setEnabled(True)
        self.btnPlayScore.setEnabled(True)
        self.btnExportPng.setEnabled(True)
        self.btnExportPdf.setEnabled(True)
        self.btnExportSvg.setEnabled(True)

        self.mKey.setText(f"{res['key']['tonic']} {'小调' if res['key']['mode']=='minor' else '大调'}")
        self.mBpm.setText(str(res["bpm"]))
        self.mNotes.setText(str(res["note_count"]))
        self.mConf.setText(f"{res.get('confidence', 0):.0%}")
        self.mBackend.setText(f"后端 {res['backend']} · 预处理 {res['perf']['preprocess_ms']}ms · "
                              f"音高 {res['perf']['pitch_ms']}ms · 解析 {res['perf']['parse_ms']}ms")
        self.jianpu.setPlainText(res["jianpu"] or "（无声）")
        self.staff.setData(res["notes"], res["key"], bpm=res.get("bpm", 120.0))
        self.pitch.setData(res["notes"])
        self.table.setRowCount(len(res["notes"]))
        for i, n in enumerate(res["notes"]):
            self.table.setItem(i, 0, QTableWidgetItem(str(i + 1)))
            self.table.setItem(i, 1, QTableWidgetItem(str(n["midi"])))
            self.table.setItem(i, 2, QTableWidgetItem(n["name"]))
            self.table.setItem(i, 3, QTableWidgetItem(str(n["start"])))
            self.table.setItem(i, 4, QTableWidgetItem(str(n["dur"])))
        robust_info = ""
        if res.get("robust_runs", 1) > 1:
            robust_info = f" · 重识别{res['robust_runs']}次→共识保留 {res['robust_kept']} 音 · 置信度 {res.get('confidence',0):.0%}"

        # 自动生成标准歌谱（后台线程，避免 LilyPond 渲染阻塞主线程造成卡顿）
        self.sheetLabel.setText("正在生成标准歌谱…")
        # 断开并停止旧的 sheet worker，避免其迟到信号串到新结果上
        old = getattr(self, "_sheet_worker", None)
        if old is not None:
            try:
                old.done.disconnect(self._on_sheet_done)
                old.failed.disconnect(self._on_sheet_failed)
            except TypeError:
                pass
            if old.isRunning():
                old.quit()
                old.wait(1000)
        self._sheet_worker = SheetWorker(res, self.titleEdit.text())
        self._sheet_worker.done.connect(self._on_sheet_done)
        self._sheet_worker.failed.connect(self._on_sheet_failed)
        self._sheet_worker.start()

        self.status.setText(f"完成 · {res['note_count']} 个音符 · 来源 {res.get('source','')}{robust_info}")

        # 旋律识别歌曲（离线曲库 DTW 音程匹配）
        try:
            from core.song_match import match_song
            mr = match_song(res.get("notes", []), top_k=5)
            lines = []
            if mr["matched"]:
                lines.append(f"✅ 匹配成功！最佳候选：{mr['candidates'][0]['title_zh']} "
                             f"({mr['candidates'][0]['title_en']})  匹配度 {mr['best_score']:.0f}%\n")
            else:
                lines.append(f"🔍 未在曲库中命中（最佳 {mr.get('best_score',0):.0f}%，阈值 55%）。"
                             f"可能是曲库外的歌，或片段过短。\n")
            lines.append(f"查询音符数：{mr['query_len']}　┄┄　Top 候选：\n")
            for i, c in enumerate(mr["candidates"], 1):
                bar = "█" * int(round(c["score"] / 5))
                lines.append(f"  {i}. {c['title_zh']:<6} {c['title_en']:<26} "
                             f"{c['score']:.0f}% {bar}")
            self.songResult.setPlainText("\n".join(lines))
        except Exception as e:
            self.songResult.setPlainText(f"（识别歌曲模块出错：{e}）")

    def on_err(self, msg: str):
        self.progress.setVisible(False)
        self.btnSample.setEnabled(self.sampleCombo.isEnabled())
        self.status.setText(f"错误：{msg}")
        QMessageBox.critical(self, "识别失败", msg)

    def preview_original(self):
        """试听已选择的原曲（mp3/wav 等）。

        延迟解码：若选文件时未解码（为不卡 UI 线程），这里首次试听时
        用缓存字节即时解码一次并复用，避免二次磁盘读取。
        """
        raw = getattr(self, "_raw_y", None)
        if raw is None:
            pending = getattr(self, "_pending_bytes", None)
            if pending:
                try:
                    self._raw_y, self._raw_sr = load_audio_bytes(pending, 22050)
                    raw = self._raw_y
                except Exception as e:
                    QMessageBox.critical(self, "试听失败", f"解码原曲出错：{e}")
                    return
            elif getattr(self, "_pending_sample_path", None):
                try:
                    with open(self._pending_sample_path, "rb") as f:
                        self._raw_y, self._raw_sr = load_audio_bytes(f.read(), 22050)
                    raw = self._raw_y
                except Exception as e:
                    QMessageBox.critical(self, "试听失败", f"解码原曲出错：{e}")
                    return
            if raw is None:
                QMessageBox.information(self, "试听", "尚未载入可播放的音频。")
                return
        try:
            self.status.setText("正在播放原曲…")
            self._score_playing = False          # 单播放器：原曲接管钢琴曲
            self.btnPlayScore.setText("🎹 播放钢琴曲")
            play_raw(raw, self._raw_sr)
        except Exception as e:
            self.status.setText(f"播放失败：{e}")

    def play_score_audio(self):
        """按识别出的音符序列合成钢琴曲播放（播放中再点 = 停止）。"""
        if not self.current or not self.current.get("notes"):
            QMessageBox.information(self, "播放", "尚无可播放的识别结果。")
            return
        if getattr(self, "_score_playing", False) and is_playing():
            audio_stop()                          # teardown 会触发 on_done → 恢复按钮
            return
        try:
            self._score_playing = True
            self.btnPlayScore.setText("⏹ 停止播放")
            self.status.setText("🎹 合成并播放钢琴曲中…（流式合成，即点即响）")
            play_score(self.current["notes"], sr=22050,
                       on_done=_PLAY_BRIDGE.finished.emit)
        except Exception as e:
            self._score_playing = False
            self.btnPlayScore.setText("🎹 播放钢琴曲")
            self.status.setText(f"钢琴播放失败：{e}")

    def _on_score_play_done(self):
        """播放结束/停止后恢复按钮（主线程执行，由 _PlayBridge 投递）。"""
        self._score_playing = False
        self.btnPlayScore.setText("🎹 播放钢琴曲")

    def _on_sheet_done(self, fpath: str):
        self._auto_sheet_path = fpath
        if fpath and os.path.exists(fpath):
            self.sheetLabel.setText("")
            self.sheetLabel.setPixmap(QPixmap(fpath).scaledToWidth(
                self.sheetLabel.width() - 20, Qt.SmoothTransformation))
        else:
            self._on_sheet_failed("输出文件不存在：" + str(fpath))

    @staticmethod
    def _sheet_fail_summary(tb: str) -> str:
        """从 traceback 提取人读失败摘要（最后一行异常 + 关键 RuntimeError 文案）。"""
        last = ""
        for line in tb.strip().splitlines():
            line = line.strip()
            if line and not line.startswith("File "):
                last = line
        return last[:300] or tb[-300:]

    def _on_sheet_failed(self, tb: str):
        """歌谱生成失败：展示真实原因（企业级可观测性，替代旧版静态误导文案）。"""
        summary = self._sheet_fail_summary(tb)
        self._auto_sheet_path = None
        self.sheetLabel.setText(f"标准歌谱生成失败：\n{summary}")
        self.status.setText("标准歌谱生成失败（详见标签区域）")

    def export_sheet(self, fmt: str):
        """导出标准歌谱。走后台 SheetWorker，避免渲染生成 PDF/SVG 时阻塞主线程。"""
        if not self.current:
            return
        self.status.setText(f"⏳ 正在后台生成 {fmt.upper()}…")
        # 停掉仍在跑的旧 sheet worker，避免后台堆积 / 迟到弹窗
        old = getattr(self, "_sheet_worker", None)
        if old is not None and old.isRunning():
            old.quit()
            old.wait(1000)
        locator = self  # 闭包里引用最新 worker

        def _ok(path: str):
            if getattr(locator, "_sheet_worker", None) is not w:
                return  # 已被新导出取代，丢弃迟到结果
            if fmt == "png":
                self.sheetLabel.setPixmap(QPixmap(path).scaledToWidth(
                    self.sheetLabel.width() - 20, Qt.SmoothTransformation))
            QMessageBox.information(self, "已导出", f"标准歌谱已导出：\n{path}")
            self.status.setText(f"已导出 {fmt.upper()}：{os.path.basename(path)}")

        def _fail(tb: str):
            if getattr(locator, "_sheet_worker", None) is not w:
                return
            self.status.setText("导出失败")
            QMessageBox.warning(self, "导出失败",
                                f"标准歌谱导出失败：\n{self._sheet_fail_summary(tb)}")

        w = SheetWorker(self.current, self.titleEdit.text() or "未命名旋律", fmt)
        self._sheet_worker = w  # 保引用防 GC

        def _ok(path: str):
            if fmt == "png":
                self.sheetLabel.setPixmap(QPixmap(path).scaledToWidth(
                    self.sheetLabel.width() - 20, Qt.SmoothTransformation))
            QMessageBox.information(self, "已导出", f"标准歌谱已导出：\n{path}")
            self.status.setText(f"已导出 {fmt.upper()}：{os.path.basename(path)}")

        def _fail(tb: str):
            self.status.setText("导出失败")
            QMessageBox.warning(self, "导出失败",
                                f"标准歌谱导出失败：\n{self._sheet_fail_summary(tb)}")

        w.done.connect(_ok)
        w.failed.connect(_fail)
        w.start()

    def save_md(self):
        if not self.current:
            return
        from core.paths import exports_dir
        title = self.titleEdit.text() or "未命名旋律"
        ts = time.strftime("%Y%m%d_%H%M%S")
        safe = re.sub(r"[^\w一-鿿-]", "_", title)[:40]
        export_dir = exports_dir()
        fname = f"{safe or 'melody'}_{ts}.md"
        fpath = os.path.join(export_dir, fname)
        lines = []
        lines.append(f"# 旋律转谱报告：{title}\n")
        lines.append(f"> 生成时间：{time.strftime('%Y-%m-%d %H:%M:%S')}  | 来源：{self.current.get('source','')}  | 音高后端：{self.current.get('backend','')}\n")
        lines.append("\n## 一、识别概要\n")
        lines.append("| 指标 | 值 |")
        lines.append("|------|----|")
        res = self.current
        lines.append(f"| 调式 | {res['key']['tonic']} {res['key']['mode']} |")
        lines.append(f"| 速度(BPM) | {res['bpm']} |")
        lines.append(f"| 音符数 | {res['note_count']} |")
        lines.append(f"| 时长(秒) | {res['duration_sec']} |")
        for k, v in [("预处理", res['perf']['preprocess_ms']), ("音高检测", res['perf']['pitch_ms']),
                     ("解析", res['perf']['parse_ms']), ("音高帧数", res['perf']['pitch_frames'])]:
            lines.append(f"| {k} | {v} |")
        lines.append("\n## 二、简谱\n")
        lines.append("```text")
        lines.append(res["jianpu"])
        lines.append("```\n")
        lines.append("## 三、音符明细（MIDI / 音名 / 起始 / 时长）\n")
        lines.append("| # | MIDI | 音名 | 起始(s) | 时长(s) |")
        lines.append("|---|------|------|---------|---------|")
        for i, n in enumerate(res["notes"], 1):
            lines.append(f"| {i} | {n['midi']} | {n['name']} | {n['start']} | {n['dur']} |")
        lines.append("\n## 四、处理算法说明\n")
        lines.append(
            "本结果由 melody2score 流水线生成，分五层处理：\n\n"
            "1. 采集层：librosa 重采样至 16kHz 单声道，峰值归一化。\n"
            "2. 预处理层：去直流偏移 + 峰值归一化 + 谱减降噪（以开头 0.1s 静音段估计噪声底）。\n"
            "3. 音高检测层：可插拔后端（crepe_onnx / torchcrepe / pyin），输出 [{t,freq,conf}]，"
            "低于置信度阈值判为无声。\n"
            "4. 音乐解析层：midi 轮廓中值滤波(win=5) 消除颤音抖动；半音量化后按相同音高分段；"
            "短段就近合并到音高最近的相邻音符；BPM 用 beat_track；调式用 Krumhansl-Schmuckler 模板"
            "（基于音符轮廓，起始/终止音加权）。\n"
            "5. 歌谱生成层：music21 量化生成 musicxml；简谱数字串（高八度 '.'，低八度 '_'，延音 '-'）；"
            "jianpu-ly + LilyPond 渲染标准歌谱图片（PNG/PDF/SVG）。\n\n"
            "优化：调式识别用音符轮廓替代整曲 CQT（提速 ~7x）；简谱以 tonic 的 4 八度音为基准计算八度偏移，"
            "标记符合记谱习惯。")
        with open(fpath, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))
        QMessageBox.information(self, "已保存", f"Markdown 报告已保存：\n{fpath}")
        self.status.setText(f"已保存：{fname}")


def _tol_match(got, exp):
    """容差匹配（与 tests/verify_real_audio.py 同算法）：DP 对齐音高一致率。"""
    if not exp or not got:
        return 0.0
    n, m = len(got), len(exp)
    dp = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(1, n + 1):
        for j in range(1, m + 1):
            eq = 1 if got[i - 1] == exp[j - 1] else 0
            dp[i][j] = max(dp[i - 1][j - 1] + eq, dp[i - 1][j], dp[i][j - 1])
    return dp[n][m] / len(exp)


# 全链路回归代表样例：乐器/人声/纯音三类 × 不同节奏复杂度
_REGRESS_PAIRS = [
    ("小星星", "piano"), ("小星星", "human_voice"),
    ("欢乐颂", "piano"), ("欢乐颂", "flute"),
    ("茉莉花", "guitar"), ("两只老虎", "strings"),
    ("致爱丽丝", "pure_sine"), ("生日歌", "human_voice"),
]


def _selftest() -> int:
    """打包产物自检模式（无头，无 Qt）：
    Melody2Score.exe --selftest [输出.json]        基础链路冒烟（识别→简谱）
    Melody2Score.exe --selftest-full [输出.json]   全链路交付验收

    企业级发行版验证标准实践：在目标电脑上无需 GUI 交互即可验证
    「frozen 环境下完整识别链路（加载→预处理→音高检测→解析→歌谱）」可用。
    --selftest：取 manifest 首个内置样例 → 完整识别 → JSON 报告 → 退出码
    0=通过 / 1=失败。可用于 CI 冒烟。
    --selftest-full：另加 MusicXML 导出 / 标准歌谱 PNG / 钢琴合成 / mp3 解码 /
    离线曲库匹配 / 8 样例真实音频回归（manifest 自带 expected_midi 真值，
    精确率+容差率+音高类覆盖三项量化指标），全部通过才 exit 0。交付验收用。
    """
    out_path = None
    full = "--selftest-full" in sys.argv
    args = [a for a in sys.argv[2:] if not a.startswith("--")]
    if args:
        out_path = args[0]
    try:
        import json as _json
        from core.pipeline import Melody2Score          # 重型依赖链：librosa/music21
        from core import capture

        man_path = resource_path("audio", "manifest.json")
        with open(man_path, encoding="utf-8") as f:
            manifest = _json.load(f)
        item = manifest[0]
        wav = resource_path("audio", os.path.basename(item["file"]))

        cfg = Config()
        t0 = time.time()
        y = capture.load_audio(wav, cfg.sr)
        res = Melody2Score(cfg).recognize({"kind": "array", "y": y,
                                           "sr": cfg.sr, "cfg": cfg})
        elapsed = round(time.time() - t0, 2)

        notes = res.get("notes", [])
        report = {
            "selftest": "Melody2Score 打包产物自检",
            "mode": "full" if full else "basic",
            "frozen": is_frozen(),
            "sample": item["file"],
            "title_zh": item.get("title_zh", ""),
            "note_count": len(notes),
            "bpm": res.get("bpm"),
            "backend": res.get("backend"),
            "jianpu_head": (res.get("jianpu") or "")[:60],
            "elapsed_sec": elapsed,
            "pass": bool(notes) and bool(res.get("jianpu")),
        }

        if full:
            # ---- 全链路功能模块逐项验收（临时目录产出，不污染发行版） ----
            import tempfile

            checks = {}

            def _run(name, fn):
                """fn 返回 detail；以 'SKIP' 开头视为可选外部依赖缺失（不算失败）。"""
                try:
                    detail = fn()
                    checks[name] = {"pass": True,
                                    "skip": str(detail).startswith("SKIP"),
                                    "detail": str(detail)}
                except Exception as e:
                    checks[name] = {"pass": False, "detail": str(e)[:200]}

            def _musicxml():
                from core import score
                tmp = os.path.join(tempfile.mkdtemp(prefix="st_mx_"), "out.musicxml")
                score.to_musicxml(notes, res.get("bpm", 120),
                                  (res["key"]["tonic"], res["key"]["mode"]), fp=tmp)
                mx = open(tmp, encoding="utf-8").read()
                assert "part-list" in mx and "<note" in mx, "musicxml 内容异常"
                return f"{len(mx)} chars"

            def _score_png():
                from core import score_sheet, jianpu_render
                if not jianpu_render.find_lilypond():
                    return "SKIP：未安装 LilyPond（可选外部工具，安装说明见 README）"
                tmp = os.path.join(tempfile.mkdtemp(prefix="st_ly_"), "score.png")
                score_sheet.export_score(
                    notes=notes, key=res.get("key", {"tonic": "C", "mode": "major"}),
                    bpm=float(res.get("bpm", 120)), output_path=tmp,
                    title="全链路自检")
                sz = os.path.getsize(tmp)
                assert sz > 5000, f"渲染产物过小: {sz}"
                return f"png {sz} bytes（jianpu-ly + LilyPond）"

            def _synth():
                import numpy as np
                from core.synth import synth_piano
                y3 = synth_piano(60, 0.2, 16000)
                assert isinstance(y3, np.ndarray) and len(y3) > 3000, "合成异常"
                return f"{len(y3)} samples"

            def _mp3():
                import numpy as np
                import soundfile as sf
                tmp = os.path.join(tempfile.mkdtemp(prefix="st_mp3_"), "t.mp3")
                t = np.arange(16000) / 16000.0
                sf.write(tmp, 0.5 * np.sin(2 * np.pi * 440.0 * t).astype(np.float32), 16000)
                y2 = capture.load_audio(tmp, cfg.sr)
                assert len(y2) > 15000, f"mp3 解码异常: len={len(y2)}"
                os.remove(tmp)
                return f"decode {len(y2)} samples（libsndfile 原生）"

            def _song_match():
                from core.song_match import match_song
                mr = match_song(notes, top_k=3)
                assert mr["matched"], f"未命中（best={mr.get('best_score', 0):.0f}%）"
                return f"命中《{mr['candidates'][0]['title_zh']}》{mr['best_score']:.0f}%"

            def _play_smoke():
                """播放死锁回归（frozen 交付验收）：旧版 play() 持锁调
                stop() 二次获取不可重入锁，点「播放钢琴曲」GUI 永久冻结。
                无声卡环境（PortAudio 无设备）降级跳过，不算失败。

                计时前热身：frozen 进程首次开流含一次性 PortAudio 冷启动
                （Pa_Initialize + 设备枚举 + WASAPI 探测），实测可达 2s+
                （构建后杀软扫描新 exe 时更甚），与死锁无关。热身吸收冷
                启动后，play() 本体（建流+启线程）远低于阈值；死锁表现为
                永久阻塞，3s 阈值仍必然命中。"""
                from app.audio_play import play_score, stop
                pn = [{"midi": 60 + i, "dur": 0.1} for i in range(4)]
                try:
                    import sounddevice as _sd
                    _sd.query_devices()               # 热身 1：Pa_Initialize
                    _sd.RawOutputStream(              # 热身 2：首流设备打开
                        samplerate=16000, blocksize=2048,
                        dtype="int16", channels=1).close()
                    t0 = time.time()
                    play_score(pn, bpm=120, sr=22050)
                    dt = time.time() - t0
                    stop()
                except Exception as e:                # 无声卡/被占用
                    if isinstance(e, _sd.PortAudioError):
                        return "跳过（无音频设备）"
                    raise
                assert dt < 3.0, f"play() 阻塞 {dt:.2f}s（死锁回归！）"
                return f"play 返回耗时 {dt*1000:.0f}ms，无死锁"

            _run("MusicXML 导出", _musicxml)
            _run("标准歌谱 PNG 渲染", _score_png)
            _run("钢琴合成（试听链路）", _synth)
            _run("钢琴播放冒烟（死锁回归）", _play_smoke)
            _run("mp3 解码", _mp3)
            _run("离线曲库匹配（DTW）", _song_match)
            report["full_chain"] = checks

            # ---- 真实音频回归：manifest expected_midi 真值逐一比对 ----
            by_key = {(m["title_zh"], m["timbre"]): m for m in manifest}
            rcfg = Config()
            rcfg.enable_denoise = True     # 样例含底噪（SNR 42dB）
            eng = Melody2Score(rcfg)
            rows, exact_hits, tols, pcs = [], 0, [], []
            for title, timbre in _REGRESS_PAIRS:
                it = by_key.get((title, timbre))
                if not it:
                    rows.append({"title": title, "timbre": timbre,
                                 "skip": "样例缺失"})
                    continue
                wav2 = resource_path("audio", os.path.basename(it["file"]))
                y2 = capture.load_audio(wav2, rcfg.sr)
                r = eng.recognize({"kind": "array", "y": y2,
                                   "sr": rcfg.sr, "cfg": rcfg})
                got = [n["midi"] for n in r["notes"]]
                exp = it["expected_midi"]
                exact = (got == exp)
                tol = _tol_match(got, exp)
                pc = (len(set(x % 12 for x in got) & set(x % 12 for x in exp))
                      / max(1, len(set(x % 12 for x in exp))))
                exact_hits += int(exact)
                tols.append(tol)
                pcs.append(pc)
                rows.append({"title": title, "timbre": timbre, "exact": exact,
                             "tol": round(tol, 3), "pc": round(pc, 3),
                             "notes": f"{len(got)}/{len(exp)}",
                             "bpm": round(r.get("bpm") or 0, 1)})
            n = len(tols)
            avg_tol = round(sum(tols) / n, 3) if n else 0.0
            avg_pc = round(sum(pcs) / n, 3) if n else 0.0
            report["regression"] = {
                "samples": rows, "n": n, "exact_hits": exact_hits,
                "avg_tol": avg_tol, "avg_pc": avg_pc,
                "pass": bool(n > 0 and avg_tol >= 0.85 and exact_hits >= n * 0.5),
            }

            report["pass"] = bool(
                report["pass"]
                and all(c["pass"] for c in checks.values())
                and report["regression"]["pass"])

        print("[SELFTEST] " + _json.dumps(report, ensure_ascii=False))
        if out_path:
            with open(out_path, "w", encoding="utf-8") as f:
                _json.dump(report, f, ensure_ascii=False, indent=2)
        return 0 if report["pass"] else 1
    except Exception:
        tb = traceback.format_exc()
        traceback.print_exc()
        if out_path:
            try:
                import json as _json2
                with open(out_path, "w", encoding="utf-8") as f:
                    _json2.dump({"pass": False, "error": tb[-4000:]}, f,
                                ensure_ascii=False, indent=2)
            except OSError:
                pass
        return 1


def _sheet_repro() -> int:
    """GUI 环境歌谱生成复现模式（打包产物故障诊断专用）：

    Melody2Score.exe --sheet-repro [输出.json]

    与 --selftest（主线程、无 Qt）不同，本模式 100% 复刻真实 GUI 的执行
    时序：QApplication 先行创建 → 识别（QThread）→ 歌谱生成（SheetWorker
    QThread）。用于暴露「仅 Qt 多线程环境」才会出现的异常；windowed 发行版
    stderr 为 NullWriter，traceback 全量落 JSON 报告，不静默丢失。
    """
    out_path = None
    args = [a for a in sys.argv[2:] if not a.startswith("--")]
    if args:
        out_path = args[0]
    report = {"mode": "sheet-repro", "frozen": is_frozen(),
              "python": sys.version.split()[0]}
    try:
        from core.pipeline import Melody2Score
        from core import capture

        app = QApplication(sys.argv)     # 与真实 GUI 一致：Qt 运行时先就位
        man_path = resource_path("audio", "manifest.json")
        with open(man_path, encoding="utf-8") as f:
            manifest = json.load(f)
        item = manifest[0]
        wav = resource_path("audio", os.path.basename(item["file"]))
        cfg = Config()
        y = capture.load_audio(wav, cfg.sr)
        res = Melody2Score(cfg).recognize(
            {"kind": "array", "y": y, "sr": cfg.sr, "cfg": cfg})
        report["recognize"] = {
            "notes": len(res.get("notes", [])),
            "backend": res.get("backend"),
            "bpm": res.get("bpm"),
        }

        # 与真实 GUI 完全一致：SheetWorker 在 QThread 中生成歌谱。
        # 以 worker.outcome 直读属性为判据（信号经队列投递需事件循环驱动，
        # processEvents 仅用于维持 Qt 运行时活性，不作为结果通道）。
        w = SheetWorker(res, "复现测试")
        w.start()
        deadline = time.time() + 180
        while not w.isFinished() and time.time() < deadline:
            app.processEvents()
            time.sleep(0.05)
        w.wait(5000)
        if w.outcome is None:
            report["sheet"] = {"pass": False,
                               "traceback": "worker 未在 180s 内产出结果（线程疑似死锁）"}
        elif w.outcome[0] == "ok":
            report["sheet"] = {"pass": True, "path": w.outcome[1],
                               "size": os.path.getsize(w.outcome[1])}
        else:
            report["sheet"] = {"pass": False, "traceback": w.outcome[1]}
    except Exception:
        report["fatal"] = traceback.format_exc()[-4000:]
    report["pass"] = bool(report.get("sheet", {}).get("pass"))
    if out_path:
        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(report, f, ensure_ascii=False, indent=2)
    print("[SHEET-REPRO] pass=%s" % report["pass"])
    return 0 if report["pass"] else 1


def _ensure_windowed_streams() -> None:
    """windowed 发行版 std 流兜底（PyInstaller console=False 全局根因修复）。

    windowed 模式下 sys.stdout / sys.stderr 均为 None，任何第三方库的
    print / sys.stderr.write 都会 AttributeError：
      - jianpu-ly：30+ 处警告路径（已由 jianpu_render._run_jianpu_ly
        局部捕获兜底，含警告文本回传诊断）；
      - music21：MusicXML 导出时写环境警告 → selftest 实测必炸；
      - 其它库的零星 print。
    入口处一次性替换为丢弃式 writer（异常链路已有各自的错误回传
    机制——信号 / selftest JSON / RuntimeError，不依赖 stderr）。
    """
    class _NullWriter:
        def write(self, *_a): return 0
        def flush(self): pass
        def isatty(self): return False
    if sys.stdout is None:
        sys.stdout = _NullWriter()
    if sys.stderr is None:
        sys.stderr = _NullWriter()


def main():
    _ensure_windowed_streams()   # 必须先于一切第三方库调用（含 selftest）
    if "--selftest" in sys.argv or "--selftest-full" in sys.argv:
        sys.exit(_selftest())
    if "--sheet-repro" in sys.argv:
        sys.exit(_sheet_repro())
    app = QApplication(sys.argv)
    app.setStyle("Fusion")
    win = MainWindow()
    win.show()
    sys.exit(app.exec_())


if __name__ == "__main__":
    main()
