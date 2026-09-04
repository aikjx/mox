# MOX mox 模块化系统架构低代码平台

> 企业级动态SQL管理 + 自研知识图谱 + 字段级权限 + AI驱动
> 版本: 1.0.0 | 状态: 生产就绪

---

## 快速开始

```bash
# 1. 启动后端
cd platform/mox-server
pip install -r requirements.txt
python run.py 8600

# 2. 打开前端
# 浏览器打开: frontend-ui/mox-website/index.html
# 管理中心: #/admin
```

## 一键部署

```bash
# Docker一体化部署(推荐)
docker-compose up -d --build

# 本地一体化部署
python tools/deploy.py local --start

# 打包发布
python tools/package.py --version 1.0.0 --with-data
```

## 数据导出/导入

```bash
# 导出全部数据
python tools/export_data.py

# 仅导出内核(SQL模板/权限配置)
python tools/export_data.py --kernel-only

# 导入数据(先预览)
python tools/import_data.py export.json --dry-run
python tools/import_data.py export.json

# 校验导出文件
python tools/validate_export.py export.json
```

## 目录结构

```
infotopograph/
├── platform/mox-server/   # 后端服务(FastAPI)
├── frontend-ui/
│   ├── mox-website/        # 企业官网SPA
│   └── mox-console/        # 管理控制台SPA
├── tools/                   # 运维工具(导出/导入/校验/打包/部署)
├── docs/                    # 文档(架构/数据交换/部署)
├── deploy/                  # 部署配置(Docker/Nginx/Systemd)
├── docker-compose.yml       # Docker编排
└── README.md
```

## 文档

- [架构设计](docs/architecture.md) — mox 模块化系统架构、三层数据、DSQL引擎、知识图谱、权限体系
- [数据交换规范](docs/data-exchange-spec.md) — MXDEF格式、导出/导入命令、跨系统发布
- [部署指南](docs/deployment-guide.md) — 本地/Docker/Nginx/Systemd/一体化部署流程

## 核心特性

- **动态SQL管理**: 所有业务SQL在数据库中配置，Jinja2模板渲染，参数化查询
- **多数据库支持**: SQLite/MySQL/PostgreSQL，修改数据源配置即切换
- **自研知识图谱**: 实体/关系模型，多跳遍历，与SQL融合查询
- **字段级权限**: RBAC + 列级可见/可写 + 自动脱敏
- **AI智能助手**: 自然语言生成SQL，解释优化，一键试运行
- **三层数据分离**: L1内核/L2业务/L3运行时，一键导出导入，跨系统发布
- **企业级标准**: 响应式设计、SEO优化、安全防护、审计日志
