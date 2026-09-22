#!/usr/bin/env python3
"""Comprehensive brand string pollution fix:
1. Replace 'mox 模块化系统架构' -> 'MOX' in file contents (excluding _archive)
2. Rename files with 'mox 模块化系统架构' in filename
3. Update all references to renamed files
"""
import os
import re

BASE = r'd:\a10\aikjx\gitcode\infotopograph\docs'
OLD = 'mox 模块化系统架构'
NEW = 'MOX'
LOG_FILE = r'd:\a10\aikjx\gitcode\infotopograph\scripts\brand-fix-log.txt'

results = []

# Phase 1: Replace content in all non-_archive files
content_count = 0
content_files = 0
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
            content_count += n
            content_files += 1
            results.append(f'[CONTENT] {os.path.relpath(fp, BASE)}: {n} replacements')

results.append(f'\nPhase 1 total: {content_count} replacements in {content_files} files\n')

# Phase 2: Rename files with OLD brand string in filename
rename_count = 0
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
                results.append(f'[SKIP-RENAME] {os.path.relpath(old_path, BASE)} -> target exists: {new_name}')
                continue
            os.rename(old_path, new_path)
            rename_count += 1
            results.append(f'[RENAME] {os.path.relpath(old_path, BASE)} -> {new_name}')

results.append(f'\nPhase 2 total: {rename_count} files renamed\n')

# Phase 3: Update references to renamed files in all docs (including _archive)
ref_count = 0
# Build mapping of old filenames to new filenames
rename_map = {}
for r in results:
    if r.startswith('[RENAME]'):
        # Extract old and new names
        parts = r.replace('[RENAME] ', '')
        if ' -> ' in parts:
            old_name = parts.split(' -> ')[0].split('\\')[-1]
            new_name = parts.split(' -> ')[1]
            rename_map[old_name] = new_name

if rename_map:
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

results.append(f'\nPhase 3 total: {ref_count} files had references updated')

# Write log
with open(LOG_FILE, 'w', encoding='utf-8') as f:
    f.write('\n'.join(results))
    f.write('\n\nDone!\n')

print('Done! See scripts/brand-fix-log.txt for details.')
