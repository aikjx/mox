# scripts/ — 璇玑系统统一运维脚本目录

## 目录结构

```
scripts/
├── README.md              # 本文件（布局索引）
├── service/               # 【主入口】服务生命周期管理
│   ├── server-manage.py   # 统一运维脚本（服务生命周期 + Web 面板 + 公理验证）
│   └── manage.py          # 【兼容别名】转发到 server-manage.py（防历史命令断链）
├── startup/               # 一键启停（生命周期编排）
│   ├── start-all.ps1              # 一键启动全部服务 + 管理面板
│   ├── stop-all.ps1               # 一键停止全部服务
│   ├── start-mox-enterprise.ps1    # 企业级四进程一键启动（编排器/联盟调度/执行/网关）
│   ├── stop-mox-enterprise.ps1     # 企业级四进程一键停止
│   ├── start-alliance-local.ps1    # 联盟本地实例一键启动
│   └── stop-alliance-local.ps1     # 联盟本地实例一键停止
├── gate/                  # CI / 质量门禁 / 校验
│   ├── verify-ports.py            # 全局端口漂移校验（PORT-REGISTRY-001）
│   ├── check-doc-links.py         # 文档链接校验（DOC-GOV-ARC-V1.0）
│   ├── check-forbidden-terms.py   # 禁用词检查
│   ├── check-all.ps1              # 一键构建 + 测试 + clippy（全量健康检查）
│   └── ci-gate.ps1                # CI 门禁（本地复现流水线关键检查）
├── doc/                   # 文档 / 注册表生成与核对
│   ├── gen-api-registry.py        # 生成 docs/API-REGISTRY.md
│   └── verify-doc-ep038.py        # EP-038 文档/代码事实自动核对
├── registry/              # 仓库元信息注册表
│   ├── module_catalog.py          # 模块目录清单
│   └── ownership-registry.py      # 归属 / 责任矩阵
├── setup/                 # 开发环境初始化
│   └── setup-dev.ps1              # 本地开发环境初始化（依赖、配置、目录）
├── deploy/                # 部署 / 打包 / 启动脚本
│   ├── deploy.py                  # MOX 一体化部署工具（本地 / Docker / 远程）
│   ├── package.py                 # MOX 一键打包工具（生成可分发发布包）
│   ├── start.ps1                  # Windows 一键启动（复用 bootstrap）
│   ├── smoke_test.sh              # 冒烟测试
│   ├── deploy-kg-storage.ps1      # KG 存储服务部署
│   └── Gray-Warmup.ps1            # 灰度预热
├── ci/                    # CI/CD 脚本
│   ├── ci.py
│   ├── git_create_tag_auto.py
│   └── secret-scan.py
├── validation/            # 验证 / 校验脚本
│   ├── normalization-scan.py      # 归一化扫描
│   ├── validate-single-node.js
│   ├── validate_rust_workspace_deps.js
│   ├── verify_tts_rust_fullstack.py
│   ├── tts_rust_vs_py_dsp_regression.py
│   └── fix_pass2.py
├── maintenance/           # 一次性 / 历史重构脚本（按领域分组，仅供追溯）
│   ├── _fix_lifecycle_state.py    # 插件生命周期状态迁移（一次性修复）
│   ├── ren.py                     # 通用文件改名工具
│   ├── gateway/           # 网关层：alliance / actuator / proxy 路由修复
│   ├── cloud/             # 云存储：redis 连接修复、s3 上传、依赖注入
│   ├── kg/                # 知识图谱：storage / fusion / kb-svc 重构
│   ├── protocol/          # API 协议：ApiResponse.message→msg 改名 + 文档同步
│   ├── enterprise/        # 企业级 smoke 测试修复（括号/返回值类型）
│   ├── frontend/          # 前端：清理 mock 占位、专家中心/监控面板接真实数据
│   ├── workspace/         # 架构分析 + 工作区成员清单
│   └── probes/            # 临时探测/示例脚本（CosyVoice/TTS/Sherpa 等，可随时清理）
└── tests/                 # 测试执行脚本
    ├── Run-T10-AllTests.ps1
    ├── Run-T11-AllTests.ps1
    ├── Run-T17-EF-All.ps1
    ├── Run-T17-SDK-All.ps1
    ├── Run-T19-Regression-706.ps1
    ├── run-t1-baseline.ps1
    ├── run-enterprise-final-acceptance.ps1
    ├── run_enterprise_7gates.ps1
    ├── verify_tests.ps1
    ├── verify_tests.sh
    └── parse_test_report.py
```

> 约定：所有带 `_` 前缀的脚本均为**一次性历史重构脚本**，内含硬编码绝对路径，
> 仅用于追溯改动来源；请勿重复执行。新功能请走正式入口或对应子目录。

## 快速使用

### 查看所有服务
```bash
python scripts/service/server-manage.py list
```

### 查看全量项目目录
```bash
python scripts/service/server-manage.py list-projects
```

### 查看脚本目录索引
```bash
python scripts/service/server-manage.py scripts
```

### 启动/停止服务
```bash
python scripts/service/server-manage.py start xiaobai_voice
python scripts/service/server-manage.py stop all --force
python scripts/service/server-manage.py restart api
```

### 启动 Web 管理面板
```bash
python scripts/service/server-manage.py dashboard --port 3999
```

### 一键启动（预检 → 清残留 → 按拓扑启动 → 面板）
```bash
python scripts/service/server-manage.py bootstrap --with-dashboard
```

## 已注册服务（5 个）

| 服务 Key | 名称 | 端口 | 类型 | 自动启动 |
|----------|------|------|------|----------|
| xiaobai_voice | 小白语音服务（ASR + TTS） | 30010 | Python | ✅ |
| api | API 后端服务（Rust mox-gateway） | 3080 | Rust | ❌ |
| frontend | 用户前端界面（Vite + Vue3） | 3020 | Node | ❌ |
| melody2score | 旋律转谱服务（WebUI） | 8012 | Python | ❌ |
| primiflow | PrimiFlow 低代码拓扑引擎 | 8000 | Python | ❌ |

## 项目目录清单（20 项）

通过 `python scripts/service/server-manage.py list-projects` 查看全量清单，包含：
- **5 个可启动服务**（core_platform, frontend_ui, xiaobai_voice, melody2score, primiflow）
- **2 个库/SDK**（mox_dualrpc, business_court_docs）
- **13 个测试产物目录**（t10~t25, market-games, vendor-eval 等）

## 配置文件

所有服务配置和项目目录清单存储在仓库根的 `platform_config.json`：
- `services` — 服务定义（启动命令、端口、依赖、标签）
- `project_registry` — 全量项目目录清单
- `script_catalog` — scripts/ 目录分类索引

新增服务只需在 `platform_config.json` 的 `services` 中添加条目，`server-manage.py` 自动识别。
