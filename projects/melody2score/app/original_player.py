"""Original audio transport using Qt's native decoder and audio clock."""
import os
from PyQt5.QtCore import QUrl, QTemporaryFile, pyqtSignal
from PyQt5.QtMultimedia import QMediaPlayer, QMediaContent
from PyQt5.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QPushButton, QLabel, QSlider
from PyQt5.QtCore import Qt


class OriginalPlayer(QWidget):
    starting = pyqtSignal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.media = QMediaPlayer(self)
        self._temporary = None
        layout = QVBoxLayout(self)
        self.caption = QLabel("原曲播放器 · 未载入")
        layout.addWidget(self.caption)
        row = QHBoxLayout()
        self.toggle_button = QPushButton("播放")
        self.toggle_button.clicked.connect(self.toggle)
        row.addWidget(self.toggle_button)
        stop = QPushButton("停止")
        stop.clicked.connect(self.media.stop)
        row.addWidget(stop)
        self.volume = QSlider(Qt.Horizontal)
        self.volume.setRange(0, 100)
        self.volume.setValue(80)
        self.volume.setAccessibleName("音量")
        self.volume.valueChanged.connect(self.media.setVolume)
        self.media.setVolume(80)
        row.addWidget(QLabel("音量"))
        row.addWidget(self.volume)
        layout.addLayout(row)
        self.seek = QSlider(Qt.Horizontal)
        self.seek.setAccessibleName("原曲播放进度")
        self.seek.setEnabled(False)
        self.seek.sliderReleased.connect(lambda: self.media.setPosition(self.seek.value()))
        layout.addWidget(self.seek)
        self.clock = QLabel("00:00 / 00:00")
        layout.addWidget(self.clock)
        self.media.durationChanged.connect(self._duration)
        self.media.positionChanged.connect(self._position)
        self.media.seekableChanged.connect(self.seek.setEnabled)
        self.media.stateChanged.connect(lambda state: self.toggle_button.setText(
            "暂停" if state == QMediaPlayer.PlayingState else "播放"))
        self.media.error.connect(lambda _: self.caption.setText("播放失败：" + self.media.errorString()))

    def set_source(self, path=None, data=None):
        url = QUrl.fromLocalFile(os.path.abspath(path)) if path else None
        if url is not None and self.media.media().canonicalUrl() == url:
            return
        self.media.stop()
        self.media.setMedia(QMediaContent())
        if self._temporary is not None:
            self._temporary.remove()
            self._temporary = None
        if data is not None:
            temp = QTemporaryFile(self)
            if not temp.open() or temp.write(data) != len(data):
                raise OSError("无法创建录音试听文件")
            temp.close()
            self._temporary = temp
            url = QUrl.fromLocalFile(temp.fileName())
        self.seek.setValue(0)
        self.seek.setEnabled(False)
        self.caption.setText("原曲 · " + (os.path.basename(path) if path else "麦克风录音"))
        self.media.setMedia(QMediaContent(url))

    def toggle(self):
        if self.media.state() == QMediaPlayer.PlayingState:
            self.media.pause()
        elif not self.media.media().isNull():
            self.starting.emit()
            self.media.play()

    def _duration(self, duration):
        self.seek.setRange(0, max(0, duration))
        self._position(self.media.position())

    def _position(self, position):
        if not self.seek.isSliderDown():
            self.seek.setValue(position)
        def fmt(ms):
            seconds = max(0, ms // 1000)
            return f"{seconds // 60:02d}:{seconds % 60:02d}"
        self.clock.setText(fmt(position) + " / " + fmt(self.media.duration()))
