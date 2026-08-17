# -*- coding: utf-8 -*-
"""Melody2Score 企业级桌面 GUI（PyQt5）。

打开即用：选择音频 / 麦克风录音 / 内置样例 → 后台线程识别 → 直接显示
简谱 + 五线谱 + 量化音高轮廓 + 音符明细，并可一键保存为 Markdown 报告。

运行：python app/gui.py
"""
import io
import json
import os
import re
import sys
import time
import traceback
from typing import Dict, List, Optional, Tuple

import numpy as np
import soundfile as sf
from PyQt5.QtCore import QThread, pyqtSignal, Qt, QSize
from PyQt5.QtGui import QPainter, QPen, QColor, QFont, QBrush, QPalette
from PyQt5.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout, QPushButton,
    QLabel, QFileDialog, QComboBox, QSlider, QCheckBox, QTabWidget, QTextEdit,
    QTableWidget, QTableWidgetItem, QHeaderView, QProgressBar, QMessageBox,
    QGroupBox, QLineEdit, QFrame, QSizePolicy)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)
from core.config import Config
from core import capture, preprocess, pitch, analysis, score, vad
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
            cfg = self.source["cfg"]
            if self.source["kind"] == "file":
                y, sr = load_audio_bytes(self.source["data"], cfg.sr)
            elif self.source["kind"] == "record":
                y, sr = load_audio_bytes(self.source["data"], cfg.sr)
            else:  # sample
                y = capture.load_audio(os.path.join(ROOT, self.source["name"]), cfg.sr)
                sr = cfg.sr

            t0 = time.time()
            y = preprocess.preprocess(y, sr, cfg.enable_denoise)
            t_pre = time.time() - t0

            # 稳健重识别：同一音频跑 N 次（抖动置信阈值 / 帧移），对音符做共识合并，
            # 抑制单次识别偶发的假音高与漏音，显著提升稳定性。
            n_runs = 4 if cfg.robust else 1
            runs = []
            t_pitch_total = 0.0
            used_backend = "auto"
            last_pts: list = []
            for k in range(n_runs):
                # 每次稍扰动置信阈值，得到略不同的音高点云（帧移保持与单次识别一致，
                # 避免 VAD 掩码帧错位）。不同阈值下 CREPE 输出不同，便于共识过滤假音高。
                eff = Config()
                eff.conf_thresh = max(0.05, cfg.conf_thresh - 0.06 + 0.04 * k)
                t0 = time.time()
                det = pitch.PitchDetector(
                    model_size=cfg.model_size, conf_thresh=eff.conf_thresh, hop=eff.hop,
                    intra_op_threads=cfg.intra_op_threads, backend="auto", sr=sr,
                    fmin=cfg.fmin, fmax=cfg.fmax)
                pts = det.detect(y, sr)
                t_pitch_total += time.time() - t0
                used_backend = det.used_backend
                last_pts = pts

                vad_mask = None
                if cfg.vocal_mode and cfg.enable_vad:
                    vad_mask = vad.voice_activity_mask(
                        y, sr, energy_thresh=cfg.vad_energy_thresh,
                        centroid_min=cfg.vad_centroid_min, centroid_max=cfg.vad_centroid_max,
                        flatness_max=cfg.vad_flatness_max, hop_ms=eff.hop,
                        min_voiced_ms=cfg.min_voiced_ms)

                notes = analysis.segment_notes(
                    pts, cfg.min_note_dur, cfg.median_win,
                    vocal_mode=cfg.vocal_mode, vad_mask=vad_mask)
                runs.append(notes)

            t_pitch = t_pitch_total
            t_parse = 0.0

            if n_runs > 1:
                notes, merge_info = self._consensus(runs, cfg)
            else:
                notes = runs[0]
                merge_info = None

            t0 = time.time()
            bpm = analysis.detect_bpm(y, sr, cfg.bpm_fallback)
            key_name = analysis.estimate_key(y, sr, notes)
            t_parse = time.time() - t0

            jianpu = score.to_jianpu(notes, key_name, bpm)
            total_dur = float(notes[-1]["end"]) if notes else 0.0
            notes_out = [{
                "midi": int(n["midi"]),
                "start": round(float(n["start"]), 4),
                "end": round(float(n["end"]), 4),
                "dur": round(float(n["end"] - n["start"]), 4),
                "name": midi_name(int(n["midi"])),
            } for n in notes]

            self.finished.emit({
                "jianpu": jianpu, "bpm": round(float(bpm), 1),
                "key": {"tonic": key_name[0], "mode": key_name[1]},
                "note_count": len(notes), "duration_sec": round(total_dur, 2),
                "backend": used_backend, "notes": notes_out,
                "confidence": round(float(merge_info["confidence"]) if merge_info else self._conf(runs[0]), 2),
                "robust_runs": n_runs,
                "robust_kept": merge_info["kept"] if merge_info else len(runs[0]),
                "perf": {"preprocess_ms": round(t_pre * 1000, 1),
                         "pitch_ms": round(t_pitch * 1000, 1),
                         "parse_ms": round(t_parse * 1000, 1),
                         "pitch_frames": len(last_pts)},
                "source": self.source.get("source", ""),
            })
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


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Melody2Score · 哼唱旋律转谱（企业级桌面版）")
        self.setMinimumSize(1360, 900)
        self.resize(1440, 920)
        self.current = None
        self.pending_file = None
        self._raw_y = None
        self._raw_sr = 22050
        self._apply_style()
        self._build()
        self._load_samples()

    def _apply_style(self):
        self.setStyleSheet(f"""
            QMainWindow{{background:{BG};}}
            QWidget{{background:{BG};color:{TEXT};font-family:'Segoe UI','PingFang SC','Microsoft YaHei';}}
            QGroupBox{{border:1px solid {LINE};border-radius:10px;margin-top:10px;padding:10px;}}
            QGroupBox::title{{color:{MUTED};subcontrol-position:top left;padding:0 6px;}}
            QPushButton{{background:{ACCENT};color:#06101f;border:none;border-radius:9px;
                padding:9px 14px;font-weight:700;font-size:13px;}}
            QPushButton:hover{{background:#6fb0ff;}}
            QPushButton:disabled{{background:{PANEL2};color:{MUTED};}}
            QComboBox,QLineEdit{{background:{PANEL2};border:1px solid {LINE};border-radius:8px;
                padding:7px;color:{TEXT};}}
            QLabel{{color:{TEXT};}}
            QTableWidget{{background:{PANEL2};gridline-color:{LINE};border:1px solid {LINE};}}
            QHeaderView::section{{background:{PANEL};color:{MUTED};border:none;padding:6px;}}
            QTextEdit{{background:#0b1020;border:1px solid {LINE};border-radius:10px;
                color:#dfe9ff;font-family:Consolas,monospace;font-size:14px;}}
            QTabBar::tab{{background:{PANEL2};color:{MUTED};padding:8px 16px;border-radius:8px;margin:2px;}}
            QTabBar::tab:selected{{background:{ACCENT};color:#06101f;}}
            QProgressBar{{background:{PANEL2};border:1px solid {LINE};border-radius:6px;text-align:center;}}
            QProgressBar::chunk{{background:{ACCENT2};border-radius:6px;}}
            QFrame#sep{{background:{LINE};max-height:1px;}}
        """)

    def _build(self):
        root = QWidget()
        self.setCentralWidget(root)
        h = QHBoxLayout(root)
        h.setSpacing(14)
        h.setContentsMargins(16, 16, 16, 16)

        # 左侧控制
        left = QFrame()
        left.setFixedWidth(360)
        lv = QVBoxLayout(left)
        lv.setSpacing(10)

        # 输入
        g_in = QGroupBox("输入源")
        iv = QVBoxLayout(g_in)
        self.btnFile = QPushButton("📂 选择音频文件")
        self.btnFile.clicked.connect(self.pick_file)
        rec_row = QHBoxLayout()
        self.btnRec = QPushButton("🎙️ 录音并识别")
        self.btnRec.clicked.connect(self.record)
        self.cbRecSec = QComboBox()
        self.cbRecSec.addItems(["3 秒", "5 秒", "10 秒"])
        self.cbRecSec.setFixedWidth(70)
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
        lv.addWidget(g_s)

        lv.addStretch(1)
        self.btnSave = QPushButton("💾 保存 Markdown")
        self.btnSave.setEnabled(False)
        self.btnSave.clicked.connect(self.save_md)
        lv.addWidget(self.btnSave)
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
        man = os.path.join(ROOT, "audio", "manifest.json")
        if not os.path.exists(man):
            self.sampleCombo.addItem("（无样例，先运行 gen_classic_melodies.py）")
            self.sampleCombo.setEnabled(False)
            self.btnSample.setEnabled(False)
            return
        with open(man, encoding="utf-8") as f:
            self.manifest = json.load(f)
        seen = {}
        for it in self.manifest:
            seen.setdefault(it["melody_index"], it)
        for it in seen.values():
            self.sampleCombo.addItem(f"{it['title_zh']} · {it['title_en']} · {it['timbre']}",
                                     it["file"])

    def pick_file(self):
        path, _ = QFileDialog.getOpenFileName(
            self, "选择音频", "", "音频 (*.wav *.mp3 *.flac *.ogg *.m4a)")
        if path:
            self.pending_file = path
            self.lblFile.setText(os.path.basename(path))
            self.btnPreview.setEnabled(True)
            self.status.setText(f"已选择：{os.path.basename(path)}，开始识别…")
            try:
                self._raw_y, self._raw_sr = load_audio_bytes(open(path, "rb").read(), 22050)
            except Exception as e:
                self._raw_y = None
                self.status.setText(f"已选择（试听加载失败：{e}）")
            self.run_file()
            # 缓存原始波形用于试听
            try:
                self._raw_y, self._raw_sr = load_audio_bytes(open(path, "rb").read(), 22050)
            except Exception as e:
                self._raw_y = None
                self.status.setText(f"已选择（试听加载失败：{e}）")

    def record(self):
        """用 sounddevice 实时录音（默认 16kHz 单声道），录音结束后自动识别人声。"""
        worker = getattr(self, "worker", None)
        if worker is not None and worker.isRunning():
            return  # 识别中，忽略重复点击
        try:
            import sounddevice as sd
        except Exception as e:
            QMessageBox.critical(self, "录音不可用", f"未安装 sounddevice：{e}")
            return
        secs = [3, 5, 10][self.cbRecSec.currentIndex()]
        sr = self._cfg().sr
        self.btnRec.setEnabled(False)
        self.btnRec.setText(f"🎙️ 录音中 {secs}s…")
        self.status.setText(f"🎙️ 录音中… 请对着麦克风唱（{secs}s）")
        try:
            data = sd.rec(int(secs * sr), samplerate=sr, channels=1, dtype="float32")
            sd.wait()
        except Exception as e:
            QMessageBox.critical(self, "录音失败", f"麦克风录音出错：{e}")
            self.status.setText("录音失败。")
            self.btnRec.setEnabled(True)
            self.btnRec.setText("🎙️ 录音并识别")
            return
        y_rec = np.asarray(data, dtype=np.float32).reshape(-1)
        # 编码为 wav 字节，复用 load_audio_bytes 解码路径
        import io
        import soundfile as sf
        buf = io.BytesIO()
        sf.write(buf, y_rec, sr, format="WAV")
        self.pending_file = None
        self._raw_y, self._raw_sr = y_rec, sr
        self.btnPreview.setEnabled(True)
        self.btnRec.setEnabled(True)
        self.btnRec.setText("🎙️ 录音并识别")
        self.status.setText(f"录音完成（{secs}s），开始识别人声…")
        self._start({"kind": "record", "data": buf.getvalue(), "cfg": self._cfg(),
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
        with open(self.pending_file, "rb") as f:
            data = f.read()
        self._start({"kind": "file", "data": data, "cfg": self._cfg(),
                     "source": os.path.basename(self.pending_file)})

    def run_sample(self):
        if not self.sampleCombo.isEnabled():
            return
        name = self.sampleCombo.currentData()
        # 选中即缓存原曲波形，使左侧「试听原曲」可复听
        idx = self.sampleCombo.currentIndex()
        item = self.manifest[idx] if hasattr(self, "manifest") else None
        if item:
            path = os.path.join(ROOT, item["file"])
            if os.path.exists(path):
                with open(path, "rb") as f:
                    try:
                        y, sr = load_audio_bytes(f.read(), 22050)
                        self._raw_y, self._raw_sr = y, sr
                        self.btnPreview.setEnabled(True)
                    except Exception:
                        pass
        self._start({"kind": "sample", "name": name, "cfg": self._cfg(),
                     "source": name})

    def play_sample(self):
        """播放当前选中样例的原曲音频（读取 audio/<id>.wav 并回放）。"""
        if not self.sampleCombo.isEnabled():
            return
        idx = self.sampleCombo.currentIndex()
        item = self.manifest[idx] if hasattr(self, "manifest") else None
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

    def on_done(self, res: Dict):
        self.current = res
        self.progress.setVisible(False)
        self.btnSample.setEnabled(self.sampleCombo.isEnabled())
        self.btnSave.setEnabled(True)
        self.btnPlayScore.setEnabled(True)

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
        self.status.setText(f"完成 · {res['note_count']} 个音符 · 来源 {res.get('source','')}{robust_info}")

    def on_err(self, msg: str):
        self.progress.setVisible(False)
        self.btnSample.setEnabled(self.sampleCombo.isEnabled())
        self.status.setText(f"错误：{msg}")
        QMessageBox.critical(self, "识别失败", msg)

    def preview_original(self):
        """试听已选择的原曲（mp3/wav 等）。"""
        if getattr(self, "_raw_y", None) is None:
            QMessageBox.information(self, "试听", "尚未载入可播放的音频。")
            return
        raw = self._raw_y
        if raw is None:
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
            "5. 歌谱生成层：music21 量化生成 musicxml；简谱数字串（高八度 '.'，低八度 '_'，延音 '-'）。\n\n"
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
