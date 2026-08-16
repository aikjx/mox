#!/usr/bin/env bash
# 开发板一键运行脚本（树莓派 / RK3568）
# 用法：
#   ./board_run.sh record 6        # 录音 6 秒转谱
#   ./board_run.sh file test.wav   # 对已有文件转谱
set -e

# 限制 onnxruntime CPU 线程，避免占满核导致系统卡顿
export OMP_NUM_THREADS="${OMP_NUM_THREADS:-2}"
export PYTHONUNBUFFERED=1

cd "$(dirname "$0")"

case "$1" in
  record)
    SEC="${2:-5}"
    python3 board/run_board.py record "$SEC" -o /tmp/melody.xml
    ;;
  file)
    [ -z "$2" ] && { echo "用法: ./board_run.sh file <音频路径>"; exit 1; }
    python3 board/run_board.py file "$2" -o /tmp/melody.xml
    ;;
  *)
    echo "用法:"
    echo "  ./board_run.sh record [秒数]"
    echo "  ./board_run.sh file  <音频路径>"
    exit 1
    ;;
esac
