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
from PyQt5.QtCore import QThread, pyqtSignal, Qt, QSize, QTimer
from PyQt5.QtGui import QPainter, QPen, QColor, QFont, QBrush, QPalette, QPixmap
from PyQt5.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout, QPushButton,
    QLabel, QFileDialog, QComboBox, QSlider, QCheckBox, QTabWidget, QTextEdit,
    QTableWidget, QTableWidgetItem, QHeaderView, QProgressBar, QMessageBox,
    QGroupBox, QLineEdit, QFrame, QSizePolicy, QDialog, QScrollArea)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)
from core.config import Config
from core import score_sheet
from app.audio_play import play_raw, play_score, stop as audio_stop

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
            from core.pipeline import Melody2Score
            result = Melody2Score(self.source["cfg"]).recognize(self.source)
            self.finished.emit(result)
        except Exception as e:
            traceback.print_exc()
            self.error.emit(str(e))

    @staticmethod
    def _conf(notes):
        """简单置信度：基于音符平均时长与数量（越长越多越可信）。"""
        if not notes:
            return 0.0
        durs = [n["end"] - n["start"] for n in notes]
        avg = float(np.mean(durs))
        # 经验映射：平均时长 0.15s 视为高置信
        return float(min(1.0, 0.4 + avg / 0.3))

    @staticmethod
    def _consensus(runs: List[List[Dict]], cfg) -> Tuple[List[Dict], Dict]:
        """音符级共识合并。

        把多次识别得到的音符按时间重叠聚成簇：同一簇内取「出现次数最多 +
        时间跨度最长」的 MIDI 作为共识音高；簇的起止取各次并集。
        仅在 ≥2 次识别中都出现的音符才保留（过滤单次偶发假音高），
        但单次出现的长音（>2*min_note_dur）也保留以防漏音。
        """
        # 收集所有音符，按 start 排序
        all_notes = []
        for notes in runs:
            for n in notes:
                all_notes.append(n)
        all_notes.sort(key=lambda n: n["start"])

        clusters = []
        for n in all_notes:
            placed = False
            for c in clusters:
                # 时间重叠判定：重叠量需超过较短音符的一定比例，才视为同一发声事件。
                # 仅端点相切（相邻音符首尾相接）不算重叠，避免把真实相邻音符误并为一簇。
                overlap = False
                for m in c["members"]:
                    ov = min(n["end"], m["end"]) - max(n["start"], m["start"])
                    short = min(n["end"] - n["start"], m["end"] - m["start"])
                    if ov > 0.6 * short:
                        overlap = True
                        break
                if overlap:
                    c["members"].append(n)
                    placed = True
                    break
            if not placed:
                clusters.append({"members": [n]})

        merged = []
        kept = 0
        conf_sum = 0.0
        for c in clusters:
            members = c["members"]
            # 统计各 MIDI 出现次数
            from collections import Counter
            cnt = Counter(int(round(m["midi"])) for m in members)
            best_midi, best_cnt = cnt.most_common(1)[0]
            # 簇起止仅取共识音高(best_midi)成员的时间并集，避免偶发假音的时间污染边界
            consensus_members = [m for m in members if int(round(m["midi"])) == best_midi]
            start = min(m["start"] for m in consensus_members)
            end = max(m["end"] for m in consensus_members)
            # 保留条件：出现 ≥2 次，或单次但足够长（防漏音）
            long_single = best_cnt == 1 and (end - start) > 2 * cfg.min_note_dur
            if best_cnt >= 2 or long_single:
                merged.append({"midi": best_midi, "start": start, "end": end})
                # 簇内一致度作为该音符置信
                conf_sum += best_cnt / len(members)
                kept += 1

        merged.sort(key=lambda n: n["start"])
        # 整体置信度：平均簇一致度（共识越高越可信）
        confidence = (conf_sum / len(merged)) if merged else 0.0
        return merged, {"kept": len(merged), "confidence": confidence}


