import os, shutil
BASE = r'd:\a10\aikjx\gitcode\infotopograph\docs'
OLD = 'mox 模块化系统架构'
NEW = 'MOX'
c = 0
for root, dirs, files in os.walk(BASE):
    if os.path.relpath(root, BASE).startswith('_archive'):
        continue
    for f in files:
        if OLD in f:
            old_path = os.path.join(root, f)
            new_path = os.path.join(root, f.replace(OLD, NEW))
            if not os.path.exists(new_path):
                shutil.move(old_path, new_path)
                c += 1
                with open(r'd:\a10\aikjx\gitcode\infotopograph\scripts\rename-done.txt', 'a', encoding='utf-8') as log:
                    log.write(f'OK: {f} -> {f.replace(OLD, NEW)}\n')
with open(r'd:\a10\aikjx\gitcode\infotopograph\scripts\rename-done.txt', 'a', encoding='utf-8') as log:
    log.write(f'Total: {c}\n')
