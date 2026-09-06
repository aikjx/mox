# MOX 应用商店与发布系统 — mox 模块化系统架构设计

> 版本: 1.0 | 日期: 2026-08-28 | 状态: 企业级生产就绪

---

## 一、核心概念

### 1.1 三种发布模式

```
┌─────────────────────────────────────────────────────────────────┐
│                    MOX 应用发布三模式                              │
├──────────────────┬──────────────────┬───────────────────────────┤
│   子系统模式      │   独立运行模式    │     插件模式               │
│  (Subsystem)     │  (Standalone)    │    (Plugin)               │
├──────────────────┼──────────────────┼───────────────────────────┤
│ 安装到MOX内运行   │ 导出为独立Docker  │ 扩展MOX核心功能            │
│ 数据app_key隔离   │ 含MOX运行时+数据  │ 钩子/扩展点注入            │
│ 路由/菜单自动注册  │ 完全脱离MOX运行   │ 无需独立界面               │
│ 适用于:企业系统   │ 适用于:交付客户   │ 适用于:功能增强            │
└──────────────────┴──────────────────┴───────────────────────────┘
```

### 1.2 应用包格式 MXAP (MOX Application Package)

基于 MXDEF 扩展，一个 `.mxap` 文件（ZIP格式）包含：

```
my-app.mxap (ZIP)
├── manifest.json          # 应用元数据（名称/版本/作者/图标/描述/分类/依赖）
├── signature.sig          # 数字签名（发布者身份验证）
├── icon.png               # 应用图标（512x512）
├── screenshots/           # 截图（展示用）
│   ├── 1.png
│   └── 2.png
├── frontend/              # 前端静态文件（SPA）
│   ├── index.html
│   ├── app.js
│   └── style.css
├── backend/               # 后端扩展（可选，插件模式）
│   ├── hooks.py           # 钩子实现
│   └── api.py             # 自定义API
├── data/                  # 初始化数据（MXDEF格式）
│   ├── kernel.json        # SQL模板/权限配置
│   ├── business.json      # 业务数据
│   └── knowledge_graph.json
└── README.md              # 应用说明文档
```

### 1.3 manifest.json 规范

```json
{
  "format": "MXAP",
  "version": "1.0",
  "app_key": "my-crm",
  "app_name": "客户关系管理系统",
  "app_type": "subsystem",
  "version": "2.1.0",
  "author": "墨行科技",
  "author_key": "mox",
  "icon": "icon.png",
  "description": "企业级CRM系统，含客户管理、销售漏斗、合同管理等模块",
  "long_description": "...",
  "category": "办公协同",
  "tags": ["CRM", "客户管理", "销售"],
  "screenshots": ["screenshots/1.png", "screenshots/2.png"],
  "homepage": "https://mox.tech/apps/my-crm",
  "license": "MIT",
  "price": "free",
  "runtime": {
    "min_mox_version": "1.0.0",
    "requires_backend": false,
    "requires_database": true,
    "memory_min": "256MB"
  },
  "dependencies": [
    {"app_key": "mox-core", "version": ">=1.0.0"}
  ],
  "permissions": {
    "api_scopes": ["dsql:execute", "kg:query"],
    "data_access": ["app_key=my-crm"]
  },
  "routes": [
    {"path": "/crm", "title": "客户管理", "icon": "users", "entry": "frontend/index.html"},
    {"path": "/crm/sales", "title": "销售漏斗", "icon": "trending-up", "entry": "frontend/index.html#/sales"}
  ],
  "created_at": "2026-08-28T10:00:00+08:00",
  "updated_at": "2026-08-28T10:00:00+08:00"
}
```

---

## 二、mox 模块化系统架构

### 2.1 系统架构图

