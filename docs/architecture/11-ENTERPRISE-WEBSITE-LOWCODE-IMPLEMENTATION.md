# 企业官网低代码实施方案 · 基于需求图谱

> 基于7维度需求图谱（业务目标/用户场景/功能需求/内容需求/技术需求/运营需求/合规需求）
> 技术栈：mox-dsql-core（动态SQL引擎）+ axum（API）+ Nuxt.js（前端SSR）
> 版本：v1.0

---

## 一、需求映射总览

### 1.1 功能模块→数据库表映射

| 功能编号 | 功能模块 | 优先级 | 对应数据表 | SQL定义数量 |
|:---:|------|:---:|------|:---:|
| F1 | 首页展示 | P0 | cms_banner, cms_home_block | 4 |
| F2 | 关于我们 | P0 | cms_about, cms_history, cms_team, cms_honor | 6 |
| F3 | 产品/服务展示 | P0 | cms_product, cms_product_category, cms_product_download | 8 |
| F4 | 新闻动态 | P1 | cms_news, cms_news_category | 6 |
| F5 | 案例展示 | P1 | cms_case, cms_case_category | 6 |
| F6 | 人才招聘 | P2 | cms_job, cms_resume | 5 |
| F7 | 联系我们 | P0 | cms_contact, cms_message, cms_branch | 5 |
| F8 | 搜索功能 | P1 | （跨表搜索，无需独立表） | 3 |
| F9 | 在线咨询 | P0 | cms_consultation, cms_chat_record | 4 |
| F10 | 多语言支持 | P2 | （所有表加lang字段） | 0 |
| - | 后台管理 | P0 | cms_admin, cms_role, cms_permission, cms_operation_log | 6 |
| - | SEO管理 | P0 | cms_seo | 3 |
| - | 数据统计 | P1 | cms_visit_log, cms_page_view | 4 |
| **总计** | | | **22张表** | **60个SQL定义** |

### 1.2 内容需求→数据表映射

| 内容编号 | 内容类型 | 对应数据表 | 优先级 |
|:---:|------|------|:---:|
| C1 | 公司介绍 | cms_about, cms_history, cms_team | P0 |
| C2 | 产品内容 | cms_product, cms_product_download | P0 |
| C3 | 客户案例 | cms_case | P1 |
| C4 | 新闻资讯 | cms_news | P1 |
| C5 | FAQ问答库 | cms_faq | P1 |
| C6 | 团队风采 | cms_team, cms_gallery | P2 |

---

## 二、数据库表设计

### 2.1 核心业务表（P0）

#### 表1：cms_banner（首页Banner）

```sql
CREATE TABLE cms_banner (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title VARCHAR(200) NOT NULL COMMENT '标题',
    subtitle VARCHAR(500) COMMENT '副标题',
    image_url VARCHAR(500) NOT NULL COMMENT '图片地址',
    mobile_image_url VARCHAR(500) COMMENT '移动端图片',
    link_type VARCHAR(20) DEFAULT 'url' COMMENT '链接类型：url/page/product/news/none',
    link_value VARCHAR(500) COMMENT '链接值',
    sort INTEGER DEFAULT 0 COMMENT '排序',
    status TINYINT DEFAULT 1 COMMENT '状态：0禁用 1启用',
    start_time DATETIME COMMENT '开始时间',
    end_time DATETIME COMMENT '结束时间',
    lang VARCHAR(10) DEFAULT 'zh' COMMENT '语言',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_banner_status ON cms_banner(status, sort);
```

#### 表2：cms_product（产品）

