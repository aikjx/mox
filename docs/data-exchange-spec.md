# MOX 数据交换规范 (MXDEF)

> 版本: 1.0 | 日期: 2026-08-28
> MOX Data Exchange Format — 全维标准化数据导出/导入协议

---

## 一、设计原则

1. **三层分离**：L1内核 / L2业务数据 / L3运行时，导出时按层隔离
2. **内核与数据分离**：L1可跨系统复用，L2按app_key隔离可独立迁移
3. **运行时不导出**：L3由新系统自生，避免脏数据
4. **幂等导入**：基于业务唯一键upsert，重复导入不产生重复
5. **向前兼容**：导入时忽略未知字段，缺失字段用默认值
6. **安全脱敏**：敏感字段自动脱敏，可强制包含（内网迁移）

---

## 二、文件格式

### 2.1 单文件（推荐）

文件名：`mox-export-{app_key}-{YYYYMMDD-HHmmss}.json`

### 2.2 结构

```json
{
  "format": "MXDEF",
  "version": "1.0",
  "meta": {
    "exported_at": "2026-08-28T10:30:00+08:00",
    "source_system": "mox-server",
    "source_version": "1.0.0",
    "app_key": "mox",
    "app_name": "企业平台",
    "layers": {"kernel": true, "business": true, "runtime": false},
    "record_count": {"kernel": 12, "business": 45, "knowledge_graph": 28}
  },
  "kernel": {
    "sql_templates": [],
    "datasources": [],
    "apps": [],
    "roles": [],
    "permissions": [],
    "field_permissions": []
  },
  "business": {
    "products": [], "news": [], "cases": [], "team": [],
    "history": [], "honors": [], "faqs": [], "jobs": [], "messages": []
  },
  "knowledge_graph": {"entities": [], "relations": []},
  "checksum": "sha256:..."
}
```

### 2.3 分文件（大系统）

```
mox-export-{app_key}-{timestamp}/
├── meta.json
├── kernel/{sql_templates,datasources,apps,roles,permissions,field_permissions}.json
├── business/{products,news,cases,team,history,honors,faqs,jobs,messages}.json
└── knowledge_graph/{entities,relations}.json
```

---

## 三、数据标准化

### 3.1 字段命名规范

| 规则 | 示例 |
|------|------|
| 统一 snake_case | `app_key`, `created_at`, `field_permissions` |
| 主键统一 `id` | 整数自增或UUID |
| 业务唯一键 | SQL模板用`code`，应用用`app_key` |
| 时间统一 ISO 8601 | `2026-08-28T10:30:00+08:00` |
| 布尔统一 true/false | 不用0/1 |
| 空值统一 null | 不用空字符串或"null" |

### 3.2 敏感字段处理

| 字段类型 | 处理方式 | 示例 |
|---------|---------|------|
| 密码/密钥 | 不导出，置`__REDACTED__` | `password: "__REDACTED__"` |
| 邮箱 | 保留首尾，中间掩码 | `m***@mox-tech.com` |
| 电话 | 保留前3后4 | `139****8888` |
| 身份证 | 保留前6后4 | `440600********1234` |

`--include-sensitive` 可强制包含原始值（仅限内网安全迁移）。

### 3.3 编码

- 文件编码：UTF-8 **无 BOM**
- JSON标准转义：换行`\n`、引号`\"`、反斜杠`\\`
- 富文本字段保留HTML，不做实体编码

---

## 四、导出命令

```bash
# 导出默认应用(mox)的全部数据
python tools/export_data.py

# 导出指定应用
python tools/export_data.py --app-key corp_demo

# 导出全部应用(忽略app_key过滤)
python tools/export_data.py --all

# 仅导出L1内核(新系统初始化用)
python tools/export_data.py --kernel-only

# 仅导出L2业务数据(应用迁移用)
python tools/export_data.py --app-key corp_demo --business-only

# 包含敏感信息(默认脱敏)
python tools/export_data.py --include-sensitive

# 分文件导出(大系统>1万条)
python tools/export_data.py --split

# gzip压缩
python tools/export_data.py --gzip

# 指定输出目录
python tools/export_data.py --output ./exports
```

### 导出流程

```
1. 连接数据库(自动检测或--db指定)
2. 读取meta(系统版本/应用信息)
3. L1内核: sql_templates/datasources/apps/roles/permissions/field_permissions
4. L2业务: products/news/cases/team/history/honors/faqs/jobs/messages
5. L2图谱: kg_entities/kg_relations
6. 敏感字段脱敏
7. 计算checksum(sha256)
8. 写入JSON文件(或分文件目录/压缩包)
9. 输出导出报告(记录数/大小/耗时)
```

