fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-storage-svc\src\lib.rs'
with open(fpath, encoding='utf-8-sig', errors='ignore') as f:
    content = f.read()

old = '''/// CDC 发布者重导出
pub use cdc_publisher::{
    CdcPublisher, CdcEvent, CdcEventType, FlowControlPolicy,
};'''

new = '''/// CDC 发布者重导出
pub use cdc_publisher::{
    CdcPublisher, CdcEvent, CdcEventType, FlowControlPolicy,
};

/// CDC 源重导出
pub use cdc_source::CdcSource;

/// 图谱编解码重导出
pub use graph_codec::PropValue;

/// 存储服务端重导出
pub use storage_server::StorageServer;

/// 存储 API 重导出（LRU 缓存 + 热邻接缓存）
pub use storage_api::{HotNeighborCache, LruCache};'''

n = content.count(old)
print(f'匹配到 {n} 处')
if n > 0:
    content = content.replace(old, new, 1)
    with open(fpath, 'w', encoding='utf-8', newline='') as f:
        f.write(content)
    print('已添加 pub use 导出')
else:
    print('未匹配到，检查内容')
    # 打印 cdc_publisher 附近内容
    idx = content.find('cdc_publisher')
    if idx >= 0:
        print(repr(content[idx-50:idx+200]))