```sql
CREATE TABLE cms_product (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(200) NOT NULL COMMENT '产品名称',
    en_name VARCHAR(200) COMMENT '英文名称',
    category_id INTEGER COMMENT '分类ID',
    cover_image VARCHAR(500) NOT NULL COMMENT '封面图',
    images TEXT COMMENT '图片列表JSON',
    videos TEXT COMMENT '视频列表JSON',
    summary VARCHAR(500) COMMENT '摘要',
    description TEXT COMMENT '详细描述（富文本）',
    specs TEXT COMMENT '技术参数JSON',
    price DECIMAL(10,2) COMMENT '价格（0表示面议）',
    price_unit VARCHAR(20) COMMENT '价格单位',
    tags VARCHAR(500) COMMENT '标签JSON',
    features TEXT COMMENT '核心卖点JSON',
    sort INTEGER DEFAULT 0 COMMENT '排序',
    is_recommend TINYINT DEFAULT 0 COMMENT '是否推荐',
    is_new TINYINT DEFAULT 0 COMMENT '是否新品',
    is_hot TINYINT DEFAULT 0 COMMENT '是否热销',
    status TINYINT DEFAULT 1 COMMENT '状态：0草稿 1发布 2下架',
    views INTEGER DEFAULT 0 COMMENT '浏览量',
    lang VARCHAR(10) DEFAULT 'zh' COMMENT '语言',
    seo_title VARCHAR(200) COMMENT 'SEO标题',
    seo_keywords VARCHAR(500) COMMENT 'SEO关键词',
    seo_description VARCHAR(500) COMMENT 'SEO描述',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_product_category ON cms_product(category_id, status);
CREATE INDEX idx_product_status ON cms_product(status, sort);
CREATE INDEX idx_product_recommend ON cms_product(is_recommend, status);
```

#### 表3：cms_product_category（产品分类）

