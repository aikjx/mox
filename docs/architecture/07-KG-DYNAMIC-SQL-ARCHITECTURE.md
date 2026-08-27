# KG驱动的动态SQL配置平台 · 全维架构设计

> **版本**: v1.0  
> **日期**: 2026-08-27  
> **架构师**: 开发专家联盟 · 璇玑 RelGraph  
> **状态**: 设计定稿  
> **关联模块**: mox-platform-dsql-core (拟建) · mox-platform-datastore-core · mox-kg-hub-svc · mox-kg-algo-core · mox-platform-iam-core

---

## 目录

1. [架构概述](#1-架构概述)
2. [七层架构模型](#2-七层架构模型)
3. [知识图谱建模](#3-知识图谱建模)
4. [字段级权限控制体系](#4-字段级权限控制体系)
5. [自定义权限配置引擎](#5-自定义权限配置引擎)
6. [业务处理流程](#6-业务处理流程)
7. [核心数据结构设计](#7-核心数据结构设计)
8. [与MOX现有架构集成](#8-与mox现有架构集成)
9. [关键技术选型](#9-关键技术选型)
10. [可行性全维评估](#10-可行性全维评估)
11. [实施路线图](#11-实施路线图)

---

## 1. 架构概述

### 1.1 核心思想

**知识图谱是「大脑」，SQL是「肌肉」，数据库是「骨骼」，权限是「免疫系统」。**

传统动态SQL平台仅管理SQL文本，缺乏对SQL关系、数据血缘、影响范围、字段级权限的认知。引入知识图谱后，平台从「SQL文本管理器」升级为「数据操作智能中枢」：

- SQL不再是孤立文本，而是图谱中的**节点**，与表、字段、参数、权限、业务实体、上下游SQL形成**关系网络**
- 修改一个SQL，可瞬间分析**影响范围**（哪些下游SQL/报表/接口/权限策略受影响）
- 新业务需求可通过**图谱推理**推荐复用已有SQL，或自动生成SQL草稿
- **字段级权限**通过图谱中 `:Column` 节点的 `sensitive_level` 属性 + `HAS_FIELD_PERM` 关系实现，执行时自动脱敏
- **自定义权限策略**支持条件表达式（行级过滤）、字段级脱敏、动态权限计算

### 1.2 设计目标

| 目标 | 指标 |
|------|------|
| SQL配置化率 | >95% 业务查询无需硬编码 |
| 新SQL上线时间 | 从「天级」降至「分钟级」 |
| 字段级权限覆盖 | 100% 敏感字段受控 |
| 权限策略配置 | 可视化拖拽 + 表达式，零代码 |
| 缓存命中率 | >80%（多级缓存 + KG驱动预热） |
| 多数据库支持 | MySQL/PG/Oracle/SQLServer/ClickHouse/SQLite |
| 影响分析响应 | <100ms（图谱遍历） |

---

## 2. 七层架构模型

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  L7 智能层    KG推理引擎：SQL推荐 / 影响分析 / 血缘追踪 / 自动生成草稿        │
│              基于图算法(PageRank/最短路径/子图匹配) + LLM增强                 │
│              权限智能推荐：基于角色/部门/历史行为推荐字段权限策略              │
├──────────────────────────────────────────────────────────────────────────────┤
│  L6 配置层    可视化配置画布：SQL编辑器 + 图谱拖拽 + 参数表单 + 测试面板       │
│              权限策略编辑器：字段权限矩阵 + 行级过滤表达式 + 脱敏规则          │
│              版本对比 / 灰度发布 / 审批流 / 操作审计                           │
├──────────────────────────────────────────────────────────────────────────────┤
│  L5 编排层    业务处理Pipeline：接收→KG元数据解析→权限鉴权(字段级)→缓存判断  │
│              →SQL渲染→执行→结果脱敏→缓存回填→审计→事件发布                    │
│              (复用MOX Orchestrator 10阶段Pipeline)                            │
├──────────────────────────────────────────────────────────────────────────────┤
│  L4 缓存层    L1本地Caffeine(moka) → L2 Redis → L3空值缓存                   │
│              缓存键 = KG版本哈希 + SQL版本哈希 + 参数哈希 + 权限指纹            │
│              KG变更/权限变更触发缓存级联失效                                    │
├──────────────────────────────────────────────────────────────────────────────┤
│  L3 执行层    动态SQL执行引擎：连接池路由→预编译→执行→结果映射→慢SQL采集       │
│              字段级脱敏引擎：执行后按权限策略脱敏                               │
│              支持事务/批量/流式/分页/读写分离                                  │
├──────────────────────────────────────────────────────────────────────────────┤
│  L2 适配层    DatabaseAdapter SPI：MySQL/PG/Oracle/SQLServer/ClickHouse       │
│              方言翻译 + 分页包装 + 类型映射 + 连接池多路复用                    │
├──────────────────────────────────────────────────────────────────────────────┤
│  L1 元数据层  ┌──────────────────┐  ┌───────────────────────────────────────┐ │
│              │  关系型存储        │  │  知识图谱存储 (Neo4j)                 │ │
│              │  dsql_definition   │  │  节点：SQL/Table/Column/Param/        │ │
│              │  dsql_datasource   │  │        Entity/Permission/FieldPolicy  │ │
│              │  dsql_cache_rule   │  │  关系：USES_TABLE/USES_COLUMN/        │ │
│              │  dsql_audit_log    │  │        DEPENDS_ON/HAS_FIELD_PERM/     │ │
│              │  dsql_version      │  │        OWNS/REQUIRES_PERM/CALLS       │ │
│              │  dsql_field_policy │  │  图算法：影响分析/血缘/推荐/聚类       │ │
│              └──────────────────┘  └───────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. 知识图谱建模

### 3.1 节点类型（8类核心实体）

| 节点类型 | 标签 | 关键属性 | 说明 |
|----------|------|----------|------|
| **SQL节点** | `:SQL` | sql_code, name, template, version, status, result_type, operation_type(READ/WRITE) | 可执行的动态SQL定义 |
| **表节点** | `:Table` | table_name, datasource, schema, row_count, description, classification(公开/内部/机密) | 物理表或视图 |
| **字段节点** | `:Column` | column_name, data_type, is_pk, is_fk, sensitive_level(0-4), pii_type, description | 表字段，含敏感等级和PII类型 |
| **参数节点** | `:Param` | param_name, data_type, required, default_value, description, validation_rule | SQL入参定义 |
| **业务实体节点** | `:Entity` | entity_code, name, domain, industry_package | MOX业务实体（订单/用户/合同等） |
| **权限策略节点** | `:FieldPolicy` | policy_code, name, policy_type(WHITELIST/BLACKLIST/MASK/CONDITION), expression, mask_function, priority | 字段级权限策略 |
| **权限点节点** | `:Permission` | perm_code, name, perm_type(ENTITY/FIELD/ROW), description | IAM权限点 |
| **业务流程节点** | `:BizFlow` | flow_code, name, trigger_type | 触发SQL执行的业务流程 |

### 3.2 字段敏感等级定义

| 等级 | 名称 | 说明 | 默认处理 | 示例 |
|------|------|------|----------|------|
| L0 | 公开 | 可对外公开 | 无限制 | 商品名称、公开分类 |
| L1 | 内部 | 内部员工可见 | 登录可查 | 订单状态、部门名称 |
| L2 | 机密 | 需特定权限 | 权限校验 | 客户名称、合同金额 |
| L3 | 高敏 | 严格管控+脱敏 | 默认脱敏+审批 | 手机号、身份证号、银行卡 |
| L4 | 绝密 | 最高管控 | 禁止明文+全程审计 | 密码哈希、密钥、生物特征 |

### 3.3 关系类型（12类核心关系）

| 关系 | 方向 | 语义 | 图算法应用 |
|------|------|------|------------|
| `USES_TABLE` | SQL→Table | SQL读取/写入哪些表 | 影响分析、数据源路由 |
| `USES_COLUMN` | SQL→Column | SQL引用哪些字段 | **列级血缘、敏感字段自动识别、权限校验** |
| `DEPENDS_ON` | SQL→SQL | SQL_B的输入依赖SQL_A的输出 | 执行依赖排序、级联失效 |
| `OUTPUTS_ENTITY` | SQL→Entity | SQL的结果映射到哪个业务实体 | 实体-SQL反向查询 |
| `HAS_PARAM` | SQL→Param | SQL有哪些入参 | 参数校验、表单自动生成 |
| `REQUIRES_PERM` | SQL→Permission | 执行需要什么权限 | 鉴权、权限影响分析 |
| `HAS_FIELD_PERM` | Permission→Column | 权限点对哪些字段有什么权限 | **字段级权限矩阵、权限继承分析** |
| `APPLIES_POLICY` | FieldPolicy→Column | 策略应用于哪些字段 | **脱敏规则匹配、策略冲突检测** |
| `OWNS_SQL` | Entity→SQL | 业务实体拥有哪些SQL | 实体级SQL管理、行业包打包 |
| `TRIGGERS` | BizFlow→SQL | 哪个业务流程触发哪些SQL | 流程-SQL映射、影响评估 |
| `CALLS` | SQL→SQL | 存储过程/函数调用关系 | 调用链分析、性能瓶颈定位 |
| `JOINS_WITH` | Table→Table | 表之间的关联关系（权重=使用次数） | 自动JOIN推荐、联表SQL生成 |

### 3.4 图谱核心能力

#### ① 影响分析（Impact Analysis）

```
修改SQL_A → 遍历 DEPENDS_ON 关系 → 找到所有下游SQL
→ 遍历 OUTPUTS_ENTITY → 找到受影响的业务实体
→ 遍历 TRIGGERS 反向 → 找到受影响的业务流程
→ 遍历 USES_COLUMN → 找到涉及的敏感字段 → 关联权限策略 → 评估权限影响
→ 输出影响报告：影响X个SQL、Y个实体、Z个流程、W个权限策略
```

#### ② 数据血缘（Data Lineage）

```
字段级血缘：Column_A → USES_COLUMN ← SQL_X → OUTPUTS_ENTITY → Entity_Y
→ 可追溯：订单金额字段从哪些表、经过哪些SQL、最终出现在哪些报表
→ 合规审计：GDPR/等保要求的数据流追踪自动化
→ 权限血缘：某字段被哪些SQL读取 → 哪些用户通过这些SQL接触到该字段
```

#### ③ 敏感字段自动识别

```
新SQL模板保存 → 解析SQL中的字段引用 → 匹配 :Column 节点
→ 读取 sensitive_level 属性 → 自动标记该SQL涉及的敏感等级
→ 自动关联对应的 FieldPolicy → 执行时自动应用脱敏规则
→ 配置者无需手动指定哪些字段需要脱敏，图谱自动完成
```

#### ④ SQL推荐（SQL Recommendation）

```
新需求：查询近30天高价值客户订单
→ 图谱中匹配 Entity=订单 + Entity=客户 + 时间范围参数
→ 按 PageRank 排序已有SQL（被引用多的优先）
→ 过滤当前用户有权限的SQL（通过 REQUIRES_PERM + HAS_FIELD_PERM）
→ 推荐TOP5可复用SQL，或基于相似SQL模板生成草稿
```

---

## 4. 字段级权限控制体系

### 4.1 权限模型总览

```
┌─────────────────────────────────────────────────────────────────┐
│                     字段级权限控制三层模型                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  L1 实体级权限    谁能访问哪个业务实体（订单/客户/合同）          │
│                   REQUIRES_PERM 关系 → IAM鉴权                   │
│                                                                   │
│  L2 字段级权限    谁能看到/修改实体的哪些字段                     │
│                   HAS_FIELD_PERM 关系 → 字段权限矩阵              │
│                   Column.sensitive_level → 自动脱敏               │
│                                                                   │
│  L3 行级权限      谁能看到哪些数据行（条件过滤）                  │
│                   FieldPolicy.policy_type=CONDITION               │
│                   表达式动态注入WHERE子句                          │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 字段权限矩阵

字段级权限通过图谱中 `HAS_FIELD_PERM` 关系实现，关系属性定义权限类型：

```cypher
(:Permission)-[HAS_FIELD_PERM {
    perm_type: "READ" | "WRITE" | "READ_MASKED" | "READ_APPROVAL",
    mask_function: "MASK_PHONE" | "MASK_ID_CARD" | "MASK_EMAIL" | "MASK_AMOUNT" | "CUSTOM",
    custom_mask: "LEFT(3,2)||'****'||RIGHT(2)",
    approval_required: false,
    granted_by: "admin",
    granted_at: "2026-08-27T10:00:00Z"
}]->(:Column)
```

**权限类型说明：**

| 权限类型 | 码 | 说明 | 示例 |
|----------|-----|------|------|
| 可读明文 | `READ` | 可查看字段原始值 | 管理员可看客户手机号明文 |
| 可读脱敏 | `READ_MASKED` | 可查看但自动脱敏 | 客服可看手机号前3后4 |
| 可写 | `WRITE` | 可修改该字段值 | 财务可修改合同金额 |
| 审批可读 | `READ_APPROVAL` | 需审批后临时查看明文 | 审计员查看身份证需审批 |
| 无权限 | — | 字段不返回（或返回null） | 普通员工看不到薪资字段 |

### 4.3 脱敏函数库

| 函数名 | 适用字段 | 效果 | 示例输入 → 输出 |
|--------|----------|------|-----------------|
| `MASK_PHONE` | 手机号 | 前3后4，中间* | `13812345678` → `138****5678` |
| `MASK_ID_CARD` | 身份证 | 前6后4，中间* | `440101199001011234` → `440101**********1234` |
| `MASK_EMAIL` | 邮箱 | 用户名首字符+***+域名 | `zhangsan@example.com` → `z***@example.com` |
| `MASK_AMOUNT` | 金额 | 取整到千位+*** | `12345.67` → `12000***` |
| `MASK_NAME` | 姓名 | 姓+*（双字名保留姓） | `张三` → `张*`，`欧阳峰` → `欧**` |
| `MASK_BANK_CARD` | 银行卡 | 前6后4，中间* | `6222021234567890123` → `622202*********0123` |
| `MASK_ADDRESS` | 地址 | 保留到区/县，后续* | `广州市天河区珠江新城XX路1号` → `广州市天河区***` |
| `CUSTOM` | 任意 | 自定义SQL表达式 | `LEFT(name,1)||'***'` |

### 4.4 执行时字段权限校验流程

```
SQL执行请求 (user_id + sql_code + params)
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 1: KG元数据加载                                          │
│   加载 :SQL 节点 → 遍历 USES_COLUMN → 获取所有涉及的字段列表  │
│   加载每个 :Column 节点的 sensitive_level 属性                 │
│   加载用户角色 → 遍历 HAS_FIELD_PERM → 获取用户对每个字段的权限│
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 2: 权限决策                                              │
│   对每个涉及字段，决策结果：                                   │
│   ┌──────────────┬──────────────────────────────────────┐    │
│   │ 权限类型      │ 处理方式                               │    │
│   ├──────────────┼──────────────────────────────────────┤    │
│   │ READ          │ 保留原始值                             │    │
│   │ READ_MASKED   │ 执行后脱敏（保留字段，值替换）         │    │
│   │ READ_APPROVAL │ 检查审批状态→未审批则脱敏+标记待审批   │    │
│   │ WRITE(写操作)  │ 校验写权限→无权限则拒绝整个请求        │    │
│   │ 无权限(读操作) │ 从SELECT列表中移除该字段（返回null）   │    │
│   └──────────────┴──────────────────────────────────────┘    │
│   实体级权限不足 → 直接拒绝（403）                            │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 3: 行级过滤注入                                          │
│   检查用户是否有 CONDITION 类型的 FieldPolicy                 │
│   → 解析策略表达式 → 动态注入 WHERE 子句                      │
│   示例：用户只能看自己部门的数据 → 注入 WHERE dept_id = ?    │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 4: SQL执行 + 结果脱敏                                    │
│   执行SQL → 获取结果集                                        │
│   对每行每列应用脱敏规则：                                     │
│   - READ_MASKED字段 → 调用mask_function替换值                 │
│   - 无权限字段 → 设置为null（或从结果中移除）                 │
│   - 敏感字段访问 → 写入审计日志（谁、何时、访问了什么字段）   │
└─────────────────────────────────────────────────────────────┘
```

### 4.5 写操作字段权限控制

写操作（INSERT/UPDATE/DELETE）的字段权限控制更严格：

```
写操作请求
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 1: 解析写操作涉及的字段                                   │
│   INSERT → 解析所有赋值字段                                    │
│   UPDATE → 解析SET子句中的字段                                 │
│   DELETE → 表级权限（不涉及字段级）                            │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 2: 逐字段写权限校验                                       │
│   对每个写字段，检查用户是否有 WRITE 权限                      │
│   → 无 WRITE 权限 → 拒绝整个请求（403 FieldWriteDenied）     │
│   → 有 WRITE 权限但字段是 L3/L4 高敏 → 要求审批（写入待审批）│
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 3: 字段值校验（可选）                                     │
│   检查 FieldPolicy 中的 validation_rule（如金额范围、格式）    │
│   → 校验失败 → 拒绝（400 ValidationError）                    │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 4: 执行 + 审计                                            │
│   执行写操作 → 记录前后值对比（UPDATE）→ 写入审计日志          │
│   高敏字段修改 → 触发事件通知（合规审计）                       │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. 自定义权限配置引擎

### 5.1 权限策略模型

权限策略通过 `:FieldPolicy` 节点定义，支持四种策略类型：

#### 类型一：白名单策略（WHITELIST）

```
策略名称：财务部门可看合同金额
适用字段：contract.amount, contract.tax
适用角色：finance_manager, finance_director
权限类型：READ
条件：部门 = '财务部'
```

#### 类型二：黑名单策略（BLACKLIST）

```
策略名称：普通员工禁止看薪资
适用字段：employee.salary, employee.bonus
适用角色：employee (非HR/财务)
权限类型：DENY
效果：字段返回null
```

#### 类型三：脱敏策略（MASK）

```
策略名称：客服看手机号脱敏
适用字段：customer.phone
适用角色：customer_service
权限类型：READ_MASKED
脱敏函数：MASK_PHONE
效果：138****5678
```

#### 类型四：条件策略（CONDITION）

```
策略名称：销售只能看自己的客户
适用字段：customer.* (行级过滤)
适用角色：sales_rep
权限类型：READ
条件表达式：customer.owner_id = #{current_user_id}
效果：自动注入 WHERE customer.owner_id = ?
```

```
策略名称：大区经理可看大区数据
适用字段：order.* (行级过滤)
适用角色：region_manager
条件表达式：order.region_code IN (#{user_regions})
效果：自动注入 WHERE order.region_code IN (?, ?, ?)
```

### 5.2 策略优先级与冲突解决

当多个策略同时适用时，按以下优先级解决冲突：

```
优先级从高到低：
1. DENY（黑名单）    → 最高优先级，拒绝一切
2. READ_APPROVAL     → 需审批，未审批时脱敏
3. READ_MASKED       → 脱敏可读
4. READ              → 明文可读
5. 默认策略          → 基于字段 sensitive_level 的默认处理
```

**冲突解决规则：**
- 同一字段同时有 DENY 和其他策略 → **DENY 胜出**
- 同一字段同时有 READ 和 READ_MASKED → **更严格的胜出**（READ_MASKED）
- 同一字段有多个 CONDITION 策略 → **AND 组合**（所有条件都需满足）
- 策略优先级相同 → **取交集**（最严格的组合）

### 5.3 可视化权限配置界面

```
┌─────────────────────────────────────────────────────────────────────────┐
│  字段权限策略编辑器                                                       │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  策略名称：[________________]  策略类型：[白名单 ▼]                      │
│                                                                           │
│  ┌──────────────────────┐  ┌──────────────────────┐                     │
│  │ 适用字段（图谱选择）   │  │ 适用角色              │                     │
│  │  ☑ contract.amount    │  │  ☑ finance_manager   │                     │
│  │  ☑ contract.tax       │  │  ☑ finance_director  │                     │
│  │  ☑ customer.phone     │  │  ☐ customer_service  │                     │
│  │  ☐ employee.salary    │  │  ☐ employee          │                     │
│  └──────────────────────┘  └──────────────────────┘                     │
│                                                                           │
│  权限类型：(●) READ明文  ( ) READ_MASKED脱敏  ( ) WRITE可写  ( ) DENY禁止│
│                                                                           │
│  脱敏函数：[MASK_PHONE ▼] （仅脱敏模式可用）                              │
│                                                                           │
│  行级条件（可选）：                                                        │
│  [部门 = '财务部' AND 状态 = '在职']                                      │
│  [从图谱拖拽字段到表达式] [函数] [操作符] [值]                            │
│                                                                           │
│  审批设置：[ ] 需要审批后查看明文   审批人：[________]                    │
│                                                                           │
│  优先级：[5 ▼] (1-10，数字越大优先级越高)                                │
│                                                                           │
│  [测试策略] [保存草稿] [提交审批] [发布]                                  │
│                                                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.4 权限策略测试与仿真

配置权限策略后，支持**仿真测试**：

```
策略仿真测试面板
┌─────────────────────────────────────────────────────────────┐
│  模拟用户：[zhangsan (销售代表)]                              │
│  模拟SQL：  [SELECT id, name, phone, amount FROM customer]   │
│                                                               │
│  [运行仿真]                                                   │
│                                                               │
│  仿真结果：                                                    │
│  ┌────┬────────┬──────────────┬──────────┬──────────────┐  │
│  │ id │ name   │ phone        │ amount   │ 权限说明      │  │
│  ├────┼────────┼──────────────┼──────────┼──────────────┤  │
│  │ 1  │ 张*    │ 138****5678  │ null     │ 姓名脱敏     │  │
│  │ 2  │ 李*    │ 139****1234  │ null     │ 手机号脱敏   │  │
│  │ 3  │ 王*    │ 137****9876  │ null     │ 金额无权限   │  │
│  └────┴────────┴──────────────┴──────────┴──────────────┘  │
│                                                               │
│  行级过滤：已注入 WHERE owner_id = 'zhangsan' (仅返回3条)    │
│  涉及策略：客户手机号脱敏、销售数据行级隔离、金额字段禁止      │
└─────────────────────────────────────────────────────────────┘
```

### 5.5 权限变更影响分析

修改权限策略时，图谱自动分析影响范围：

```
修改策略「客服看手机号脱敏」→ 增加 email 字段
    │
    ▼
图谱影响分析：
┌─────────────────────────────────────────────────────────────┐
│  影响范围：                                                    │
│  ✅ 涉及字段：customer.email (新增)                           │
│  ✅ 影响SQL：12个（通过 USES_COLUMN 关系找到）               │
│  ✅ 影响接口：8个（通过 OUTPUTS_ENTITY → API映射）           │
│  ✅ 影响用户：156个客服角色用户                                │
│  ✅ 缓存失效：需失效12个SQL的所有缓存（约2.3万条缓存键）     │
│                                                               │
│  变更预览：                                                    │
│  - 变更前：customer.email 返回明文 zhangsan@example.com      │
│  - 变更后：customer.email 返回脱敏 z***@example.com           │
│                                                               │
│  [确认发布] [返回修改] [查看受影响SQL列表]                     │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. 业务处理流程

### 6.1 配置流程（SQL从创建到上线）

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│ 1.需求输入 │───→│ 2.KG匹配 │───→│ 3.草稿生成│───→│ 4.可视化编辑│
│ 业务实体   │    │ 推荐已有  │    │ 模板填充  │    │ SQL+参数  │
│ +操作类型  │    │ SQL/表   │    │ 参数表单  │    │ +图谱关联 │
└──────────┘    └──────────┘    └──────────┘    └──────────┘
                                                     │
                                                     ▼
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│ 8.上线发布 │←───│ 7.审批流 │←───│ 6.测试验证│←───│ 5.影响分析│
│ 灰度+全量  │    │ 权限+合规 │    │ 执行+对比 │    │ 下游影响  │
│ 缓存预热   │    │ 审批记录  │    │ 性能基线  │    │ 权限影响  │
└──────────┘    └──────────┘    └──────────┘    └──────────┘
```

**关键步骤说明：**

- **步骤2 KG匹配**：输入业务实体+操作类型，图谱通过 `OWNS_SQL` 关系找到该实体已有的所有SQL，按 `DEPENDS_ON` 入度 PageRank排序，推荐TOP3可复用
- **步骤5 影响分析**：新SQL或修改SQL保存前，图谱自动分析：涉及的表→关联SQL→输出实体/流程→涉及的敏感字段→关联权限策略→生成影响报告
- **步骤6 测试验证**：支持仿真测试（模拟不同用户角色，验证字段权限/脱敏/行级过滤效果）+ 性能基线测试
- **步骤8 上线发布**：支持灰度（按租户/用户比例放量），全量发布后自动触发缓存预热，图谱中SQL版本节点状态从 `DRAFT`→`ACTIVE`

### 6.2 执行流程（运行时请求处理）

```
客户端请求 (entity_code + sql_code + params)
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 1: 接收请求                                            │
│   解析 sql_code / params / tenant_id / user_id              │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 2: KG元数据解析（核心差异化）                          │
│   从图谱加载 SQL节点 → 验证 status=ACTIVE                    │
│   加载 HAS_PARAM 关系 → 参数校验/类型转换/默认值填充         │
│   加载 REQUIRES_PERM 关系 → 权限点列表                       │
│   加载 USES_TABLE 关系 → 涉及的表（用于缓存失效判断）        │
│   加载 USES_COLUMN 关系 → 涉及的字段列表（用于权限校验）     │
│   加载字段 sensitive_level → 自动识别敏感字段                 │
│   图谱版本哈希 → 参与缓存键生成（KG变更自动失效缓存）        │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 3: 权限鉴权（字段级）                                   │
│   实体级权限：遍历 REQUIRES_PERM → IAM.has_permission()      │
│   字段级权限：遍历 USES_COLUMN → HAS_FIELD_PERM → 权限决策   │
│   行级权限：检查 CONDITION 策略 → 生成过滤表达式              │
│   生成「权限指纹」→ 参与缓存键（不同权限用户缓存隔离）        │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 4: 缓存命中判断                                        │
│   cache_key = "dsql:{kg_ver}:{sql_ver}:{param_hash}:{perm_fingerprint}"│
│   → L1本地Caffeine命中? → 直接返回                           │
│   → L2 Redis命中? → 回填L1 → 返回                            │
│   → 未命中 → 进入执行阶段                                     │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 5: SQL模板渲染                                         │
│   加载 sql_template → 解析 #{} 参数 + <if> 动态片段         │
│   条件分支求值 → 生成最终SQL + 有序参数列表                  │
│   行级过滤表达式注入 → 追加 WHERE 子句                       │
│   方言翻译（分页/函数）→ 适配目标数据库                      │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 6: 执行                                                │
│   datasource_code → 连接池路由 → 获取连接                   │
│   预编译 → 绑定参数 → 执行                                    │
│   结果映射（MAP/LIST/SINGLE/PAGE）                           │
│   慢SQL采集（>阈值）→ 写入审计 + 告警                        │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 7: 结果脱敏（字段级）                                   │
│   遍历结果集每行每列：                                        │
│   - READ_MASKED字段 → 调用mask_function替换值                 │
│   - 无权限字段 → 设置为null                                   │
│   - 高敏字段访问 → 写入审计日志（谁/何时/访问了什么字段）     │
│   - 审批可读字段未审批 → 脱敏 + 标记待审批                    │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 8: 缓存回填                                            │
│   结果写入 L2 Redis（TTL=缓存规则）                          │
│   结果写入 L1 本地Caffeine（短TTL）                          │
│   空结果写入空值缓存（防穿透）                                │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 9: 审计 + 事件发布                                     │
│   审计日志：谁/何时/执行什么SQL/参数/影响行数/耗时/涉及敏感字段│
│   事件发布：biz.sql.executed → 下游订阅（数据同步/通知）     │
│   指标采集：QPS/延迟/失败率/缓存命中率/脱敏次数              │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
                      返回结果给客户端
```

### 6.3 写操作流程（INSERT/UPDATE/DELETE）

```
写操作请求
    │
    ▼
Stage 1-5: 同读操作（接收/KG解析/鉴权/模板渲染）
    │
    ▼
Stage 6: 字段写权限校验（额外）
    │   解析写操作涉及的字段 → 逐字段检查WRITE权限
    │   → 无WRITE权限 → 拒绝（403 FieldWriteDenied）
    │   → 高敏字段写 → 要求审批
    │   → 字段值校验（validation_rule）
    │
    ▼
Stage 7: 事务执行（TxManager）
    │   ├── 单SQL：自动事务包裹
    │   └── 批量/多SQL：嵌套事务 + SAVEPOINT
    │
    ▼
Stage 8: 缓存级联失效（KG驱动）
    │   执行成功后，遍历 USES_TABLE 关系
    │   → 找到该表关联的所有读SQL（通过 USES_TABLE 反向）
    │   → 按 sql_code 前缀批量删除 Redis 缓存
    │   → 通知所有实例失效本地缓存（Redis Pub/Sub）
    │
    ▼
Stage 9: KG图谱更新
    │   写操作可能影响数据分布 → 更新表节点的 row_count
    │   新表/新字段被使用 → 自动创建 USES_TABLE/USES_COLUMN 关系
    │
    ▼
Stage 10: 审计 + 事件
    写操作审计更严格：记录前后值对比（for UPDATE）
    高敏字段修改 → 触发合规审计事件
    事件：biz.sql.created / updated / deleted
```

---

## 7. 核心数据结构设计

### 7.1 关系型存储表结构

```sql
-- ============================================================
-- SQL定义主表
-- ============================================================
CREATE TABLE dsql_definition (
    id              BIGINT PRIMARY KEY AUTO_INCREMENT,
    sql_code        VARCHAR(128) UNIQUE NOT NULL,
    sql_name        VARCHAR(256) NOT NULL,
    description     TEXT,
    datasource_code VARCHAR(64) NOT NULL,
    sql_template    TEXT NOT NULL,
    param_defs      JSON NOT NULL,
    result_type     VARCHAR(16) NOT NULL,
    operation_type  VARCHAR(8)  NOT NULL DEFAULT 'READ',
    cache_enabled   BOOLEAN DEFAULT TRUE,
    cache_ttl       INT DEFAULT 300,
    permission_code VARCHAR(128),
    entity_code     VARCHAR(128),
    status          VARCHAR(16) DEFAULT 'DRAFT',
    version         INT DEFAULT 1,
    version_hash    CHAR(64),
    created_by      VARCHAR(64),
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_entity (entity_code),
    INDEX idx_status (status)
);

-- ============================================================
-- 数据源配置表
-- ============================================================
CREATE TABLE dsql_datasource (
    id                  BIGINT PRIMARY KEY AUTO_INCREMENT,
    datasource_code     VARCHAR(64) UNIQUE NOT NULL,
    db_type             VARCHAR(32) NOT NULL,
    host                VARCHAR(256),
    port                INT,
    database_name       VARCHAR(128),
    username            VARCHAR(64),
    password_enc        VARCHAR(512),
    pool_config         JSON,
    read_write_split    BOOLEAN DEFAULT FALSE,
    read_datasource_codes JSON,
    status              VARCHAR(16) DEFAULT 'ACTIVE',
    created_at          TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- 字段权限策略表
-- ============================================================
CREATE TABLE dsql_field_policy (
    id              BIGINT PRIMARY KEY AUTO_INCREMENT,
    policy_code     VARCHAR(128) UNIQUE NOT NULL,
    policy_name     VARCHAR(256) NOT NULL,
    policy_type     VARCHAR(16) NOT NULL,
    entity_code     VARCHAR(128),
    table_name      VARCHAR(128),
    column_names    JSON NOT NULL,
    role_codes      JSON NOT NULL,
    perm_type       VARCHAR(16) NOT NULL,
    mask_function   VARCHAR(64),
    custom_mask     TEXT,
    condition_expr  TEXT,
    approval_required BOOLEAN DEFAULT FALSE,
    approver_role   VARCHAR(64),
    priority        INT DEFAULT 5,
    status          VARCHAR(16) DEFAULT 'ACTIVE',
    version         INT DEFAULT 1,
    created_by      VARCHAR(64),
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_entity (entity_code),
    INDEX idx_policy_type (policy_type)
);

-- ============================================================
-- 字段敏感等级元数据表（可从KG同步）
-- ============================================================
CREATE TABLE dsql_column_metadata (
    id              BIGINT PRIMARY KEY AUTO_INCREMENT,
    datasource_code VARCHAR(64) NOT NULL,
    table_name      VARCHAR(128) NOT NULL,
    column_name     VARCHAR(128) NOT NULL,
    data_type       VARCHAR(64),
    sensitive_level TINYINT DEFAULT 0,
    pii_type        VARCHAR(32),
    description     TEXT,
    updated_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uk_table_column (datasource_code, table_name, column_name)
);

-- ============================================================
-- 版本历史表（支持回滚）
-- ============================================================
CREATE TABLE dsql_version_history (
    id          BIGINT PRIMARY KEY AUTO_INCREMENT,
    sql_code    VARCHAR(128) NOT NULL,
    version     INT NOT NULL,
    sql_template TEXT NOT NULL,
    param_defs  JSON NOT NULL,
    change_note TEXT,
    created_by  VARCHAR(64),
    created_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY uk_sql_version (sql_code, version)
);

-- ============================================================
-- 执行审计日志表
-- ============================================================
CREATE TABLE dsql_audit_log (
    id              BIGINT PRIMARY KEY AUTO_INCREMENT,
    trace_id        VARCHAR(64),
    sql_code        VARCHAR(128),
    tenant_id       VARCHAR(64),
    user_id         VARCHAR(64),
    params          JSON,
    row_count       INT,
    duration_ms     INT,
    success         BOOLEAN,
    error_msg       TEXT,
    is_slow         BOOLEAN DEFAULT FALSE,
    sensitive_columns_accessed JSON,
    masked_columns  JSON,
    denied_columns  JSON,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_sql_code (sql_code),
    INDEX idx_user (user_id),
    INDEX idx_created (created_at)
);

-- ============================================================
-- 权限审批表
-- ============================================================
CREATE TABLE dsql_permission_approval (
    id              BIGINT PRIMARY KEY AUTO_INCREMENT,
    approval_code   VARCHAR(64) UNIQUE NOT NULL,
    user_id         VARCHAR(64) NOT NULL,
    policy_code     VARCHAR(128) NOT NULL,
    sql_code        VARCHAR(128),
    column_name     VARCHAR(128),
    reason          TEXT,
    status          VARCHAR(16) DEFAULT 'PENDING',
    approver_id     VARCHAR(64),
    approved_at     TIMESTAMP NULL,
    expires_at      TIMESTAMP NULL,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_user (user_id),
    INDEX idx_status (status)
);
```

### 7.2 知识图谱存储方案

**推荐：Neo4j 5.x（图数据库）+ 关系型双写**

| 操作 | 存储 | 说明 |
|------|------|------|
| SQL定义CRUD | 关系型（主存储） | 事务一致性、版本管理 |
| 关系/图谱查询 | Neo4j（图存储） | 图算法、多层遍历、血缘分析 |
| 元数据同步 | 双写 + 定时对账 | 关系型变更后异步写入Neo4j |
| 高频缓存 | Redis | 图谱版本哈希、热点SQL元数据、权限指纹 |

**图谱节点/关系创建时机**：
- SQL创建/修改 → 创建/更新 `:SQL` 节点 + `HAS_PARAM`/`REQUIRES_PERM`/`OWNS_SQL` 关系
- SQL模板中解析到表名 → 自动创建 `:Table` 节点（如不存在）+ `USES_TABLE` 关系
- SQL模板中解析到字段名 → 自动创建 `:Column` 节点 + `USES_COLUMN` 关系
- 权限策略创建/修改 → 创建/更新 `:FieldPolicy` 节点 + `APPLIES_POLICY` 关系
- 执行时发现表间JOIN → 创建/增强 `JOINS_WITH` 关系（带权重=使用次数）

---

## 8. 与MOX现有架构集成

| MOX模块 | 集成方式 | 复用价值 |
|---------|----------|----------|
| `UniversalBizDAO` | 动态SQL作为通用查询通道，DAO的biz_data表操作可配置化为SQL | 扩展DAO能力，消除硬编码 |
| `Orchestrator` 10阶段Pipeline | 动态SQL执行嵌入execute阶段，KG元数据解析嵌入meta_resolve阶段，字段权限嵌入auth阶段 | 复用编排框架，统一Pipeline |
| `IamRepository` / `InMemoryIamRepo` | `REQUIRES_PERM` 关系 → 执行前鉴权；`HAS_FIELD_PERM` → 字段级权限校验 | 复用IAM体系，权限统一管理 |
| `MetaRepository` | `entity_code` 关联MOX业务实体，行业包可预置SQL模板+权限策略 | 行业包一键安装，SQL+权限随实体打包 |
| `TxManager` | 写操作事务支持，嵌套事务 + SAVEPOINT | 复用事务管理器 |
| `Metrics` / `EventBus` | 执行指标采集 + 事件发布（sql.executed/created/updated/field_accessed） | 复用可观测性体系 |
| `AuditLog` | 执行审计，写操作前后值对比，敏感字段访问审计 | 复用审计链 |
| `kg-hub-svc` | 知识图谱存储/查询/图算法复用 | 复用KG基础设施，无需新建图数据库层 |
| `kg-algo-core` | PageRank/最短路径/子图匹配/社区发现算法复用 | SQL推荐、影响分析、SQL聚类、权限社区发现 |
| `FieldSlotAllocator` | 动态SQL结果与biz_data扩展槽位映射 | 复用字段分配算法 |

**建议新增 crate**：`mox-platform-dsql-core`（动态SQL核心），依赖：
- `mox-platform-datastore-core`（DAO/事务/字段分配）
- `mox-platform-iam-core`（权限）
- `mox-platform-meta-core`（业务实体元数据）
- `mox-kg-hub-svc`（图谱存储/查询）
- `mox-kg-algo-core`（图算法）

---

## 9. 关键技术选型

| 维度 | 选型 | 理由 |
|------|------|------|
| 图数据库 | Neo4j 5.x Community/Enterprise | 成熟稳定，Cypher查询语言强大，图算法库(APOC/GDS)丰富，Rust驱动(neo4rs)完善，支持企业级高可用 |
| SQL模板引擎 | 自研(兼容MyBatis动态SQL子集) | Rust原生，无JVM依赖，语法团队熟悉，支持`#{}`参数化+`<if>/<choose>/<foreach>/<trim>`动态片段 |
| 连接池 | deadpool (Rust) / HikariCP等价 | 高性能，支持多路复用，监控完善，异步友好 |
| 本地缓存 | moka (Rust) / Caffeine等价 | 高性能，TTL+LRU+容量限制+异步刷新，Rust原生 |
| 分布式缓存 | Redis 7.x (Hash+Pub/Sub+Lua) | 缓存存储 + 实例间失效通知 + 分布式锁(防击穿) + Lua原子操作 |
| 关系型存储 | MySQL 8.0 / PostgreSQL 15 | SQL定义主存储，事务支持，JSON字段支持 |
| 配置同步 | 双写 + Debezium CDC (可选) | 关系型变更实时同步到图谱，最终一致性 |
| 权限表达式引擎 | 自研表达式解析器 (类SpEL子集) | 支持字段引用、函数调用、逻辑运算，安全沙箱执行 |
| 可视化 | React + AntV G6（图谱可视化）+ Monaco Editor（SQL编辑器） | 图谱拖拽、血缘可视化、SQL智能提示、权限矩阵编辑器 |
| 脱敏函数 | 自研函数库 + 自定义SQL表达式 | 内置常见脱敏函数，支持自定义脱敏规则 |

---

## 10. 可行性全维评估

### 10.1 技术可行性：★★★★★

- 动态SQL+多级缓存是成熟企业模式，业界有MyBatis/ShardingSphere/Apache Calcite等大量参考
- KG元数据管理有Neo4j+图算法(GDS)成熟支撑，PageRank/最短路径/子图匹配均有工业级实现
- 字段级权限控制是金融/政务系统标配，RBAC+ABAC+字段脱敏有成熟方案
- Rust生态所有组件均有对应库（deadpool/moka/neo4rs/redis-rs），无技术盲区
- 与MOX现有架构复用率>70%，无需从零构建

### 10.2 架构合理性：★★★★★

- 七层分层清晰，各层职责单一，可独立演进
- KG与SQL解耦但通过关系网络深度协同，KG作为「认知层」不影响执行性能
- 字段级权限通过图谱关系实现，配置与执行分离，权限变更无需修改SQL
- 自定义权限策略支持四种类型（白名单/黑名单/脱敏/条件），覆盖99%企业场景
- 与MOX现有架构高度契合，可复用DAO/IAM/元数据/事务/指标/审计/KG-Hub/KG-Algo等8大核心模块

### 10.3 业务价值：★★★★★

- 消除硬编码SQL，运营自助配置，发布效率提升10倍+（从天级到分钟级）
- KG带来影响分析/血缘/推荐，是传统动态SQL平台的降维打击
- 字段级权限+自定义策略，满足等保三级/GDPR/金融监管等合规要求
- 权限仿真测试+影响分析，降低配置错误风险，避免生产事故
- 行业包预置SQL+权限策略，一键安装，大幅降低交付成本

### 10.4 实施复杂度：★★★☆☆

- 核心模块清晰，SQL执行引擎+缓存+权限引擎是主要工作量
- KG建模和图谱同步是主要增量工作，但可复用kg-hub-svc
- 多库方言适配需逐库验证，但适配器SPI模式可并行开发
- 字段级权限引擎需仔细设计权限决策树，但模型成熟

### 10.5 性能表现：★★★★☆

- 多级缓存命中后毫秒级响应（<5ms）
- 未命中时SQL执行+图谱元数据加载（元数据可缓存，首次加载后<1ms）
- 字段脱敏开销：每行每列O(1)字符串操作，万行结果集<10ms
- 写操作缓存级联失效有KG优化（只失效相关表的SQL缓存，而非全量失效）
- 权限指纹缓存隔离：不同权限用户不共享缓存，避免越权

### 10.6 可扩展性：★★★★★

- DatabaseAdapter SPI支持任意数据库，新增数据库只需实现适配器
- KG节点/关系类型可扩展，新增权限维度只需新增节点/关系类型
- 权限策略可扩展，新增策略类型只需实现策略处理器
- 脱敏函数可扩展，支持自定义函数注册
- 与MOX行业包体系兼容，SQL+权限策略可随行业包打包分发

### 10.7 风险可控性：★★★★☆

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| SQL方言差异导致执行失败 | 中 | 高 | 方言适配层 + CI中每方言回归测试 + 标准化函数封装 |
| 缓存与DB数据不一致 | 中 | 中 | 写操作主动失效 + TTL兜底 + 版本号校验 + 可配置缓存开关 |
| 动态SQL注入风险 | 低 | 极高 | 强制参数化查询(#{}) + 禁止${}拼接 + SQL白名单校验 + IAM权限 |
| 字段权限配置错误导致越权 | 中 | 高 | 仿真测试 + 权限影响分析 + 审批流 + 最小权限默认 + 审计告警 |
| 连接池耗尽 | 低 | 高 | 每数据源独立池 + 超时控制 + 熔断降级 + 监控告警 |
| 热点Key缓存击穿 | 中 | 中 | singleflight合并 + 互斥锁 + 本地缓存兜底 |
| 大结果集OOM | 低 | 高 | 流式查询 + 分页强制 + 结果集大小限制 + 游标模式 |
| KG与关系型数据不一致 | 低 | 中 | 双写 + 定时对账 + 版本哈希校验 + 自动修复 |

### 10.8 安全合规：★★★★★

- 参数化查询防注入（100%强制）
- IAM实体级权限鉴权 + 字段级权限矩阵 + 行级条件过滤
- 字段级敏感数据自动脱敏（5级敏感等级+8种脱敏函数）
- 完整审计日志（谁/何时/执行什么SQL/访问了什么敏感字段/影响行数/耗时）
- 权限变更审批流 + 高敏字段访问审批
- KG血缘支持合规审计自动化（数据流向追踪、敏感字段扩散分析）
- 密码加密存储、数据源连接信息加密

**综合判定：高度可行，强烈建议实施。**

---

## 11. 实施路线图

### Phase 1: 核心MVP（3周）

**目标**：跑通核心链路，单数据库+基础权限+单层缓存

- [ ] 关系型存储表设计 + SQL定义CRUD API
- [ ] 单数据库（MySQL）动态SQL执行引擎
- [ ] 基础模板渲染（`#{}` 参数 + `<if>` 动态片段）
- [ ] Redis单层缓存
- [ ] KG基础建模（SQL/Table/Column/Param节点 + USES_TABLE/USES_COLUMN/HAS_PARAM关系）
- [ ] Neo4j集成 + 双写同步
- [ ] 字段级权限基础版（敏感等级自动脱敏 + 白名单策略）
- [ ] 与 `UniversalBizDAO` 集成
- [ ] 核心测试覆盖（>80%）

### Phase 2: KG增强 + 企业级权限（4周）

**目标**：完整字段级权限+自定义策略+多级缓存+多数据库

- [ ] 多数据库适配器（MySQL/PostgreSQL/Oracle/SQLServer/SQLite）
- [ ] 多级缓存（本地moka + Redis）+ 防穿透/击穿/雪崩
- [ ] 完整模板引擎（`<choose>`/`<foreach>`/`<trim>`/`<where>`/`<set>`）
- [ ] KG影响分析 + 数据血缘查询API
- [ ] 字段级权限完整版（4种策略类型 + 优先级冲突解决 + 行级条件过滤）
- [ ] 自定义权限策略引擎（表达式解析 + 仿真测试）
- [ ] 权限变更影响分析
- [ ] 事务支持 + 批量执行
- [ ] IAM权限集成 + 字段级脱敏引擎
- [ ] 审计日志 + 慢SQL监控 + 敏感字段访问审计
- [ ] 版本管理 + 回滚
- [ ] 权限审批流

### Phase 3: 智能化 + 高阶（4周）

**目标**：KG智能推荐+可视化+读写分离+行业包

- [ ] KG驱动SQL推荐（PageRank排序 + 子图匹配 + 权限过滤）
- [ ] 自动JOIN推荐（JOINS_WITH关系分析 + 关联字段推荐）
- [ ] 读写分离 + 分库分表路由
- [ ] 灰度发布 + A/B测试
- [ ] 自动缓存预热 + 智能TTL调整
- [ ] 与 `Orchestrator` Pipeline深度集成（作为算子）
- [ ] 行业包SQL预置 + 权限策略一键导入
- [ ] 可视化配置画布（图谱拖拽 + SQL编辑器 + 权限矩阵 + 血缘可视化）
- [ ] 权限策略仿真测试面板
- [ ] 影响分析报告生成
- [ ] 性能调优 + 压测报告
- [ ] 文档完善 + 运维手册

---

## 附录A：字段权限决策伪代码

```rust
fn decide_field_permission(
    user: &User,
    column: &ColumnNode,
    policies: &[FieldPolicy],
) -> FieldPermissionDecision {
    // 1. 检查DENY策略（最高优先级）
    for policy in policies.iter().filter(|p| p.policy_type == BLACKLIST) {
        if policy.applies_to(user, column) {
            return FieldPermissionDecision::Deny;
        }
    }

    // 2. 检查READ_APPROVAL策略
    for policy in policies.iter().filter(|p| p.perm_type == READ_APPROVAL) {
        if policy.applies_to(user, column) {
            return if has_approval(user, column) {
                FieldPermissionDecision::ReadPlain
            } else {
                FieldPermissionDecision::ReadMasked { 
                    mask_fn: policy.mask_function,
                    pending_approval: true 
                }
            };
        }
    }

    // 3. 检查READ_MASKED策略
    for policy in policies.iter().filter(|p| p.perm_type == READ_MASKED) {
        if policy.applies_to(user, column) {
            return FieldPermissionDecision::ReadMasked { 
                mask_fn: policy.mask_function,
                pending_approval: false 
            };
        }
    }

    // 4. 检查READ明文策略
    for policy in policies.iter().filter(|p| p.perm_type == READ) {
        if policy.applies_to(user, column) {
            return FieldPermissionDecision::ReadPlain;
        }
    }

    // 5. 默认策略（基于敏感等级）
    match column.sensitive_level {
        0 | 1 => FieldPermissionDecision::ReadPlain,
        2 => FieldPermissionDecision::ReadMasked { 
            mask_fn: infer_mask_function(column), 
            pending_approval: false 
        },
        3 | 4 => FieldPermissionDecision::Deny,
        _ => FieldPermissionDecision::Deny,
    }
}
```

---

## 附录B：缓存键生成规则

```
cache_key = "dsql:" 
    + ":" + kg_version_hash      // KG图谱版本哈希（KG变更自动失效）
    + ":" + sql_version_hash     // SQL模板+参数定义哈希
    + ":" + param_hash           // 参数值哈希（排序后JSON的SHA256）
    + ":" + perm_fingerprint     // 权限指纹（用户角色+字段权限+行级条件的哈希）
    + ":" + tenant_id            // 租户隔离

示例：
dsql:kg_a1b2c3:sql_d4e5f6:param_789abc:perm_012def:tenant_t001

失效触发：
1. SQL模板/参数变更 → sql_version_hash变化 → 该SQL所有缓存失效
2. KG元数据变更（表/字段/权限关系）→ kg_version_hash变化 → 全量缓存失效
3. 权限策略变更 → perm_fingerprint变化 → 受影响用户的缓存失效
4. 写操作执行 → 按USES_TABLE关系找到相关SQL → 批量失效
5. TTL到期 → 自动失效
```

---

*文档结束 · 璇玑 RelGraph · 开发专家联盟 · 2026-08-27*
