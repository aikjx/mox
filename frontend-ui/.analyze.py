# -*- coding: utf-8 -*-
import io, sys, re
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

ROOT = r'D:\a10\aikjx\gitcode\infotopograph'
ORCH = open(ROOT + r'\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_orchestration.rs', encoding='utf-8').read().replace('\r\n','\n')
COMMON = open(ROOT + r'\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_common.rs', encoding='utf-8').read().replace('\r\n','\n')
DB = open(ROOT + r'\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_db.rs', encoding='utf-8').read().replace('\r\n','\n')
lines = ORCH.split('\n')

def own_level(text):
    out=''
    depth=0
    for ch in text:
        if ch in '{[': depth+=1
        elif ch in '}]': depth-=1
        if depth==0: out+=ch
        else: out+=' '
    return out

def json_keys(block):
    return re.findall(r'"([a-z_]+)"\s*:', block)

def json_blocks(src):
    out=[]
    for m in re.finditer(r'json!\s*\(\s*\{', src):
        open_idx = src.index('{', m.start())
        depth=0; i=open_idx
        while i < len(src):
            if src[i]=='{': depth+=1
            elif src[i]=='}':
                depth-=1
                if depth==0: break
            i+=1
        out.append(json_keys(own_level(src[open_idx+1:i])))
    return out

blocks = json_blocks(ORCH)
print('=== ALL json! blocks top-level key sets (count=%d) ===' % len(blocks))
from collections import Counter
for keys,cnt in Counter(tuple(b) for b in blocks).items():
    print('  x%d : %s' % (cnt, list(keys)))

print()
print('=== blocks exactly [id,name,title] (experts summary) ===')
for idx,b in enumerate(blocks):
    if b==['id','name','title']:
        print('  block#', idx)

print()
print('=== inline "execution": { ... } wrapper ===')
for m in re.finditer(r'"execution":\s*\{', ORCH):
    seg = ORCH[m.start():m.start()+400]
    print('  --- at char', m.start(), 'line', ORCH[:m.start()].count('\n')+1)
    print('   ', seg[:300].replace('\n','\\n'))

print()
print('=== inline "plan": { ... } wrapper ===')
for m in re.finditer(r'"plan":\s*\{', ORCH):
    seg = ORCH[m.start():m.start()+300]
    print('  --- line', ORCH[:m.start()].count('\n')+1)
    print('   ', seg[:250].replace('\n','\\n'))

print()
print('=== plan.status = "X".to_string() assignments ===')
print('  ', re.findall(r'plan\.status = "(\w+)"\.to_string\(\)', ORCH))
for m in re.finditer(r'plan\.status = ', ORCH):
    ln = ORCH[:m.start()].count('\n')+1
    print('   line', ln, ':', lines[ln-1].strip())

print()
print('=== status: "X".to_string() occurrences ===')
for m in re.finditer(r'status:\s*"\w+"\.to_string\(\)', ORCH):
    ln = ORCH[:m.start()].count('\n')+1
    print('   line', ln, ':', lines[ln-1].strip())

print()
print('=== "failed" string literals lines ===')
for m in re.finditer(r'"failed"', ORCH):
    ln = ORCH[:m.start()].count('\n')+1
    print('   line', ln, ':', lines[ln-1].strip()[:90])
