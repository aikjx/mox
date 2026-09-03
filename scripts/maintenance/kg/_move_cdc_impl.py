import re

# 1. 从 cdc_publisher.rs 中提取 impl CdcEventType 块内容（用于参考），然后删除
fpath2 = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-storage-svc\src\cdc_publisher.rs'
with open(fpath2, encoding='utf-8-sig', errors='ignore') as f:
    c2 = f.read()

# 找到并删除 impl CdcEventType 块
impl_pattern = r'\nimpl CdcEventType \{[\s\S]*?\n\}\n'
match = re.search(impl_pattern, c2)
if match:
    print(f'cdc_publisher impl CdcEventType 块 ({len(match.group())} 字节):')
    print(match.group()[:600])
    c2_new = c2[:match.start()] + '\n' + c2[match.end():]
    with open(fpath2, 'w', encoding='utf-8', newline='') as f:
        f.write(c2_new)
    print('  已从 cdc_publisher 删除 impl CdcEventType 块')
else:
    print('cdc_publisher 中未找到 impl CdcEventType 块')

# 2. 在 cdc_source.rs 中添加 impl CdcEventType 块（含 EdgeUpdated 分支）
fpath1 = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-storage-svc\src\cdc_source.rs'
with open(fpath1, encoding='utf-8-sig', errors='ignore') as f:
    c1 = f.read()

# 检查是否已有 impl CdcEventType
if 'impl CdcEventType' in c1:
    print('cdc_source 中已有 impl CdcEventType，跳过添加')
else:
    # 在 CdcEventType enum 定义之后添加 impl 块
    enum_end = c1.find('''    EdgeDeleted,
}''')
    if enum_end >= 0:
        insert_pos = enum_end + len('''    EdgeDeleted,
}''')
        impl_block = '''

impl CdcEventType {
    /// 返回事件类型的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            CdcEventType::VertexCreated => "VertexCreated",
            CdcEventType::VertexUpdated => "VertexUpdated",
            CdcEventType::VertexDeleted => "VertexDeleted",
            CdcEventType::EdgeCreated => "EdgeCreated",
            CdcEventType::EdgeUpdated => "EdgeUpdated",
            CdcEventType::EdgeDeleted => "EdgeDeleted",
        }
    }

    /// 判断是否为顶点事件
    pub fn is_vertex_event(&self) -> bool {
        matches!(
            self,
            CdcEventType::VertexCreated | CdcEventType::VertexUpdated | CdcEventType::VertexDeleted
        )
    }
}'''
        c1_new = c1[:insert_pos] + impl_block + c1[insert_pos:]
        with open(fpath1, 'w', encoding='utf-8', newline='') as f:
            f.write(c1_new)
        print('  已在 cdc_source 添加 impl CdcEventType 块（含 EdgeUpdated）')
    else:
        print('  未找到 CdcEventType enum 结束位置')
