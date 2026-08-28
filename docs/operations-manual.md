# MOX 全维低代码平台 — 操作说明手册

> **版本**: 2.0 | **日期**: 2026-08-28
> **从安装到上线，一本手册搞定。所有命令可直接复制执行。**

---

## 目录

1. [快速开始（5分钟跑起来）](#一快速开始5分钟跑起来)
2. [平台使用指南](#二平台使用指南)
3. [数据导出与导入](#三数据导出与导入)
4. [应用发布与安装](#四应用发布与安装)
5. [独立部署与交付](#五独立部署与交付)
6. [域名绑定与生产上线](#六域名绑定与生产上线)
7. [运维与监控](#七运维与监控)
8. [常见问题排查](#八常见问题排查)

---

## 一、快速开始（5分钟跑起来）

### 1.1 环境要求

- Python 3.10+
- 512MB+ 内存
- Redis 6+（可选，无Redis自动用内存缓存）

### 1.2 一键启动

```bash
# 进入项目目录
cd infotopograph

# 安装依赖
cd platform/mox-server
pip install -r requirements.txt

# 启动后端服务 (端口8600)
python run.py 8600
```

看到 `Uvicorn running on http://0.0.0.0:8600` 即启动成功。

### 1.3 访问前端

```bash
# 方式1: 直接用浏览器打开本地文件
# 企业官网: frontend-ui/mox-website/index.html
# 管理控制台: frontend-ui/mox-console/index.html
# 应用商店: frontend-ui/mox-store/index.html

# 方式2: 用Python起个静态服务器(推荐)
cd frontend-ui/mox-website
python -m http.server 8080
# 浏览器访问 http://localhost:8080
```

### 1.4 验证

```bash
# 后端健康检查
curl http://localhost:8600/api/health
# 返回 {"status":"ok"} 即正常
```

---

## 二、平台使用指南

### 2.1 管理控制台

访问 `frontend-ui/mox-console/index.html`，核心功能：

| 模块 | 功能 | 操作路径 |
|------|------|---------|
| **DSQL管理** | 动态SQL模板增删改查、版本管理、试运行 | 左侧菜单 → SQL管理 |
| **数据源** | 配置多数据库连接，测试连通性 | 左侧菜单 → 数据源 |
| **知识图谱** | 实体/关系管理，图谱可视化，多跳查询 | 左侧菜单 → 知识图谱 |
| **权限管理** | 角色/权限/字段权限配置 | 左侧菜单 → 权限 |
| **AI助手** | 自然语言生成SQL，SQL优化，业务流程生成 | 左侧菜单 → AI助手 |
| **内容管理** | 产品/新闻/案例/团队/FAQ等内容CRUD | 左侧菜单 → 内容 |

### 2.2 配置一条动态SQL（核心操作）

**步骤**：

1. 进入管理控制台 → SQL管理 → 新增
2. 填写：
   - **SQL编码**：`product_list`（全局唯一，前端调用用这个）
   - **数据源**：`default`（选已配置的数据源）
   - **缓存时间**：`60`秒（0=不缓存）
   - **SQL模板**：

```sql
SELECT id, name, category, price, image, summary
FROM products
WHERE 1=1
{% if category %} AND category = :category {% endif %}
{% if keyword %} AND (name LIKE :kw OR summary LIKE :kw) {% endif %}
ORDER BY id ASC
```

3. 点击「试运行」，输入参数测试
4. 保存后，前端通过 `POST /api/dsql/execute/product_list` 调用

**Jinja2模板语法速查**：

| 语法 | 说明 | 示例 |
|------|------|------|
| `{% if x %}` | 条件分支 | `{% if category %} AND category=:category {% endif %}` |
| `{% for x in list %}` | 循环 | `{% for id in ids %} id=:id_{{loop.index}} {% endfor %}` |
| `:param` | 参数占位符 | `WHERE id = :id`（自动参数化防注入） |
| `{{ var }}` | 变量输出 | `ORDER BY {{ sort_field }}` |

### 2.3 AI助手生成SQL

1. 进入AI助手
2. 输入自然语言：`查询价格大于1000的产品，按价格降序排列`
3. AI自动生成SQL模板，可编辑后保存
4. 支持一键试运行、优化建议、执行计划解释

### 2.4 配置字段级权限

1. 进入权限管理 → 字段权限
2. 选择角色 + 表名
3. 配置：
   - **可见字段**：勾选该角色能看到的列
   - **可写字段**：勾选该角色能修改的列
   - **脱敏字段**：勾选自动掩码的列（邮箱/电话/身份证）
4. 保存后，该角色的所有查询自动应用字段权限

---

## 三、数据导出与导入

### 3.1 一键导出

```bash
# 导出默认应用(mox)的全部数据(L1内核+L2业务+图谱)
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

**导出产物**：`exports/mox-export-{app_key}-{时间戳}.json`（MXDEF格式）

### 3.2 校验导出文件

```bash
python tools/validate_export.py exports/mox-export-mox-20260828-103000.json
```

检查项：JSON格式、format版本、checksum、记录数、外键完整性、脱敏、重复ID。

### 3.3 一键导入

```bash
# 幂等导入(基于唯一键upsert，重复导入不产生重复)
python tools/import_data.py exports/mox-export-mox-20260828-103000.json

# 预览模式(不写入，只显示将导入的记录数) — 推荐先跑
python tools/import_data.py export.json --dry-run

# 仅导入内核
python tools/import_data.py export.json --kernel-only

# 仅导入业务数据
python tools/import_data.py export.json --business-only

# 导入到指定应用(覆盖app_key)
python tools/import_data.py export.json --target-app new_corp

# 强制覆盖(忽略checksum校验失败)
python tools/import_data.py export.json --force

# 导入前清空目标表(危险，慎用)
python tools/import_data.py export.json --purge
```

### 3.4 典型场景

**场景A：新系统初始化**
```bash
# 1. 源系统导出
python tools/export_data.py --output ./exports
# 2. 校验
python tools/validate_export.py ./exports/mox-export-mox-*.json
# 3. 目标系统预览
python tools/import_data.py ./exports/mox-export-mox-*.json --dry-run
# 4. 正式导入
python tools/import_data.py ./exports/mox-export-mox-*.json
```

**场景B：应用迁移（A系统→B系统）**
```bash
# A系统导出指定应用
python tools/export_data.py --app-key corp_demo --business-only
# B系统导入(自动重新映射ID)
python tools/import_data.py corp_demo-export.json --target-app corp_demo
```

**场景C：内核升级（SQL模板/权限同步）**
```bash
# 开发环境导出内核
python tools/export_data.py --kernel-only
# 生产环境先预览
python tools/import_data.py kernel-export.json --kernel-only --dry-run
# 确认后导入(SQL模板自动版本+1可回滚)
python tools/import_data.py kernel-export.json --kernel-only
```

---

## 四、应用发布与安装

### 4.1 启动应用商店

```bash
# 启动商店服务 (端口8601)
cd platform/mox-store
python store_server.py
# 看到 "MOX Store running on http://0.0.0.0:8601" 即成功
```

### 4.2 发布应用到商店

**准备应用目录结构**：
```
my-app/
├── frontend/          # 前端静态文件(必须有index.html)
├── data/              # 初始化数据(MXDEF格式，可选)
├── icon.png           # 应用图标(可选)
└── README.md          # 说明文档(可选)
```

**一键发布**：
```bash
python tools/publish_app.py \
  --app-dir ./my-app \
  --app-key my-crm \
  --name "客户关系管理" \
  --version 1.0.0 \
  --author "你的公司" \
  --category "办公协同" \
  --description "企业级客户关系管理系统" \
  --tags "CRM,客户管理" \
  --upload --publish
```

**参数说明**：

| 参数 | 必填 | 说明 |
|------|------|------|
| `--app-dir` | 是 | 应用目录路径 |
| `--app-key` | 是 | 应用唯一标识(英文/数字/下划线) |
| `--name` | 是 | 应用显示名称 |
| `--version` | 否 | 版本号，默认1.0.0 |
| `--author` | 否 | 作者/公司 |
| `--category` | 否 | 分类(办公协同/数据分析/营销工具/行业方案/开发工具/其他) |
| `--description` | 否 | 简短描述 |
| `--tags` | 否 | 标签，逗号分隔 |
| `--upload` | 否 | 上传到商店 |
| `--publish` | 否 | 自动发布上架(需先--upload) |

**发布流程自动完成**：打包MXAP → 数字签名 → 上传商店 → 自动审核 → 上架

### 4.3 安装应用

**方式1：命令行安装**
```bash
# 从商店安装
python tools/install_app.py --app-key my-crm

# 从本地MXAP包安装
python tools/install_app.py --file ./exports/my-crm-1.0.0.mxap

# 查看已安装
python tools/install_app.py --list

# 更新
python tools/install_app.py --app-key my-crm --update

# 卸载
python tools/install_app.py --app-key my-crm --uninstall
```

**方式2：应用商店前端安装**
1. 打开 `frontend-ui/mox-store/index.html`
2. 浏览或搜索应用
3. 点击应用卡片 → 「安装应用」
4. 安装后在「我的应用」查看管理

### 4.4 应用商店前端使用

访问 `frontend-ui/mox-store/index.html`，三个视图：

| 视图 | 路由 | 功能 |
|------|------|------|
| **浏览** | `#/` | 应用卡片列表、分类筛选、搜索、排序、详情弹框、一键安装 |
| **我的应用** | `#/my` | 已安装应用列表、运行状态、卸载 |
| **发布** | `#/publish` | 发布命令参考、MXAP格式说明 |

---

## 五、独立部署与交付

### 5.1 导出为独立Docker包

将应用导出为脱离MOX的独立运行包，接收方开箱即用：

```bash
python tools/export_standalone.py \
  --app-key my-crm \
  --name "客户关系管理" \
  --version 1.0.0 \
  --description "企业级CRM系统" \
  --output ./dist \
  --port 8600
```

**产物**：
- `dist/my-crm-standalone-1.0.0/` — 完整目录
- `dist/my-crm-standalone-1.0.0.tar.gz` — 压缩包

**接收方使用**：
```bash
tar xzf my-crm-standalone-1.0.0.tar.gz
cd my-crm-standalone-1.0.0
docker-compose up -d --build
# 访问 http://localhost:8600
```

**独立包内含**：MOX运行时(后端) + 应用前端 + 初始化数据 + Dockerfile + docker-compose.yml + 启动脚本 + README

### 5.2 平台整体打包

```bash
# 打包整个MOX平台(含数据)
python tools/package.py --version 1.0.0 --format tar.gz --with-data

# 产物: dist/mox-platform-1.0.0-{时间戳}.tar.gz
```

### 5.3 一体化部署

```bash
# 本地一键部署(安装依赖+初始化+导入数据+启动)
python tools/deploy.py local --data exports/mox-export-mox-*.json --start

# Docker一键部署
python tools/deploy.py docker
```

---

## 六、域名绑定与生产上线

### 6.1 域名方案选择

| 方案 | 格式 | 优点 | 缺点 | 推荐场景 |
|------|------|------|------|---------|
| **子域名** | `crm.yourcompany.com` | 独立SSL、独立部署、隔离好 | 需配DNS解析 | 生产系统、多应用 |
| **子路径** | `yourcompany.com/crm` | 一个域名搞定、共用SSL | 需Nginx路由配置、前端base路径 | 简单展示、单应用 |
| **独立域名** | `crm-system.com` | 完全独立、品牌感强 | 需额外备案 | 对外产品、SaaS |

**推荐**：企业内部多系统用**子域名**，简单高效。

### 6.2 子域名部署步骤

```bash
# 1. DNS解析: 添加A记录 crm.yourcompany.com → 服务器IP
#    (在域名服务商控制台操作，如阿里云/腾讯云/Cloudflare)

# 2. 服务器上部署MOX(见第五章)
cd /opt && tar xzf /tmp/mox-platform-1.0.0-*.tar.gz
cd mox-platform-1.0.0-*
docker-compose up -d --build

# 3. 配置Nginx反向代理
sudo cp deploy/nginx.conf /etc/nginx/conf.d/crm.conf
# 编辑nginx.conf，将server_name改为crm.yourcompany.com
sudo sed -i 's/your-domain.com/crm.yourcompany.com/g' /etc/nginx/conf.d/crm.conf

# 4. 申请免费SSL证书
sudo apt install certbot python3-certbot-nginx
sudo certbot --nginx -d crm.yourcompany.com

# 5. 重载Nginx
sudo nginx -t && sudo systemctl reload nginx

# 6. 验证
curl https://crm.yourcompany.com/api/health
# 浏览器访问 https://crm.yourcompany.com
```

### 6.3 Nginx配置模板

```nginx
server {
    listen 80;
    server_name crm.yourcompany.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name crm.yourcompany.com;

    ssl_certificate     /etc/letsencrypt/live/crm.yourcompany.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/crm.yourcompany.com/privkey.pem;

    # 前端静态文件
    root /var/www/mox-website;
    index index.html;

    # 后端API反向代理
    location /api/ {
        proxy_pass http://127.0.0.1:8600/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 60s;
    }

    # SPA路由回退
    location / {
        try_files $uri $uri/ /index.html;
    }

    # 静态资源缓存
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff2?)$ {
        expires 30d;
        add_header Cache-Control "public, immutable";
    }

    # 安全头
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
}
```

### 6.4 ICP备案（中国大陆必填）

1. 在服务器提供商（阿里云/腾讯云等）提交ICP备案
2. 备案通过后，在网站底部展示备案号：
```html
<a href="https://beian.miit.gov.cn" target="_blank">粤ICP备XXXXXXXX号</a>
```

---

## 七、运维与监控

### 7.1 服务管理

```bash
# 查看服务状态
curl http://localhost:8600/api/health

# 查看商店状态
curl http://localhost:8601/api/store/health

# Docker方式
docker-compose ps
docker-compose logs -f mox-server
docker-compose restart mox-server

# Systemd方式
sudo systemctl status mox-server
sudo systemctl restart mox-server
sudo journalctl -u mox-server -f
```

### 7.2 数据备份

```bash
# 备份数据库(SQLite)
cp platform/mox-server/data/mox_business.db backup/mox_business_$(date +%Y%m%d).db

# 备份为MXDEF(推荐，可跨版本迁移)
python tools/export_data.py --output ./backup --gzip

# 定时备份(crontab)
# 每天凌晨2点备份
# 0 2 * * * cd /opt/mox && python tools/export_data.py --output ./backup --gzip
```

### 7.3 日志查看

```bash
# 后端日志
tail -f platform/mox-server/mox_server.log

# 商店日志
tail -f platform/mox-store/store.log

# Docker日志
docker-compose logs -f --tail=100 mox-server
```

### 7.4 性能监控

```bash
# 查看慢查询(在管理控制台 → AI助手 → SQL优化)
# 查看缓存命中率
curl http://localhost:8600/api/admin/cache-stats

# 查看系统资源
docker stats
```

---

## 八、常见问题排查

### Q1: 后端启动但前端连不上API？

**排查**：
1. 确认后端在运行：`curl http://localhost:8600/api/health`
2. 检查前端API_BASE配置：file://协议自动用 `http://127.0.0.1:8600`
3. 生产环境确认Nginx配置了 `/api/` 反向代理
4. 浏览器F12查看Network面板，看API请求是否报错

### Q2: 导入数据后管理中心看不到内容？

**排查**：
1. 检查app_key是否匹配：导出时 `--app-key` 和导入时 `--target-app` 需一致
2. 前端URL的 `?app=xxx` 参数需与app_key匹配
3. 确认导入命令没有 `--dry-run`（预览模式不写入）
4. 查看导入输出的记录数是否>0

### Q3: Docker容器内无法连接Redis？

**解决**：docker-compose.yml中 `MOX_REDIS` 应使用服务名 `redis://redis:6379/0`，不是localhost。

### Q4: 如何更新SQL模板而不影响业务数据？

```bash
# 只导出内核(SQL模板/权限/数据源)
python tools/export_data.py --kernel-only
# 只导入内核，业务数据不受影响
python tools/import_data.py kernel.json --kernel-only
# SQL模板自动版本+1，可回滚到旧版本
```

### Q5: 应用商店安装失败？

**排查**：
1. 确认商店服务在运行：`curl http://localhost:8601/api/store/health`
2. 确认MXAP包格式正确（用 `python tools/publish_app.py` 重新打包）
3. 检查磁盘空间是否充足
4. 查看商店日志：`tail -f platform/mox-store/store.log`

### Q6: 前端页面空白？

**排查**：
1. 浏览器F12查看Console是否有JS错误
2. 确认index.html路径正确，用 `python -m http.server` 启动而非直接file://（某些浏览器限制）
3. 确认API服务可访问（空白通常是API请求失败导致渲染异常）

### Q7: 如何切换数据库（SQLite→MySQL）？

1. 在管理控制台 → 数据源 → 新增MySQL数据源
2. 测试连通性
3. 将SQL模板的数据源从 `default` 改为新数据源
4. 或修改 `default` 数据源的连接配置为MySQL
5. 数据迁移用MySQL官方工具或ETL

### Q8: 字段权限不生效？

**排查**：
1. 确认用户角色已绑定字段权限配置
2. 确认SQL模板通过DSQL引擎执行（直接写死的SQL不经过权限过滤）
3. 清除Redis缓存（权限变更后缓存可能未失效）
4. 重新登录获取新的权限token

---

## 附录：命令速查表

### 服务启动
```bash
python platform/mox-server/run.py 8600          # 核心后端
python platform/mox-store/store_server.py         # 应用商店
python -m http.server 8080 -d frontend-ui/mox-website  # 前端静态
```

### 数据管理
```bash
python tools/export_data.py                         # 导出
python tools/import_data.py file.json               # 导入
python tools/validate_export.py file.json           # 校验
```

### 应用商店
```bash
python tools/publish_app.py --app-dir ./app --app-key xxx --upload --publish  # 发布
python tools/install_app.py --app-key xxx           # 安装
python tools/install_app.py --list                   # 已安装列表
python tools/export_standalone.py --app-key xxx     # 独立导出
```

### 部署
```bash
python tools/deploy.py local --start                 # 本地部署
python tools/deploy.py docker                        # Docker部署
python tools/package.py --version 1.0.0              # 平台打包
```

---

*本手册覆盖MOX平台从安装到上线的全流程操作。如有问题，先查第八章常见问题，再查看日志。*
