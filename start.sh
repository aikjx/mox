#!/usr/bin/env bash
# 璇玑（infotopograph）一键启动脚本 · POSIX 版 (Linux / macOS / WSL)
#
# 用法:
#   ./start.sh                 # 默认：预检 → 清残留 → 拉起 Web 管理面板；项目服务需在页面上 ▶ 启动
#   ./start.sh --with-services # 与旧版一致：同步启动 auto_start 服务 + 管理面板
#   ./start.sh --dry-run       # 仅预检（二进制、工作目录、依赖关系），不动真格
#   ./start.sh --strict        # 严格模式：任何服务启动失败立即退出非零
#   ./start.sh --with-dashboard # 一键流程最后前台挂起管理面板（Start 默认已启用）
#   ./start.sh --build-rust    # 额外执行 Rust release 构建（遗留后端组件可选）
#   ./start.sh --legacy        # *仅限旧 operator-server 用户*：走原始 cargo 模式（不推荐）
#   ./start.sh --stop          # 一键停止所有服务（按拓扑）
#   ./start.sh --restart       # 一键重启所有 auto_start 服务（严格模式）
#   ./start.sh --verify        # 额外执行 verify（六大公理数学自洽性验证）

set -u
set -o pipefail

# ----- 路径与颜色 -----
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"

PY_BIN=""
for c in python3 python py; do
  if command -v "$c" >/dev/null 2>&1; then
    if "$c" -c '' >/dev/null 2>&1; then
      PY_BIN="$c"; break
    fi
  fi
done

ESC=$'\033'
C_RED="${ESC}[0;31m";   C_GRN="${ESC}[0;32m";   C_YLW="${ESC}[1;33m"
C_BLU="${ESC}[0;34m";   C_CYN="${ESC}[0;36m";   C_RST="${ESC}[0m"
tick="${C_GRN}✔${C_RST}";  cross="${C_RED}✗${C_RST}";  warn="${C_YLW}⚠${C_RST}"

banner() {
  echo
  echo "============================================================"
  echo "  ${C_CYN}璇玑 Mox · 全维数字孪生中台 一键启动（POSIX）${C_RST}"
  echo "  仓库根：${C_BLU}${HERE}${C_RST}"
  echo "============================================================"
}

usage() {
  sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//' | head -n 15
}

# ----- 参数解析 -----
DRY_RUN=0; STRICT=0; WITH_DASH=0; BUILD_RUST=0; LEGACY=0
DO_STOP=0; DO_RESTART=0; DO_VERIFY=0; WITH_SERVICES=0
for arg in "$@"; do
  case "$arg" in
    -h|--help)         usage; exit 0 ;;
    --dry-run)         DRY_RUN=1 ;;
    --strict)          STRICT=1 ;;
    --with-dashboard)  WITH_DASH=1 ;;
    --build-rust)      BUILD_RUST=1 ;;
    --legacy)          LEGACY=1 ;;
    --stop)            DO_STOP=1 ;;
    --restart)         DO_RESTART=1 ;;
    --verify)          DO_VERIFY=1 ;;
    --with-services)   WITH_SERVICES=1 ;;
    *) echo "${cross} 未知参数: $arg" >&2; usage; exit 2 ;;
  esac
done

ensure_py() {
  if [ -z "$PY_BIN" ]; then
    echo "${cross} 未找到 python3 / python。请先安装 Python ≥ 3.10 并添加到 PATH。"
    exit 127
  fi
}