```
┌──────────────────────────────────────────────────────────────────────┐
│                          客户端层                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌────────────────────────────┐  │
│  │ 应用商店     │  │ 我的应用     │  │ 子系统运行时(iframe/路由)  │  │
│  │ 浏览/搜索    │  │ 已安装/管理  │  │ 第三方应用隔离运行         │  │
│  │ 详情/安装    │  │ 发布/更新    │  │ API代理/数据隔离           │  │
│  └──────┬──────┘  └──────┬──────┘  └─────────────┬──────────────┘  │
└─────────┼──────────────────┼─────────────────────────┼─────────────────┘
          │                  │                         │
┌─────────▼──────────────────▼─────────────────────────▼─────────────────┐
│                     应用商店服务层 (Store Service)                       │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ │
│  │ 应用目录      │ │ 发布管理      │ │ 安装引擎      │ │ 评分评论      │ │
│  │ 分类/搜索     │ │ 打包校验      │ │ 数据导入      │ │ 下载统计      │ │
│  │ 推荐/排行     │ │ 签名验证      │ │ 路由注册      │ │ 更新通知      │ │
│  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘ └──────┬───────┘ │
└─────────┼──────────────────┼──────────────────┼──────────────────┼─────────┘
          │                  │                  │                  │
┌─────────▼──────────────────▼──────────────────▼──────────────────▼─────────┐
│                     数据存储层 (Storage)                                      │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────────┐ │
│  │ mox_meta.db  │ │ mox_business │ │ 对象存储      │ │ 缓存(Redis)      │ │
│  │ store_apps   │ │ 应用业务数据  │ │ MXAP包/图标  │ │ 热门/推荐/统计   │ │
│  │ store_installs│ │ (app_key隔离)│ │ 截图         │ │                  │ │
│  │ store_ratings │ │              │ │              │ │                  │ │
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────────┘ │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 数据模型

**store_apps（应用商店目录）**
| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER | 主键 |
| app_key | TEXT | 应用唯一标识 |
| app_name | TEXT | 应用名称 |
| app_type | TEXT | subsystem/standalone/plugin |
| version | TEXT | 版本号 |
| author | TEXT | 作者/开发商 |
| author_key | TEXT | 发布者标识 |
| description | TEXT | 简短描述 |
| long_description | TEXT | 详细描述 |
| category | TEXT | 分类 |
| tags | TEXT | 标签(JSON数组) |
| icon_url | TEXT | 图标URL |
| screenshots | TEXT | 截图URL(JSON数组) |
| price | TEXT | free/paid/价格 |
| download_count | INTEGER | 下载量 |
| install_count | INTEGER | 安装量 |
| rating_avg | REAL | 平均评分 |
| rating_count | INTEGER | 评分人数 |
| status | TEXT | pending/approved/rejected/offline |
| mxap_url | TEXT | MXAP包下载地址 |
| manifest | TEXT | 完整manifest(JSON) |
| signature | TEXT | 数字签名 |
| created_at | TEXT | 上架时间 |
| updated_at | TEXT | 更新时间 |

**store_installs（已安装应用）**
| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER | 主键 |
| app_key | TEXT | 应用标识 |
| app_name | TEXT | 应用名称 |
| version | TEXT | 已安装版本 |
| install_path | TEXT | 安装路径 |
| status | TEXT | running/stopped/error |
| config | TEXT | 应用配置(JSON) |
| installed_at | TEXT | 安装时间 |
| updated_at | TEXT | 更新时间 |

**store_ratings（评分评论）**
| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER | 主键 |
| app_key | TEXT | 应用标识 |
| user_key | TEXT | 用户标识 |
| rating | INTEGER | 评分1-5 |
| comment | TEXT | 评论内容 |
| created_at | TEXT | 时间 |

---

## 三、发布流程（mox 模块化系统架构一键）

### 3.1 发布到应用商店

```
开发者本地                    MOX系统                      应用商店
   │                            │                            │
   │ 1. 开发完成                │                            │
   │    (前端+SQL模板+数据)     │                            │
   │                            │                            │
   │ 2. tools/publish_app.py    │                            │
   │    ├─ 打包MXAP             │                            │
   │    ├─ 生成签名             │                            │
   │    ├─ 校验manifest         │                            │
   │    └─ 上传 ───────────────►│                            │
   │                            │ 3. 接收+校验+签名验证      │
   │                            │    ├─ 格式校验             │
   │                            │    ├─ 安全扫描             │
   │                            │    ├─ 病毒检测             │
   │                            │    └─ 入库(pending) ─────►│
   │                            │                            │ 4. 审核(自动/人工)
   │                            │                            │    └─ approved
   │                            │                            │
   │ 5. 上架成功通知 ◄──────────│◄───────────────────────────│
   │                            │                            │
```

### 3.2 一键发布命令

```bash
# 完整一键发布（打包+签名+上传+上架）
python tools/publish_app.py \
  --app-dir ./my-app \
  --app-key my-crm \
  --name "客户关系管理" \
  --version 1.0.0 \
  --author "墨行科技" \
  --category "办公协同" \
  --upload \
  --publish

# 仅打包（不上传）
python tools/publish_app.py --app-dir ./my-app --pack-only

