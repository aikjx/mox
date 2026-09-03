import re

# 1. storage_api.rs: 删除残留的 derive 属性
fpath1 = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-storage-svc\src\storage_api.rs'
with open(fpath1, encoding='utf-8-sig', errors='ignore') as f:
    c1 = f.read()

old1 = '''#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub use crate::storage_engine::Direction;'''
new1 = '''pub use crate::storage_engine::Direction;'''
n1 = c1.count(old1)
print(f'storage_api 残留 derive 匹配: {n1}')
if n1 > 0:
    c1 = c1.replace(old1, new1, 1)
    with open(fpath1, 'w', encoding='utf-8', newline='') as f:
        f.write(c1)
    print('  已修复 storage_api.rs')

# 2. cdc_publisher.rs: 删除残留的 derive 属性
fpath2 = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-storage-svc\src\cdc_publisher.rs'
with open(fpath2, encoding='utf-8-sig', errors='ignore') as f:
    c2 = f.read()

old2 = '''/// CDC 事件类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub use crate::cdc_source::CdcEventType;'''
new2 = '''/// CDC 事件类型（统一从 cdc_source 导出，含 EdgeUpdated）
pub use crate::cdc_source::CdcEventType;'''
n2 = c2.count(old2)
print(f'cdc_publisher 残留 derive 匹配: {n2}')
if n2 > 0:
    c2 = c2.replace(old2, new2, 1)
    with open(fpath2, 'w', encoding='utf-8', newline='') as f:
        f.write(c2)
    print('  已修复 cdc_publisher.rs')

# 3. 检查 cdc_publisher 中的 impl CdcEventType 块
impl_match = re.search(r'impl CdcEventType \{[\s\S]*?\n\}', c2)
if impl_match:
    print(f'\ncdc_publisher 中存在 impl CdcEventType 块 ({len(impl_match.group())} 字节):')
    print(impl_match.group()[:500])
else:
    print('\ncdc_publisher 中无 impl CdcEventType 块')

# 4. 检查 cdc_publisher 中的 match CdcEventType 语句
for i, line in enumerate(c2.split('\n'), 1):
    if 'match' in line and ('event_type' in line or 'CdcEventType' in line or 'evt' in line):
        print(f'  match 语句行 {i}: {line.strip()[:100]}')
