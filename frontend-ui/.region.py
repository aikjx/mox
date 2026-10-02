# -*- coding: utf-8 -*-
import io, sys
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
ROOT = r'D:\a10\aikjx\gitcode\infotopograph'
ORCH = open(ROOT + r'\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_orchestration.rs', encoding='utf-8').read().replace('\r\n','\n')
lines = ORCH.split('\n')
def show(a,b):
    print('----- lines %d-%d -----' % (a,b))
    for i in range(a-1, min(b,len(lines))):
        print('%4d| %s' % (i+1, lines[i]))
show(380,470)
