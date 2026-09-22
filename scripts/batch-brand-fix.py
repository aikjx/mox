#!/usr/bin/env python3
"""Batch replace brand string pollution: 'mox 模块化系统架构' -> 'MOX'
Processes all non-_archive directories under docs/."""
import os

BASE = r'd:\a10\aikjx\gitcode\infotopograph\docs'
OLD = 'mox 模块化系统架构'
NEW = 'MOX'
count = 0
files_changed = 0

for root, dirs, files in os.walk(BASE):
    rel = os.path.relpath(root, BASE)
    if rel.startswith('_archive'):
        continue
    for f in files:
        fp = os.path.join(root, f)
        try:
            with open(fp, 'r', encoding='utf-8') as fh:
                content = fh.read()
        except:
            continue
        if OLD in content:
            new_content = content.replace(OLD, NEW)
            with open(fp, 'w', encoding='utf-8') as fh:
                fh.write(new_content)
            n = content.count(OLD)
            count += n
            files_changed += 1
            print(f'  {os.path.relpath(fp, BASE)}: {n} replacements')

print(f'\nTotal: {count} replacements in {files_changed} files')
