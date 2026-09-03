fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-fusion-svc\src\lib.rs'
with open(fpath, encoding='utf-8-sig', errors='ignore') as f:
    content = f.read()

old = '''use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;'''

new = '''use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// 业务模块
pub mod audit_sync;
pub mod cdc_stage;
pub mod graph_projection_bridge;
pub mod graph_writer;
pub mod tag_parser;'''

n = content.count(old)
print(f'匹配: {n}')
if n > 0:
    content = content.replace(old, new, 1)
    with open(fpath, 'w', encoding='utf-8', newline='') as f:
        f.write(content)
    print('已添加模块声明')
else:
    print('未匹配，检查内容')
    # 打印前 500 字符
    print(repr(content[:500]))
