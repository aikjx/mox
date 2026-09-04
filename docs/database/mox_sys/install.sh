#!/usr/bin/env bash
# =============================================================================
# 一键安装 mox_sys 归一化母版（Linux / macOS）
# 用法：  ./install.sh [SERVER] [PORT] [USER] [PASSWORD]
# 默认：  127.0.0.1 3306 root （无密码）
# 说明：  单一权威 DDL 为 mox_sys-universal-template.sql（含全部 56 张表）。
# =============================================================================
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SQL="$DIR/mox_sys-universal-template.sql"
SERVER="${1:-127.0.0.1}"
PORT="${2:-3306}"
USER="${3:-root}"
PASS="${4:-}"

PASS_ARG=""
if [ -n "$PASS" ]; then PASS_ARG="-p$PASS"; fi

echo ">> 安装 mox_sys 归一化母版 -> mysql -h$SERVER -P$PORT -u$USER $PASS_ARG"
mysql -h"$SERVER" -P"$PORT" -u"$USER" $PASS_ARG --default-character-set=utf8mb4 < "$SQL"
echo ">> mox_sys 归一化母版安装完成（库 mox_v3）"