# 从现有MOX应用导出并发布
python tools/publish_app.py --from-app my-crm --upload --publish
```

### 3.3 安装流程

```
用户                          MOX系统                     应用商店
 │                              │                            │
 │ 1. 浏览商店                  │                            │
 │    搜索/分类/推荐             │                            │
 │                              │                            │
 │ 2. 点击安装 ────────────────►│                            │
 │                              │ 3. 下载MXAP ──────────────►│
 │                              │    ◄───────────────────────│
 │                              │ 4. 校验签名+安全扫描       │
 │                              │ 5. 解压+数据导入(MXDEF)    │
 │                              │    ├─ SQL模板注册           │
 │                              │    ├─ 业务数据导入          │
 │                              │    ├─ 权限配置导入          │
 │                              │    └─ 前端文件部署          │
 │                              │ 6. 路由/菜单注册           │
 │                              │ 7. 启动子系统              │
 │                              │                            │
 │ 8. 安装完成，可使用 ◄────────│                            │
```

### 3.4 一键安装命令

```bash
# 从商店安装
python tools/install_app.py --app-key my-crm

# 从本地MXAP包安装
python tools/install_app.py --file ./my-crm-1.0.0.mxap

# 卸载
python tools/install_app.py --app-key my-crm --uninstall

# 更新
python tools/install_app.py --app-key my-crm --update
```

---

## 四、子系统运行时

### 4.1 隔离机制

| 维度 | 隔离方式 | 说明 |
|------|---------|------|
| **数据隔离** | app_key字段过滤 | 所有业务查询自动注入 `WHERE app_key=?` |
| **路由隔离** | 独立路由前缀 | 应用路由自动注册为 `/apps/{app_key}/...` |
| **UI隔离** | iframe沙箱 | 子系统前端在iframe中运行，CSS/JS不污染主系统 |
| **API隔离** | API代理+权限 | 子系统API通过代理调用，自动注入app_key和权限 |
| **资源隔离** | 独立目录 | 前端文件存放在独立目录，不与主系统混合 |
| **权限隔离** | 应用级权限 | 用户需被授权才能访问子系统 |

### 4.2 运行模式

**模式A: iframe沙箱（推荐，安全隔离）**
```
主系统页面
├── 顶部导航（MOX统一）
├── 侧边栏（含已安装应用菜单）
└── 主内容区
    └── iframe (src="/apps/my-crm/index.html")
        └── 子系统完整界面
```

**模式B: 路由集成（轻量，体验好）**
```
主系统路由
├── /home          (MOX首页)
├── /products      (MOX产品)
└── /apps/my-crm   (子系统路由)
    └── 子系统前端直接渲染在主系统DOM中
```

**模式C: API-only（无界面，插件模式）**
```
子系统仅提供后端API钩子
├── 数据处理钩子
├── 业务逻辑扩展
└── 定时任务
```

---

## 五、独立发布（脱离MOX运行）

### 5.1 独立导出流程

```
MOX系统内的应用
    │
    ▼
tools/export_standalone.py
    ├─ 1. 导出应用数据(MXDEF)
    ├─ 2. 打包前端文件
    ├─ 3. 生成MOX运行时(精简版)
    │   ├─ FastAPI后端
    │   ├─ SQLite数据库
    │   └─ DSQL引擎
    ├─ 4. 生成Dockerfile
    ├─ 5. 生成docker-compose.yml
    ├─ 6. 生成启动脚本
    └─ 7. 打包为独立发布包
        └── my-crm-standalone-1.0.0.tar.gz
            ├── docker-compose.yml
            ├── Dockerfile
            ├── app/
            │   ├── backend/      (MOX运行时+应用后端)
            │   ├── frontend/     (应用前端)
            │   └── data/         (初始化数据)
            ├── start.sh
            └── README.md
```

### 5.2 独立运行

```bash
# 接收方只需：
tar xzf my-crm-standalone-1.0.0.tar.gz
cd my-crm-standalone-1.0.0
docker-compose up -d

