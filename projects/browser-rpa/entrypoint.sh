#!/bin/sh
# Xvfb 提供虚拟显示，供 playwright codegen 有头录制使用
Xvfb :99 -screen 0 1920x1080x24 >/dev/null 2>&1 &
export DISPLAY=:99
exec python run.py