dump_logs() {
  echo
  echo "${warn} 启动/运行失败 → 最近 30 行各服务日志："
  for d in .logs/*.log; do
    [ -e "$d" ] || continue
    echo
    echo "===== $d ====="
    tail -n 30 "$d"
  done
  echo
}

trap 'dump_logs' ERR

banner

ensure_py

# --- 可选：数学公理验证 ---
if [ "$DO_VERIFY" -eq 1 ]; then
  echo
  echo "[1/5] 🧮 verify：六大公理数学自洽性验证"
  if "$PY_BIN" scripts/server-manage.py verify; then
    echo " $tick 公理验证通过"
  else
    echo " $warn 公理验证存在警告（不影响服务启动）"
  fi
fi

# --- 可选：Rust release 构建（遗留/附加） ---
if [ "$BUILD_RUST" -eq 1 ]; then
  if command -v cargo >/dev/null 2>&1; then
    echo
    echo "[*] 🦀 构建 Rust workspace（release）..."
    if ! cargo build --release; then
      echo "${cross} Rust 构建失败，停止" >&2; exit 11
    fi
    echo " $tick Rust 构建完成"
  else
    echo " $warn 未检测到 cargo，跳过 --build-rust"
  fi
fi

# --- 停止模式 ---
if [ "$DO_STOP" -eq 1 ]; then
  echo
  echo "[STOP] 按拓扑停止全部服务..."
  "$PY_BIN" scripts/server-manage.py stop all --force
  exit $?
fi

# --- 重启模式 ---
if [ "$DO_RESTART" -eq 1 ]; then
  echo
  echo "[RESTART] 严格模式重启 auto_start 服务..."
  "$PY_BIN" scripts/server-manage.py restart all --strict
  rc=$?
  if [ "$rc" -ne 0 ]; then echo "${cross} 重启失败（code=$rc）" >&2; exit "$rc"; fi
  echo " $tick 重启完成"
  "$PY_BIN" scripts/server-manage.py list
  exit 0
fi

# --- 遗留 operator-server 模式（默认关闭） ---
if [ "$LEGACY" -eq 1 ]; then
  echo
  echo "${warn} --legacy 模式：仅用于历史遗留 operator-server 分支（默认不启用）"
  if [ ! -f ./target/release/operator-server ]; then
    command -v cargo >/dev/null 2>&1 || { echo "${cross} 需要 cargo"; exit 10; }
    cargo build --release || exit 10
  fi
  echo "🚀 启动遗留 operator-server：http://localhost:3000"
  exec ./target/release/operator-server
fi

# --- 主流程：调用 manage.py bootstrap ---
echo
echo "[MAIN] 调用 scripts/server-manage.py bootstrap（默认：仅面板 → 页面上按需启动服务）"

BOOT_ARGS=(bootstrap --with-dashboard --no-browser)
[ "$DRY_RUN" -eq 1 ] && BOOT_ARGS+=(--dry-run)
[ "$STRICT" -eq 1 ]  && BOOT_ARGS+=(--strict)
[ "$WITH_SERVICES" -eq 1 ] && BOOT_ARGS+=(--with-services)
# WITH_DASH=1 时去掉 --no-browser 以便打开浏览器（或前台挂起模式；POSIX 下一般不自动开浏览器）
if [ "$WITH_DASH" -eq 1 ]; then
  # 保持原来的 --no-browser（POSIX 图形环境差异大，不开浏览器）
  :
fi

echo "   → $ $PY_BIN scripts/server-manage.py ${BOOT_ARGS[*]}"
"$PY_BIN" scripts/server-manage.py "${BOOT_ARGS[@]}"
rc=$?

if [ "$rc" -ne 0 ]; then
  echo "${cross} bootstrap 失败 exit=$rc" >&2
  if [ "$DRY_RUN" -eq 0 ]; then
    "$PY_BIN" scripts/server-manage.py status
    dump_logs
  fi
  exit "$rc"
fi

# 打印访问地址（端口一律以 platform_config.json 为单一事实源，禁止硬编码漂移）
API_PORT=$("$PY_BIN" -c "import json; d=json.load(open('platform_config.json',encoding='utf-8')); print(d['services']['api']['port'])" 2>/dev/null || echo 8080)
FE_PORT=$("$PY_BIN" -c "import json; d=json.load(open('platform_config.json',encoding='utf-8')); print(d['services']['frontend']['port'])" 2>/dev/null || echo 3020)
DASH_PORT=$("$PY_BIN" -c "import json,sys; d=json.load(open('platform_config.json',encoding='utf-8')); print(d.get('dashboard_port',3999))" 2>/dev/null || echo 3999)
echo
echo "============================================================"
echo "  $tick 完成（管理面板挂起 / 后台运行）"
echo "   · Dashboard: ${C_BLU}http://localhost:${DASH_PORT}/${C_RST}   → 登录 admin（密码见 platform_config.json → admin.password）后点 ▶ 启动所有"
echo "   · API      : ${C_BLU}http://localhost:${API_PORT}/health${C_RST}   （需先在管理面板启动 api）"
echo "   · Frontend : ${C_BLU}http://localhost:${FE_PORT}/${C_RST}         （需先在管理面板启动 frontend）"
echo "   · 停止所有 ：${C_CYN}$0 --stop  或  $PY_BIN scripts/server-manage.py stop all --force${C_RST}"
echo "   · 运维 CLI ：${C_CYN}$PY_BIN scripts/server-manage.py list|status|logs|stop${C_RST}"
echo "   · 旧行为启动（脚本同步启动服务）：${C_CYN}$0 --with-services${C_RST}"
echo "   · 仍在后台运行以下服务（如需停止：$0 --stop）"
echo "============================================================"