```sql
CREATE TABLE cms_product_category (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(100) NOT NULL COMMENT '分类名称',
    en_name VARCHAR(100) COMMENT '英文名称',
    parent_id INTEGER DEFAULT 0 COMMENT '父分类ID',
    icon VARCHAR(500) COMMENT '分类图标',
    description VARCHAR(500) COMMENT '分类描述',
    sort INTEGER DEFAULT 0 COMMENT '排序',
    status TINYINT DEFAULT 1 COMMENT '状态',
    lang VARCHAR(10) DEFAULT 'zh',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### 表4：cms_news（新闻动态）

```sql
CREATE TABLE cms_news (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title VARCHAR(200) NOT NULL COMMENT '标题',
    en_title VARCHAR(200) COMMENT '英文标题',
    category_id INTEGER COMMENT '分类ID',
    cover_image VARCHAR(500) COMMENT '封面图',
    summary VARCHAR(500) COMMENT '摘要',
    content TEXT NOT NULL COMMENT '正文（富文本）',
    author VARCHAR(50) COMMENT '作者',
    source VARCHAR(100) COMMENT '来源',
    views INTEGER DEFAULT 0 COMMENT '浏览量',
    likes INTEGER DEFAULT 0 COMMENT '点赞数',
    is_top TINYINT DEFAULT 0 COMMENT '是否置顶',
    is_recommend TINYINT DEFAULT 0 COMMENT '是否推荐',
    status TINYINT DEFAULT 1 COMMENT '0草稿 1发布 2下架',
    publish_time DATETIME COMMENT '发布时间',
    lang VARCHAR(10) DEFAULT 'zh',
    seo_title VARCHAR(200),
    seo_keywords VARCHAR(500),
    seo_description VARCHAR(500),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_news_category ON cms_news(category_id, status);
CREATE INDEX idx_news_status ON cms_news(status, publish_time DESC);
```

#### 表5：cms_case（客户案例）

```sql
CREATE TABLE cms_case (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title VARCHAR(200) NOT NULL COMMENT '案例标题',
    customer_name VARCHAR(200) COMMENT '客户名称',
    customer_logo VARCHAR(500) COMMENT '客户Logo',
    industry VARCHAR(100) COMMENT '所属行业',
    category_id INTEGER COMMENT '案例分类',
    cover_image VARCHAR(500) COMMENT '封面图',
    images TEXT COMMENT '图片列表JSON',
    summary VARCHAR(500) COMMENT '案例摘要',
    background TEXT COMMENT '项目背景',
    solution TEXT COMMENT '解决方案',
    results TEXT COMMENT '实施成果（JSON：指标+数值）',
    customer_quote TEXT COMMENT '客户评价',
    sort INTEGER DEFAULT 0,
    is_recommend TINYINT DEFAULT 0,
    status TINYINT DEFAULT 1,
    views INTEGER DEFAULT 0,
    lang VARCHAR(10) DEFAULT 'zh',
    seo_title VARCHAR(200),
    seo_keywords VARCHAR(500),
    seo_description VARCHAR(500),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_case_industry ON cms_case(industry, status);
CREATE INDEX idx_case_status ON cms_case(status, sort);
```

#### 表6：cms_message（在线留言）

```sql
CREATE TABLE cms_message (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(50) NOT NULL COMMENT '姓名',
    phone VARCHAR(20) NOT NULL COMMENT '电话',
    email VARCHAR(100) COMMENT '邮箱',
    company VARCHAR(200) COMMENT '公司名称',
    position VARCHAR(50) COMMENT '职位',
    product_id INTEGER COMMENT '咨询产品',
    content TEXT NOT NULL COMMENT '留言内容',
    source_page VARCHAR(500) COMMENT '来源页面',
    ip VARCHAR(50) COMMENT 'IP地址',
    user_agent VARCHAR(500) COMMENT 'UA',
    status TINYINT DEFAULT 0 COMMENT '0待处理 1已联系 2已成交 3无效',
    handler_id INTEGER COMMENT '处理人',
    handle_note TEXT COMMENT '处理备注',
    handled_at DATETIME COMMENT '处理时间',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_message_status ON cms_message(status, created_at DESC);
CREATE INDEX idx_message_phone ON cms_message(phone);
```

#### 表7：cms_contact（联系方式）

```sql
CREATE TABLE cms_contact (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type VARCHAR(20) NOT NULL COMMENT '类型：phone/email/address/wechat/qq/weibo',
    label VARCHAR(50) COMMENT '标签（如：销售热线、技术支持）',
    value VARCHAR(500) NOT NULL COMMENT '值',
    icon VARCHAR(100) COMMENT '图标',
    sort INTEGER DEFAULT 0,
    status TINYINT DEFAULT 1,
    lang VARCHAR(10) DEFAULT 'zh',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### 表8：cms_about（关于我们-内容块）

```sql
CREATE TABLE cms_about (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_type VARCHAR(30) NOT NULL COMMENT '块类型：company/vision/mission/value/culture',
    title VARCHAR(200) NOT NULL COMMENT '标题',
    en_title VARCHAR(200),
    content TEXT NOT NULL COMMENT '内容（富文本）',
    images TEXT COMMENT '图片JSON',
    sort INTEGER DEFAULT 0,
    status TINYINT DEFAULT 1,
    lang VARCHAR(10) DEFAULT 'zh',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 2.2 辅助业务表（P1-P2）

#### 表9：cms_history（发展历程）
#### 表10：cms_team（团队成员）
#### 表11：cms_honor（资质荣誉）
#### 表12：cms_job（招聘职位）
#### 表13：cms_resume（简历投递）
#### 表14：cms_faq（FAQ问答）
#### 表15：cms_consultation（在线咨询）
#### 表16：cms_branch（分支机构）
#### 表17：cms_gallery（图片集）
#### 表18：cms_product_download（下载中心）

### 2.3 系统表（P0）

#### 表19：cms_admin（管理员）
#### 表20：cms_role（角色）
#### 表21：cms_permission（权限）
#### 表22：cms_operation_log（操作日志）
#### 表23：cms_seo（SEO配置）
#### 表24：cms_visit_log（访问日志）
#### 表25：cms_page_view（页面统计）

---

## 三、动态SQL定义清单（60个）

### 3.1 首页模块（4个）

| SQL编码 | SQL名称 | 类型 | 模板 | 缓存 |
|---------|---------|------|------|------|
| home_banner_list | 首页Banner列表 | LIST | `SELECT * FROM cms_banner WHERE status=1 AND lang={{lang}} {?if start_time?}AND (start_time IS NULL OR start_time <= NOW()){?endif?} {?if end_time?}AND (end_time IS NULL OR end_time >= NOW()){?endif?} ORDER BY sort ASC` | 300s |
| home_recommend_products | 首页推荐产品 | LIST | `SELECT id,name,cover_image,summary,price FROM cms_product WHERE status=1 AND is_recommend=1 AND lang={{lang}} ORDER BY sort ASC LIMIT {{limit}}` | 600s |
| home_latest_news | 首页最新新闻 | LIST | `SELECT id,title,cover_image,summary,publish_time FROM cms_news WHERE status=1 AND lang={{lang}} ORDER BY is_top DESC, publish_time DESC LIMIT {{limit}}` | 300s |
| home_recommend_cases | 首页推荐案例 | LIST | `SELECT id,title,customer_name,cover_image,summary FROM cms_case WHERE status=1 AND is_recommend=1 AND lang={{lang}} ORDER BY sort ASC LIMIT {{limit}}` | 600s |

### 3.2 产品模块（8个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| product_category_tree | 产品分类树 | LIST | 600s |
| product_list | 产品列表（分页+筛选） | LIST | 300s |
| product_detail | 产品详情 | MAP | 600s |
| product_related | 相关产品 | LIST | 600s |
| product_hot | 热销产品 | LIST | 300s |
| product_new | 新品上市 | LIST | 300s |
| product_increment_view | 增加浏览量 | UPDATE | 不缓存 |
| product_search | 产品搜索 | LIST | 60s |

**product_list 模板示例：**
```sql
SELECT id, name, cover_image, summary, price, tags, is_new, is_hot
FROM cms_product
WHERE status = 1 AND lang = {{lang}}
{?if category_id?}AND category_id = {{category_id}}{?endif?}
{?if keyword?}AND (name LIKE '%{{keyword}}%' OR summary LIKE '%{{keyword}}%' OR description LIKE '%{{keyword}}%'){?endif?}
{?if is_recommend?}AND is_recommend = 1{?endif?}
{?if is_new?}AND is_new = 1{?endif?}
{?if is_hot?}AND is_hot = 1{?endif?}
ORDER BY is_top DESC, sort ASC, created_at DESC
LIMIT {{offset}}, {{page_size}}
```

### 3.3 新闻模块（6个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| news_category_list | 新闻分类列表 | LIST | 600s |
| news_list | 新闻列表（分页+筛选） | LIST | 300s |
| news_detail | 新闻详情 | MAP | 600s |
| news_related | 相关新闻 | LIST | 600s |
| news_hot | 热门新闻 | LIST | 300s |
| news_increment_view | 增加浏览量 | UPDATE | 不缓存 |

### 3.4 案例模块（6个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| case_category_list | 案例分类列表 | LIST | 600s |
| case_list | 案例列表（分页+行业筛选） | LIST | 300s |
| case_detail | 案例详情 | MAP | 600s |
| case_related | 相关案例 | LIST | 600s |
| case_by_industry | 按行业筛选案例 | LIST | 300s |
| case_increment_view | 增加浏览量 | UPDATE | 不缓存 |

### 3.5 关于我们模块（6个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| about_block_list | 关于我们内容块列表 | LIST | 600s |
| about_history_timeline | 发展历程时间线 | LIST | 600s |
| about_team_list | 团队成员列表 | LIST | 600s |
| about_honor_list | 资质荣誉列表 | LIST | 600s |
| about_gallery | 图片集 | LIST | 600s |
| about_faq_list | FAQ列表 | LIST | 600s |

### 3.6 联系我们模块（5个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| contact_list | 联系方式列表 | LIST | 600s |
| contact_branch_list | 分支机构列表 | LIST | 600s |
| message_submit | 提交留言 | UPDATE | 不缓存 |
| message_list_admin | 后台留言列表（分页） | LIST | 不缓存 |
| message_handle | 处理留言 | UPDATE | 不缓存 |

### 3.7 招聘模块（5个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| job_list | 职位列表（分页+部门筛选） | LIST | 300s |
| job_detail | 职位详情 | MAP | 600s |
| job_department_list | 部门列表 | LIST | 600s |
| resume_submit | 提交简历 | UPDATE | 不缓存 |
| resume_list_admin | 后台简历列表 | LIST | 不缓存 |

### 3.8 搜索模块（3个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| search_all | 全站搜索（产品+新闻+案例联合） | LIST | 60s |
| search_hot_keywords | 热门搜索词 | LIST | 300s |
| search_suggest | 搜索建议（ autocomplete） | LIST | 60s |

### 3.9 在线咨询模块（4个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| consultation_submit | 提交咨询 | UPDATE | 不缓存 |
| consultation_list_admin | 后台咨询列表 | LIST | 不缓存 |
| chat_record_list | 聊天记录列表 | LIST | 不缓存 |
| chat_record_add | 添加聊天记录 | UPDATE | 不缓存 |

### 3.10 后台管理模块（6个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| admin_login | 管理员登录 | MAP | 不缓存 |
| admin_list | 管理员列表 | LIST | 不缓存 |
| role_list | 角色列表 | LIST | 不缓存 |
| permission_tree | 权限树 | LIST | 不缓存 |
| operation_log_list | 操作日志列表 | LIST | 不缓存 |
| operation_log_add | 添加操作日志 | UPDATE | 不缓存 |

### 3.11 SEO模块（3个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| seo_get | 获取页面SEO配置 | MAP | 600s |
| seo_list_admin | 后台SEO列表 | LIST | 不缓存 |
| seo_save | 保存SEO配置 | UPDATE | 不缓存 |

### 3.12 数据统计模块（4个）

| SQL编码 | SQL名称 | 类型 | 缓存 |
|---------|---------|------|------|
| visit_log_add | 记录访问 | UPDATE | 不缓存 |
| pv_stat_daily | 日PV统计 | LIST | 300s |
| uv_stat_daily | 日UV统计 | LIST | 300s |
| page_view_rank | 页面访问排行 | LIST | 300s |

---

## 四、API设计（通用DSQL API + 专用API）

### 4.1 通用DSQL API（核心，1个接口搞定所有查询）

```
POST /api/dsql/execute
Content-Type: application/json

Request:
{
  "sql_code": "product_list",
  "params": {
    "lang": "zh",
    "category_id": 1,
    "keyword": "手机",
    "offset": 0,
    "page_size": 20
  },
  "trace_id": "optional-trace-id"
}

Response:
{
  "success": true,
  "code": 0,
  "message": "ok",
  "data": [...],
  "total": 156,
  "page": 1,
  "page_size": 20,
  "cache_hit": true,
  "duration_ms": 2,
  "trace_id": "xxx"
}
```

### 4.2 专用API（写操作+文件上传）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | /api/message/submit | 提交留言（含参数校验+防刷） |
| POST | /api/resume/submit | 提交简历（含文件上传） |
| POST | /api/consultation/submit | 提交咨询 |
| POST | /api/upload/image | 图片上传（返回URL） |
| POST | /api/upload/file | 文件上传（返回URL） |
| POST | /api/admin/login | 管理员登录 |
| POST | /api/admin/logout | 管理员登出 |
| GET | /api/sitemap.xml | 生成sitemap |
| GET | /api/robots.txt | robots.txt |

### 4.3 后台管理CRUD API（通用）

```
# 通用CRUD（通过table参数指定表）
GET    /api/admin/crud/{table}?page=1&page_size=20&keyword=xxx
GET    /api/admin/crud/{table}/{id}
POST   /api/admin/crud/{table}
PUT    /api/admin/crud/{table}/{id}
DELETE /api/admin/crud/{table}/{id}
```

**支持的table：** product, product_category, news, news_category, case, case_category, banner, about, team, honor, job, contact, message, resume, faq, seo, admin, role

---

## 五、前端页面规划（Nuxt.js SSR）

### 5.1 页面路由

| 路由 | 页面 | 优先级 | 依赖SQL |
|------|------|:---:|---------|
| / | 首页 | P0 | home_banner_list, home_recommend_products, home_latest_news, home_recommend_cases |
| /about | 关于我们 | P0 | about_block_list, about_history_timeline, about_team_list, about_honor_list |
| /products | 产品列表 | P0 | product_category_tree, product_list |
| /products/:id | 产品详情 | P0 | product_detail, product_related |
| /news | 新闻列表 | P1 | news_category_list, news_list |
| /news/:id | 新闻详情 | P1 | news_detail, news_related |
| /cases | 案例列表 | P1 | case_category_list, case_list |
| /cases/:id | 案例详情 | P1 | case_detail, case_related |
| /contact | 联系我们 | P0 | contact_list, contact_branch_list |
| /jobs | 招聘列表 | P2 | job_list, job_department_list |
| /jobs/:id | 职位详情 | P2 | job_detail |
| /faq | FAQ | P1 | about_faq_list |
| /search | 搜索结果 | P1 | search_all |
| /privacy | 隐私政策 | P0 | 静态页面 |
| /terms | 用户协议 | P1 | 静态页面 |
| /admin | 后台管理 | P0 | 后台全套 |

### 5.2 通用组件

| 组件名 | 功能 | 复用页面 |
|--------|------|---------|
| Header | 顶部导航（含多语言切换、搜索框） | 全部 |
| Footer | 底部（联系方式、备案号、友情链接） | 全部 |
| BannerCarousel | Banner轮播 | 首页 |
| ProductCard | 产品卡片 | 首页、产品列表、相关产品 |
| NewsCard | 新闻卡片 | 首页、新闻列表、相关新闻 |
| CaseCard | 案例卡片 | 首页、案例列表、相关案例 |
| PageHeader | 页面头部（标题+面包屑） | 列表页、详情页 |
| Pagination | 分页组件 | 所有列表页 |
| SearchBox | 搜索框 | Header、搜索页 |
| ContactForm | 联系表单 | 联系我们、产品详情 |
| LanguageSwitch | 语言切换 | Header |

---

## 六、实施计划

### 6.1 阶段一：基础搭建（2天）

| 任务 | 内容 | 产出 |
|------|------|------|
| 1 | 数据库初始化 | 25张表 + 索引 |
| 2 | SQL定义初始化 | 60个SQL定义插入数据库 |
| 3 | 后端项目搭建 | axum + mox-dsql-core + 通用API |
| 4 | 前端项目搭建 | Nuxt.js + Element Plus + 通用布局 |
| 5 | 部署环境 | Docker + Nginx + HTTPS |

### 6.2 阶段二：P0功能开发（5天）

| 模块 | 页面 | 后端SQL | 前端组件 |
|------|------|---------|---------|
| 首页 | / | 4个 | BannerCarousel + 各模块卡片 |
| 关于我们 | /about | 4个 | 时间线 + 团队卡片 + 荣誉墙 |
| 产品中心 | /products, /products/:id | 8个 | 分类树 + 产品卡片 + 详情页 |
| 联系我们 | /contact | 5个 | 联系方式 + 地图 + 留言表单 |
| 后台管理 | /admin | 6个 + 通用CRUD | 登录 + 仪表盘 + 内容管理 |

### 6.3 阶段三：P1功能开发（4天）

| 模块 | 页面 | 后端SQL | 前端组件 |
|------|------|---------|---------|
| 新闻动态 | /news, /news/:id | 6个 | 新闻列表 + 详情页 + 相关推荐 |
| 案例展示 | /cases, /cases/:id | 6个 | 案例列表 + 详情页 + 行业筛选 |
| 搜索功能 | /search | 3个 | 搜索框 + 结果页 + 高亮 |
| FAQ | /faq | 1个 | 手风琴问答组件 |
| 数据统计 | 后台 | 4个 | 访问统计图表 |

### 6.4 阶段四：P2功能+优化（3天）

| 任务 | 内容 |
|------|------|
| 人才招聘 | 职位列表 + 详情 + 简历投递 |
| 多语言 | 中英文切换（所有表lang字段） |
| 在线咨询 | 在线客服接入（第三方客服代码） |
| SEO优化 | sitemap.xml + robots.txt + 结构化数据 |
| 性能优化 | 图片懒加载 + CDN + 缓存策略调优 |
| 安全加固 | 防XSS + 防CSRF + 留言防刷 + 限流 |

### 6.5 总工期

| 阶段 | 工期 | 累计 |
|------|------|------|
| 阶段一：基础搭建 | 2天 | 2天 |
| 阶段二：P0功能 | 5天 | 7天 |
| 阶段三：P1功能 | 4天 | 11天 |
| 阶段四：P2+优化 | 3天 | 14天 |
| **总计** | **14天** | |

---

## 七、与传统开发对比

| 对比项 | 传统开发（Vue+SpringBoot） | 低代码（mox-dsql-core） | 提升 |
|--------|---------------------------|------------------------|------|
| 数据库表设计 | 2天 | 0.5天（复用模板） | 4倍 |
| 后端API开发 | 8-10天（25张表CRUD） | 1天（通用DSQL API+60个SQL定义） | 8-10倍 |
| 前端页面开发 | 8-10天 | 5-6天（通用组件+配置） | 1.5-2倍 |
| 后台管理 | 5-7天 | 2天（通用CRUD组件） | 2.5-3.5倍 |
| 内容修改 | 改代码+部署（30min-2h） | 后台配置即时生效（1min） | 30-120倍 |
| 新增功能模块 | 2-3天 | 0.5天（建表+SQL配置+模板） | 4-6倍 |
| 总工期 | 25-35天 | 14天 | 2-2.5倍 |
| 后端代码量 | 8000+行 | 800行（通用API+中间件） | 减少90% |

---

## 八、关键技术点

### 8.1 SEO优化（SSR）

- 使用Nuxt.js服务端渲染，确保爬虫能获取完整HTML
- 每个页面动态设置TDK（从cms_seo表读取）
- 自动生成sitemap.xml（产品/新闻/案例URL）
- 结构化数据（JSON-LD）：产品、文章、面包屑

### 8.2 性能优化

- 多级缓存：SQL缓存（mox-dsql-core）+ 页面缓存（Nuxt）+ CDN缓存
- 图片优化：自动压缩 + WebP格式 + 懒加载 + 响应式图片
- 首屏优化：关键CSS内联 + 字体预加载 + 骨架屏

### 8.3 安全防护

- SQL注入：mox-dsql-core内置参数化查询，所有用户输入不直接拼接SQL
- XSS防护：富文本输出前HTML转义 + CSP策略
- CSRF防护：表单Token验证
- 留言防刷：IP限流（1分钟最多3条）+ 验证码 + 敏感词过滤
- 文件上传：类型校验 + 大小限制 + 随机文件名 + 病毒扫描（可选）

### 8.4 数据统计

- 访问日志：PV/UV/来源/设备/地域
- 转化漏斗：访问→产品详情→留言→成交
- 热门页面：产品/新闻/案例访问排行
- 后台仪表盘：实时数据 + 趋势图表

---

## 九、合规需求实现

| 合规项 | 实现方式 | 优先级 |
|--------|---------|:---:|
| ICP备案 | 页脚展示备案号 + 工信部链接 | P0 |
| 隐私政策 | /privacy 静态页面 + 留言表单勾选同意 | P0 |
| 用户协议 | /terms 静态页面 | P1 |
| 知识产权声明 | 页脚版权信息 + 图片水印（可选） | P1 |
| 等保合规 | HTTPS + 访问日志 + 操作日志 + 权限管理 + 定期备份 | P2 |

---

## 十、总结

### 10.1 方案优势

1. **全维需求覆盖**：7维度需求图谱完整映射，25张表+60个SQL定义覆盖所有功能
2. **开发效率提升2-2.5倍**：传统开发25-35天，低代码14天
3. **后端代码量减少90%**：通用DSQL API替代8000行CRUD代码
4. **内容更新效率提升30-120倍**：后台配置即时生效，无需改代码部署
5. **企业级性能**：多级缓存 + SSR + CDN，首屏<3秒
6. **安全合规**：内置防注入/防XSS/防刷，满足等保基本要求
7. **可扩展性强**：新增模块只需建表+SQL配置+前端模板，0.5天搞定

### 10.2 预期效果

| 指标 | 目标值 |
|------|--------|
| 开发周期 | 14天（传统25-35天） |
| 首屏加载 | <3秒 |
| 内容更新时间 | <5分钟（传统30分钟-2小时） |
| 新增功能模块 | <0.5天（传统2-3天） |
| 后端代码量 | <1000行（传统8000+行） |
| 网站可用性 | 99.9% |
| SEO收录 | 100%页面可被爬虫抓取 |

---

**方案结论：基于mox-dsql-core的低代码方案，可在14天内完成企业官网全功能开发，开发效率提升2-2.5倍，后端代码量减少90%，同时保持企业级性能和安全性。**
