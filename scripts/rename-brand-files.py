#!/usr/bin/env python3
"""Rename files with brand string pollution and update all references."""
import os

BASE = r'd:\a10\aikjx\gitcode\infotopograph\docs'
OLD = 'mox 模块化系统架构'
NEW = 'MOX'
LOG_FILE = os.path.join(BASE, '..', 'scripts', 'rename-log.txt')

results = []

# Step 1: Find and rename files
rename_map = {}  # old_basename -> new_basename
for root, dirs, files in os.walk(BASE):
    rel = os.path.relpath(root, BASE)
    if rel.startswith('_archive'):
        continue
    for f in files:
        if OLD in f:
            old_path = os.path.join(root, f)
            new_name = f.replace(OLD, NEW)
            new_path = os.path.join(root, new_name)
            if os.path.exists(new_path):
                results.append(f'[SKIP] {os.path.relpath(old_path, BASE)} -> {new_name} (target exists)')
                continue
            os.rename(old_path, new_path)
            rename_map[f] = new_name
            results.append(f'[RENAME] {os.path.relpath(old_path, BASE)} -> {new_name}')

results.append(f'\nRenamed {len(rename_map)} files.')

# Step 2: Update references in all docs files (including _archive)
ref_count = 0
for root, dirs, files in os.walk(BASE):
    for f in files:
        fp = os.path.join(root, f)
        try:
            with open(fp, 'r', encoding='utf-8') as fh:
                content = fh.read()
        except:
            continue
        modified = content
        for old_name, new_name in rename_map.items():
            if old_name in modified:
                modified = modified.replace(old_name, new_name)
        if modified != content:
            with open(fp, 'w', encoding='utf-8') as fh:
                fh.write(modified)
            ref_count += 1
            results.append(f'[REF-UPDATE] {os.path.relpath(fp, BASE)}')

results.append(f'\nUpdated references in {ref_count} files.')

with open(LOG_FILE, 'w', encoding='utf-8') as f:
    f.write('\n'.join(results))
    f.write('\n\nDone!\n')

print(f'Done! Renamed {len(rename_map)} files, updated refs in {ref_count} files. See scripts/rename-log.txt')