---

## 五、导入命令

```bash
# 幂等导入(基于唯一键upsert)
python tools/import_data.py mox-export-mox-20260828-103000.json

# 仅导入内核
python tools/import_data.py export.json --kernel-only

# 仅导入业务数据
python tools/import_data.py export.json --business-only

# 导入到指定应用(覆盖app_key)
python tools/import_data.py export.json --target-app new_corp

# 预览模式(不写入,只显示将导入的记录数)
python tools/import_data.py export.json --dry-run

# 强制覆盖(忽略checksum校验失败)
python tools/import_data.py export.json --force

# 导入前清空目标表(危险)
python tools/import_data.py export.json --purge
```

### 导入流程

```
1. 读取JSON,验证format/version
2. checksum校验(防篡改)
3. 开启事务
4. 按依赖顺序导入:
   ├─ apps → datasources → sql_templates
   ├─ roles → permissions → field_permissions
   ├─ products/news/cases/team/history/honors/faqs/jobs
   ├─ messages
   └─ kg_entities → kg_relations(重新映射ID)
5. 验证导入记录数与导出一致
6. 提交事务(dry-run回滚)
7. 输出导入报告
```

### 冲突处理

| 场景 | 默认行为 | 可选行为 |
|------|---------|---------|
| 同code SQL模板已存在 | 更新版本+1 | --force直接覆盖 |
| 同app_key应用已存在 | 跳过 | --force覆盖配置 |
| 业务数据ID冲突 | 重新分配ID | --force覆盖原记录 |
| 图谱实体冲突 | 按type+name去重合并 | --force全部新增 |
| 未知字段 | 忽略 | --strict报错终止 |

---

## 六、校验命令

```bash
python tools/validate_export.py mox-export-mox-20260828.json
```

检查项：
- JSON格式合法
- format=MXDEF, version兼容
- checksum匹配
- 每层记录数>0
- meta声明记录数与实际一致
- 外键引用完整(图谱relations的source/target在entities中存在)
- 敏感字段已脱敏
- 无重复ID

---

## 七、跨系统发布场景

### 场景A: 新系统初始化

```bash
# 1. 源系统导出内核+业务
python tools/export_data.py --output ./exports

# 2. 校验
python tools/validate_export.py ./exports/mox-export-mox-*.json

# 3. 目标系统预览导入
python tools/import_data.py ./exports/mox-export-mox-*.json --dry-run

# 4. 正式导入
python tools/import_data.py ./exports/mox-export-mox-*.json
```

### 场景B: 应用迁移(A系统→B系统)

```bash
# A系统导出指定应用的业务数据
python tools/export_data.py --app-key corp_demo --business-only

# B系统导入(自动重新映射ID)
python tools/import_data.py corp_demo-export.json --target-app corp_demo
```

### 场景C: 内核升级(SQL模板/权限同步)

```bash
# 开发环境导出内核
python tools/export_data.py --kernel-only

# 生产环境先预览
python tools/import_data.py kernel-export.json --kernel-only --dry-run

# 确认后导入(SQL模板自动版本+1可回滚)
python tools/import_data.py kernel-export.json --kernel-only
```

---

## 八、多租户数据隔离

| 隔离级别 | 实现方式 | 适用场景 |
|---------|---------|---------|
| 应用级 | 业务表含app_key字段,查询自动过滤 | 多租户SaaS,共享数据库 |
| Schema级 | 每应用独立数据库schema | 高隔离要求客户 |
| 物理级 | 每应用独立数据库实例 | 金融/政务强合规 |

导出时`--app-key`自动过滤该应用所有业务数据；导入时`--target-app`指定目标应用，自动重新映射ID。

---

## 九、版本兼容

| 导出版本 | 导入版本 | 兼容性 |
|---------|---------|--------|
| 1.0 | 1.0 | 完全兼容 |
| 1.0 | 1.x | 向前兼容(忽略新字段) |
| 1.x | 1.0 | 部分兼容(新字段被忽略) |
| 2.0 | 1.0 | 不兼容(需升级导入工具) |

---

## 十、性能参考

| 数据规模 | 单文件JSON | 分文件 | 导入耗时 |
|---------|-----------|--------|---------|
| <1万条 | 推荐 | - | <5秒 |
| 1-10万条 | 可用 | 推荐 | 5-30秒 |
| >10万条 | 不推荐 | 必须 | 30秒+ |

大文件自动启用流式写入，支持NDJSON格式和gzip压缩。
