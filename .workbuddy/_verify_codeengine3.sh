#!/bin/bash
export PATH="/c/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64:/c/Program Files (x86)/Windows Kits/10/bin/10.0.19041.0/x64:$PATH"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64/link.exe"
export LIB="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/lib/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/um/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/ucrt/x64"
export INCLUDE="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/include;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/ucrt;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/um;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/shared"
export CARGO_TARGET_DIR="D:/cargo-target"
export CARGO_INCREMENTAL=0
cd "D:/a10/aikjx/gitcode/infotopograph"
echo "=== alliance config tests ==="
cargo test -p mox-alliance-config-core 2>&1 | grep -E "test result|FAILED" | tail -3
printf '{"text":"读取 input.csv 文件；把解析结果写入 db.orders 表；汇总计算 total_amount 字段并导出报表 output.xlsx","with_bundle":false}' > /tmp/ce_req.json
printf '{"text":"查询 db.citizen_idcard 表并导出报表 output.xlsx","auto_guard":false,"with_bundle":false}' > /tmp/ce_veto.json
D:/cargo-target/debug/codeengine-server.exe &
SRV=$!
sleep 2
echo "=== POST run (happy) ==="
curl -s -w "\nHTTP %{http_code}\n" -X POST http://127.0.0.1:3210/api/codeengine/run -H "Content-Type: application/json" --data-binary @/tmp/ce_req.json | python -c "
import sys,json
raw=sys.stdin.read()
head,_,tail=raw.partition('\nHTTP')
try:
    d=json.loads(head)
    print('delivered=',d['delivered'],'approved=',d['approved'],'score=',round(d['verdict_score'],3))
    print('stages=',[e['stage'] for e in d['events']])
except Exception as e:
    print('RAW:',raw[:500])
"
echo "=== POST run (veto) ==="
curl -s -w "\nHTTP %{http_code}\n" -X POST http://127.0.0.1:3210/api/codeengine/run -H "Content-Type: application/json" --data-binary @/tmp/ce_veto.json | python -c "
import sys,json
raw=sys.stdin.read()
head,_,tail=raw.partition('\nHTTP')
try:
    d=json.loads(head)
    print('delivered=',d['delivered'],'vetoed=',d['vetoed'])
except Exception as e:
    print('RAW:',raw[:500])
"
kill $SRV 2>/dev/null
echo "=== done ==="
