import re

# 1. 修复 storage_api.rs: 删除 Direction 定义，改为 re-export
fpath1 = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-storage-svc\src\storage_api.rs'
with open(fpath1, encoding='utf-8-sig', errors='ignore') as f:
    content1 = f.read()

old_dir = '''pub enum Direction {
    Out,
    In,
    Both,
}'''
new_dir = '''pub use crate::storage_engine::Direction;'''

n1 = content1.count(old_dir)
print(f'storage_api Direction 定义匹配: {n1} 处')
if n1 > 0:
    content1 = content1.replace(old_dir, new_dir, 1)
    with open(fpath1, 'w', encoding='utf-8', newline='') as f:
        f.write(content1)
    print('  storage_api.rs 已修复')
else:
    print('  未匹配到 Direction 定义')

# 2. 修复 cdc_publisher.rs: 删除 CdcEventType 定义，改为 re-export
fpath2 = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-storage-svc\src\cdc_publisher.rs'
with open(fpath2, encoding='utf-8-sig', errors='ignore') as f:
    content2 = f.read()

old_cdc = '''pub enum CdcEventType {
    /// 顶点创建
    VertexCreated,
    /// 顶点更新
    VertexUpdated,
    /// 顶点删除
    VertexDeleted,
    /// 边创建
    EdgeCreated,
    /// 边删除
    EdgeDeleted,
}'''
new_cdc = '''pub use crate::cdc_source::CdcEventType;'''

n2 = content2.count(old_cdc)
print(f'cdc_publisher CdcEventType 定义匹配: {n2} 处')
if n2 > 0:
    content2 = content2.replace(old_cdc, new_cdc, 1)
    with open(fpath2, 'w', encoding='utf-8', newline='') as f:
        f.write(content2)
    print('  cdc_publisher.rs 已修复')
else:
    print('  未匹配到 CdcEventType 定义')
    # 尝试找 cdc_publisher 中的 CdcEventType
    for i, line in enumerate(content2.split('\n'), 1):
        if 'CdcEventType' in line and 'enum' in line:
            print(f'  行{i}: {repr(line)}')
