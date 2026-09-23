#!/bin/bash
export PATH="/c/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64:/c/Program Files (x86)/Windows Kits/10/bin/10.0.19041.0/x64:$PATH"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64/link.exe"
export LIB="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/lib/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/um/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/ucrt/x64"
export INCLUDE="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/include;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/ucrt;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/um;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/shared"
export CARGO_TARGET_DIR="D:/cargo-target"
export CARGO_INCREMENTAL=0
cd "D:/a10/aikjx/gitcode/infotopograph" || exit 1

echo "=== build svc binary ==="
cargo build -p mox-codeengine-svc 2>&1 | tail -1
EXE="D:/cargo-target/debug/codeengine-server.exe"
"$EXE" > .workbuddy/_v11_server.log 2>&1 &
SRV=$!
sleep 2

echo "=== health (capabilities + Refine stage) ==="
curl -s http://127.0.0.1:3210/api/codeengine/health

echo
echo "=== run#1: self-repair loop (auto_guard=false) ==="
printf '{"text":"查询 db.citizen_idcard 表并导出报表 output.xlsx","auto_guard":false,"with_bundle":true,"id":"demo-repair"}' > .workbuddy/_v11_r1.json
curl -s -X POST http://127.0.0.1:3210/api/codeengine/run -H "Content-Type: application/json" --data-binary @.workbuddy/_v11_r1.json -o .workbuddy/_v11_resp1.json

echo "=== run#2: diff-first with known_files + patch roundtrip ==="
python - <<'PYEOF'
import json, io

r1 = json.load(io.open('.workbuddy/_v11_resp1.json', encoding='utf-8'))
print('run#1 rounds=%s repairs=%s delivered=%s vetoed=%s' % (r1['rounds'], r1['repairs'], r1['delivered'], r1['vetoed']))
print('run#1 stages:', ' '.join(e['stage'] for e in r1['events']))

pys = [f for f in r1['files'] if f['path'].endswith('.py')]
tasks = [f for f in pys if 'def ' in f['content']][0]
init = [f for f in pys if f['path'] != tasks['path']][0]
known = {
    init['path']: init['content'],                          # 无差异基线 → 空块 no_change
    tasks['path']: tasks['content'].replace('def ', 'def legacy_', 1),  # 过期基线 → 实际补丁
}
body = {
    'text': '查询 db.citizen_idcard 表并导出报表 output.xlsx',
    'id': 'demo-patch',
    'with_bundle': True,
    'known_files': known,
}
io.open('.workbuddy/_v11_r2.json', 'w', encoding='utf-8').write(json.dumps(body, ensure_ascii=False))
PYEOF
curl -s -X POST http://127.0.0.1:3210/api/codeengine/run -H "Content-Type: application/json" --data-binary @.workbuddy/_v11_r2.json -o .workbuddy/_v11_resp2.json
python - <<'PYEOF'
import json, io

r2 = json.load(io.open('.workbuddy/_v11_resp2.json', encoding='utf-8'))
print('run#2 delivered=%s patches=%d' % (r2['delivered'], len(r2['patches'])))
for blk in r2['patches']:
    print('patch path=%s search_lines=%d replace_lines=%d' % (blk['path'], blk['search'].count('\n'), blk['replace'].count('\n')))
r1 = json.load(io.open('.workbuddy/_v11_resp1.json', encoding='utf-8'))
gen = {f['path']: f['content'] for f in r2['files']}
patched = {blk['path'] for blk in r2['patches']}
base = {b['path']: (b['content'].replace('def ', 'def legacy_', 1) if 'def ' in b['content'] else b['content'])
        for b in r1['files'] if b['path'] in patched}
pb = {'base': base, 'blocks': r2['patches']}
io.open('.workbuddy/_v11_p1.json', 'w', encoding='utf-8').write(json.dumps(pb, ensure_ascii=False))
io.open('.workbuddy/_v11_expected.json', 'w', encoding='utf-8').write(json.dumps(gen))
PYEOF
curl -s -X POST http://127.0.0.1:3210/api/codeengine/patch -H "Content-Type: application/json" --data-binary @.workbuddy/_v11_p1.json -o .workbuddy/_v11_resp3.json
python - <<'PYEOF'
import json, io

pr = json.load(io.open('.workbuddy/_v11_resp3.json', encoding='utf-8'))
gen = json.load(io.open('.workbuddy/_v11_expected.json', encoding='utf-8'))
print('patch ok=%s applied=%s failed=%s statuses=%s' % (pr['ok'], pr['applied'], pr['failed'], [x['status'] for x in pr['results']]))
print('roundtrip restores generated content:', all(pr['files'][p] == gen[p] for p in pr['files']))

text_demo = {'base': {'x.py': 'a\nb\nc\n'}, 'blocks_text': '<<<<<<< SEARCH path=x.py\nb\n=======\nB2\n>>>>>>> REPLACE'}
io.open('.workbuddy/_v11_p2.json', 'w', encoding='utf-8').write(json.dumps(text_demo))
PYEOF
curl -s -X POST http://127.0.0.1:3210/api/codeengine/patch -H "Content-Type: application/json" --data-binary @.workbuddy/_v11_p2.json -o .workbuddy/_v11_resp4.json
python -c "import json;d=json.load(open('.workbuddy/_v11_resp4.json'));print('text-block patch ok=%s files=%s'%(d['ok'],d['files']))"

echo "=== cases (Learn) ==="
curl -s http://127.0.0.1:3210/api/codeengine/cases | python -c "import json,sys;d=json.load(sys.stdin);print('total=%s delivery_rate=%s avg_rounds=%s'%(d['total'],d['delivery_rate'],d['avg_rounds_to_converge']))"

kill $SRV 2>/dev/null
echo "=== done ==="
