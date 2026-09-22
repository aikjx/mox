#!/usr/bin/env python3
"""Rename files: read content, write to new path, delete old file."""
import os
import shutil

BASE = r'd:\a10\aikjx\gitcode\infotopograph\docs'
OLD = 'mox 模块化系统架构'
NEW = 'MOX'
LOG = []

for root, dirs, files in os.walk(BASE):
    rel = os.path.relpath(root, BASE)
    if rel.startswith('_archive'):
        continue
    for f in files:
        if OLD not in f:
            continue
        old_path = os.path.join(root, f)
        new_name = f.replace(OLD, NEW)
        new_path = os.path.join(root, new_name)
        if os.path.exists(new_path):
            LOG.append(f'SKIP (exists): {os.path.relpath(old_path, BASE)}')
            continue
        try:
            shutil.move(old_path, new_path)
            LOG.append(f'OK: {os.path.relpath(old_path, BASE)} -> {new_name}')
        except Exception as e:
            LOG.append(f'FAIL: {os.path.relpath(old_path, BASE)} -> {new_name}: {e}')

log_path = os.path.join(BASE, '..', 'scripts', 'rename-result.txt')
with open(log_path, 'w', encoding='utf-8') as fh:
    fh.write('\n'.join(LOG))
    fh.write(f'\n\nTotal: {len(LOG)} entries')
