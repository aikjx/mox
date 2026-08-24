# core 包：哼唱旋律转歌谱的核心分层模块
# 对应架构：采集 / 预处理 / 音高检测 / 音乐解析 / 歌谱生成 / 编排
from .config import Config
from .pipeline import Melody2Score

__all__ = ["Config", "Melody2Score"]
