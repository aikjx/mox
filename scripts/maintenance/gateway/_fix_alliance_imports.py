fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance.rs'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Fix 1: Remove ExpertMatcher from scheduler_core import (it's in scheduler_proto)
old = 'use mox_alliance_scheduler_core::{ExpertMatcher, InMemoryTaskRepository, RuleBasedExpertMatcher, TaskRepository};'
new = 'use mox_alliance_scheduler_core::{InMemoryTaskRepository, RuleBasedExpertMatcher, TaskRepository};'
if old in content:
    content = content.replace(old, new, 1)
    print('Removed ExpertMatcher from scheduler_core import')

# Fix 2: Add ExpertMatcher to scheduler_proto import
old_proto = 'use mox_alliance_scheduler_proto::ExpertMatchQuery;'
new_proto = 'use mox_alliance_scheduler_proto::{ExpertMatchQuery, ExpertMatcher};'
if old_proto in content:
    content = content.replace(old_proto, new_proto, 1)
    print('Added ExpertMatcher to scheduler_proto import')

# Fix 3: Add ExpertHealth to common_proto import
old_common = '''use mox_alliance_common_proto::{
    AllianceMode, Expert, ExpertStatus, FusionStrategy, TaskPriority, TaskStatus,
};'''
new_common = '''use mox_alliance_common_proto::{
    AllianceMode, Expert, ExpertHealth, ExpertStatus, FusionStrategy, TaskPriority, TaskStatus,
};'''
if old_common in content:
    content = content.replace(old_common, new_common, 1)
    print('Added ExpertHealth to common_proto import')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print('Done')