class StaffView(QWidget):
    """五线谱 Canvas（Qt 原生绘制）。"""
    def __init__(self):
        super().__init__()
        self.notes = []
        self.key = {"tonic": "C", "mode": "major"}
        self.setMinimumHeight(260)

    def setData(self, notes, key):
        self.notes = notes
        self.key = key
        self.update()

    def paintEvent(self, ev):
        p = QPainter(self)
        p.fillRect(self.rect(), QColor(BG))
        W, H = self.width(), self.height()
        padL, padR = 40, 20
        lineGap = 11
        topY = 30
        staffH = lineGap * 4
        pen = QPen(QColor(LINE))
        pen.setWidth(1)
        p.setPen(pen)
        for i in range(5):
            y = topY + i * lineGap
            p.drawLine(padL, y, W - padR, y)
        p.setPen(QColor(ACCENT))
        p.setFont(QFont("Serif", 22))
        p.drawText(8, topY + staffH - 4, "𝄞")

        if not self.notes:
            p.setPen(QColor(MUTED))
            p.setFont(QFont("Sans", 12))
            p.drawText(padL, topY + staffH / 2, "（无可视音符）")
            return

        minM = min(n["midi"] for n in self.notes) - 2
        maxM = max(n["midi"] for n in self.notes) + 2
        span = max(1, maxM - minM)
        yOf = lambda m: topY + staffH - ((m - minM) / span) * (staffH - 2) - 1
        total = self.notes[-1]["end"] or 1
        xOf = lambda t: padL + (t / total) * (W - padL - padR)
        spacing = (W - padL - padR) / max(len(self.notes), 1)

        p.setBrush(QBrush(QColor(ACCENT2)))
        p.setPen(QColor(ACCENT2))
        for n in self.notes:
            x = xOf(n["start"])
            y = yOf(n["midi"])
            p.drawEllipse(int(x) - 7, int(y) - 5, 14, 10)
            p.drawLine(int(x + 5), int(y), int(x + 5), int(y - 22))
            if n["dur"] > (total / len(self.notes)) * 1.5:
                p.fillRect(int(x + 5), int(y - 22),
                           int(min(n["dur"] / total * (W - padL - padR), spacing * 3)), 2)
        p.setPen(QColor(MUTED))
        p.setFont(QFont("Sans", 10))
        p.drawText(10, topY - 8, "高")
        p.drawText(10, topY + staffH + 12, "低")


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
        self.setMinimumSize(1680, 1050)
        self.resize(1760, 1080)
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
        self.cbRobust.setChecked(True)
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
        man = os.path.join(ROOT, "audio", "manifest.json")
        self.manifest = []
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
            path = os.path.join(ROOT, item["file"])
            if os.path.exists(path):
                self._pending_sample_path = path
                self.btnPreview.setEnabled(True)
        self._start({"kind": "sample", "name": file_rel, "cfg": self._cfg(),
                     "source": file_rel})

    def play_sample(self):
        """播放当前选中样例的原曲音频（读取 audio/<id>.wav 并回放）。"""
        if not self.sampleCombo.isEnabled():
            return
        file_rel = self.sampleCombo.currentData()
        item = self._sample_by_file(file_rel)
        if not item:
            return
        path = os.path.join(ROOT, item["file"])
        if not os.path.exists(path):
            QMessageBox.information(self, "播放原曲",
                                    f"未找到样例音频：{item['file']}\n请先运行 gen_classic_melodies.py 生成。")
            return
        with open(path, "rb") as f:
            try:
                y, sr = load_audio_bytes(f.read(), 22050)
            except Exception as e:
                QMessageBox.critical(self, "播放失败", f"解码样例音频出错：{e}")
                return
        self._raw_y, self._raw_sr = y, sr
        self.btnPreview.setEnabled(True)
        self.status.setText(f"▶ 试听原曲：{item['title_zh']} · {item['timbre']}")
        from app.audio_play import play_raw
        play_raw(y, sr)

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
        audio_dir = os.path.join(ROOT, "audio")
        try:
            os.makedirs(audio_dir, exist_ok=True)
        except Exception as e:
            QMessageBox.critical(self, "添加样例", f"无法创建 audio 目录：{e}")
            return
        base = os.path.splitext(os.path.basename(path))[0]
        ext = os.path.splitext(path)[1] or ".wav"
        ts = time.strftime("%Y%m%d_%H%M%S")
        new_file = f"audio/user_sample_{ts}{ext}"
        dst = os.path.join(ROOT, new_file)
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
            with open(os.path.join(ROOT, "audio", "manifest.json"), "w", encoding="utf-8") as f:
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
        self.staff.setData(res["notes"], res["key"])
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

        # 自动生成标准歌谱并预览
        self._auto_sheet_path = self._generate_sheet(res, "png")
        if self._auto_sheet_path:
            self.sheetLabel.setText("")
            self.sheetLabel.setPixmap(QPixmap(self._auto_sheet_path).scaledToWidth(
                self.sheetLabel.width() - 20, Qt.SmoothTransformation))
        else:
            self.sheetLabel.setText("标准歌谱生成失败（可能缺少 matplotlib 或中文字体）。")

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
            play_raw(raw, self._raw_sr)
        except Exception as e:
            self.status.setText(f"播放失败：{e}")

    def play_score_audio(self):
        """按识别出的音符序列合成钢琴曲播放。"""
        if not self.current or not self.current.get("notes"):
            QMessageBox.information(self, "播放", "尚无可播放的识别结果。")
            return
        try:
            self.status.setText("合成并播放钢琴曲中…（后台进行，界面不卡）")
            play_score(self.current["notes"], sr=22050)
        except Exception as e:
            self.status.setText(f"钢琴播放失败：{e}")

    def _generate_sheet(self, res: Dict, fmt: str) -> Optional[str]:
        """用 core.score_sheet 生成标准歌谱并返回路径。"""
        try:
            title = self.titleEdit.text() or "未命名旋律"
            export_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "exports")
            os.makedirs(export_dir, exist_ok=True)
            safe = re.sub(r"[^\w一-鿿-]", "_", title)[:40]
            ts = time.strftime("%Y%m%d_%H%M%S")
            fname = f"{safe or 'melody'}_标准歌谱_{ts}.{fmt}"
            fpath = os.path.join(export_dir, fname)
            score_sheet.export_score(
                notes=res.get("notes", []),
                key=res.get("key", {"tonic": "C", "mode": "major"}),
                bpm=float(res.get("bpm", 120)),
                output_path=fpath,
                title=title,
            )
            return fpath
        except Exception as e:
            traceback.print_exc()
            return None

    def export_sheet(self, fmt: str):
        if not self.current:
            return
        path = self._generate_sheet(self.current, fmt)
        if path:
            if fmt == "png":
                self.sheetLabel.setPixmap(QPixmap(path).scaledToWidth(
                    self.sheetLabel.width() - 20, Qt.SmoothTransformation))
            QMessageBox.information(self, "已导出", f"标准歌谱已导出：\n{path}")
            self.status.setText(f"已导出 {fmt.upper()}：{os.path.basename(path)}")
        else:
            QMessageBox.warning(self, "导出失败", "标准歌谱导出失败，请检查日志。")

    def save_md(self):
        if not self.current:
            return
        title = self.titleEdit.text() or "未命名旋律"
        ts = time.strftime("%Y%m%d_%H%M%S")
        safe = re.sub(r"[^\w一-鿿-]", "_", title)[:40]
        export_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "exports")
        os.makedirs(export_dir, exist_ok=True)
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
            "matplotlib 渲染标准歌谱图片（PNG/PDF/SVG）。\n\n"
            "优化：调式识别用音符轮廓替代整曲 CQT（提速 ~7x）；简谱以 tonic 的 4 八度音为基准计算八度偏移，"
            "标记符合记谱习惯。")
        with open(fpath, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))
        QMessageBox.information(self, "已保存", f"Markdown 报告已保存：\n{fpath}")
        self.status.setText(f"已保存：{fname}")


def main():
    app = QApplication(sys.argv)
    app.setStyle("Fusion")
    win = MainWindow()
    win.show()
    sys.exit(app.exec_())


if __name__ == "__main__":
    main()
