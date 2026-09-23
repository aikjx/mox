#!/bin/bash
# verify round 2: alliance tests + clippy clean + live HTTP demo
export PATH="/c/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64:/c/Program Files (x86)/Windows Kits/10/bin/10.0.19041.0/x64:$PATH"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64/link.exe"
export LIB="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/lib/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/um/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/ucrt/x64"
export INCLUDE="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/include;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/ucrt;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/um;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/shared"
export CARGO_TARGET_DIR="D:/cargo-target"
export CARGO_INCREMENTAL=0
cd "D:/a10/aikjx/gitcode/infotopograph"
echo "=== verify-ports WARN lines ==="
python scripts/gate/verify-ports.py 2>&1 | grep -i "warn" | head -10
echo "=== alliance tests (config-core + scheduler-core matching) ==="
cargo test -p mox-alliance-config-core -p mox-alliance-scheduler-core 2>&1 | grep -E "test result|FAILED|panicked" | head -20
echo "=== clippy codeengine (expect clean) ==="
cargo clippy -p mox-codeengine-core --all-targets --message-format short 2>&1 | grep -E "warning|error" | head -5
echo "=== build svc binary ==="
cargo build -p mox-codeengine-svc --message-format short 2>&1 | tail -3
echo "=== live HTTP demo ==="
D:/cargo-target/debug/codeengine-server.exe &
SRV=$!
sleep 3
curl -s http://127.0.0.1:3210/api/codeengine/health
echo
curl -s -X POST http://127.0.0.1:3210/api/codeengine/run -H "Content-Type: application/json" -d '{"text":"读取 input.csv 文件；把解析结果写入 db.orders 表；汇总计算 total_amount 字段并导出报表 output.xlsx","with_bundle":false}' | python -c "import json,sys; d=json.load(sys.stdin); print('delivered=',d['delivered'],'approved=',d['approved'],'vetoed=',d['vetoed'],'score=',round(d['verdict_score'],3)); print('gates=',d['gates']); print('stages=',[e['stage'] for e in d['events']])"
curl -s -X POST http://127.0.0.1:3210/api/codeengine/run -H "Content-Type: application/json" -d '{"text":"查询 db.citizen_idcard 表并导出报表 output.xlsx","auto_guard":false,"with_bundle":false}' | python -c "import json,sys; d=json.load(sys.stdin); print('VETO-CASE delivered=',d['delivered'],'vetoed=',d['vetoed'])"
curl -s http://127.0.0.1:3210/api/codeengine/cases | python -c "import json,sys; d=json.load(sys.stdin); print('cases=',d['total'],'delivery_rate=',round(d['delivery_rate'],2))"
kill $SRV 2>/dev/null
echo "=== done ==="