# 或直接运行
./start.sh
```

独立包包含完整MOX运行时，接收方无需安装MOX系统，开箱即用。

---

## 六、应用商店分类体系

### 6.1 一级分类

| 分类 | 说明 | 示例应用 |
|------|------|---------|
| 办公协同 | OA/CRM/项目管理 | CRM、项目管理、审批流 |
| 数据分析 | BI/报表/可视化 | 数据看板、报表引擎 |
| 电商零售 | 商城/订单/库存 | 商城系统、库存管理 |
| 教育培训 | 课程/考试/学习 | 在线课程、考试系统 |
| 人力资源 | 招聘/考勤/薪酬 | 招聘管理、考勤系统 |
| 财务管理 | 记账/发票/报销 | 财务记账、报销系统 |
| 内容管理 | CMS/博客/文档 | 企业官网、文档中心 |
| 客服支持 | 工单/在线客服/FAQ | 工单系统、在线客服 |
| 开发工具 | 插件/扩展/主题 | SQL模板包、主题包 |
| 行业方案 | 金融/医疗/制造等 | 银行核心、医院HIS |

### 6.2 应用类型

| 类型 | 说明 | 运行方式 |
|------|------|---------|
| subsystem | 完整子系统 | iframe/路由集成 |
| standalone | 可独立运行 | 导出为Docker包 |
| plugin | 功能插件 | 钩子注入，无独立界面 |
| template | 数据模板 | 仅SQL模板+示例数据 |
| theme | 主题包 | 仅前端样式 |

---

## 七、安全机制

### 7.1 发布安全

- **数字签名**：每个MXAP包必须签名，安装时验证发布者身份
- **安全扫描**：上传时自动扫描恶意代码、SQL注入、XSS
- **沙箱测试**：自动在隔离环境运行测试，检测异常行为
- **人工审核**：高权限应用需人工审核
- **版本回滚**：保留历史版本，发现问题可快速回滚

### 7.2 运行安全

- **数据隔离**：app_key强制过滤，应用无法访问其他应用数据
- **权限最小化**：应用仅能访问声明的API scope
- **iframe沙箱**：子系统前端在沙箱iframe中运行
- **API代理**：子系统API通过代理，自动鉴权和限流
- **审计日志**：所有应用操作记录审计日志

---

## 八、API 接口

### 8.1 商店接口

```
GET    /api/store/apps              # 浏览应用（支持分类/搜索/排序/分页）
GET    /api/store/apps/{app_key}    # 应用详情
GET    /api/store/apps/{app_key}/versions  # 版本列表
GET    /api/store/categories         # 分类列表
GET    /api/store/featured           # 推荐应用
GET    /api/store/hot                # 热门排行
GET    /api/store/new                # 最新上架
GET    /api/store/search?q=xxx       # 搜索
```

### 8.2 发布接口

```
POST   /api/store/publish            # 发布应用（上传MXAP包）
GET    /api/store/publish/{id}/status  # 发布状态
POST   /api/store/apps/{app_key}/offline  # 下架
POST   /api/store/apps/{app_key}/update   # 更新
```

### 8.3 安装管理接口

```
GET    /api/store/installed          # 已安装应用列表
POST   /api/store/install/{app_key}  # 安装应用
DELETE /api/store/install/{app_key}  # 卸载应用
POST   /api/store/install/{app_key}/update  # 更新应用
POST   /api/store/install/{app_key}/start   # 启动子系统
POST   /api/store/install/{app_key}/stop    # 停止子系统
GET    /api/store/install/{app_key}/config   # 获取配置
PUT    /api/store/install/{app_key}/config   # 更新配置
```

### 8.4 评分评论接口

```
GET    /api/store/apps/{app_key}/ratings     # 评分列表
POST   /api/store/apps/{app_key}/ratings      # 提交评分
GET    /api/store/apps/{app_key}/ratings/stats  # 评分统计
```

---

## 九、一键工具清单

| 工具 | 功能 |
|------|------|
| `tools/publish_app.py` | 一键发布（打包→签名→上传→上架） |
| `tools/install_app.py` | 一键安装/卸载/更新 |
| `tools/export_standalone.py` | 一键导出为独立运行包 |
| `tools/pack_mxap.py` | 仅打包MXAP（不上传） |
| `tools/verify_mxap.py` | MXAP包校验（签名/安全/格式） |

---

## 十、人人有系统，企业快发布

### 10.1 个人开发者

```
1. 在MOX低代码平台开发应用（可视化配置SQL+前端）
2. 一键发布到应用商店
3. 其他用户安装使用
4. 获得评分/下载量/收益
```

### 10.2 企业客户

```
1. 浏览应用商店，找到需要的系统
2. 一键安装到企业MOX平台
3. 数据自动隔离，配置企业专属参数
4. 或购买独立运行包，部署在企业自有服务器
```

### 10.3 开发商/ISV

```
1. 基于MOX平台开发行业解决方案
2. 发布到应用商店，触达海量企业客户
3. 支持免费/付费/订阅多种商业模式
4. 一键导出独立包，支持私有化交付
```
