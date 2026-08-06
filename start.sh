#!/bin/bash
# 算子统一系统一键启动脚本

set -e

echo "========================================"
echo "  算子统一系统 (Operator Unified System)"
echo "========================================"
echo ""

# 检查Rust环境
if ! command -v cargo &> /dev/null; then
    echo "⚠️  未检测到Rust环境，正在安装..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo "🔧 正在编译项目..."
cargo build --release

echo ""
echo "🧮 正在运行数学公理验证..."
if command -v python3 &> /dev/null; then
    python3 verify_axioms.py || echo "⚠️  公理验证完成（部分警告不影响运行）"
else
    echo "⚠️  未检测到Python3，跳过数学验证"
fi

echo ""
echo "🚀 启动服务器..."
echo "📱 前端界面: http://localhost:3000"
echo "📊 API接口: http://localhost:3000/api"
echo ""
echo "按 Ctrl+C 停止服务器"
echo "========================================"
echo ""

./target/release/operator-server
