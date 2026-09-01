# scripts/ — 璇玑系统统一运维脚本目录

## 目录结构

```
scripts/
├── server-manage.py       # 【主入口】统一运维脚本（服务生命周期 + Web 面板 + 公理验证）
manage.py              # 【兼容别名】转发到 server-manage.py（防历史命令断链）
├── README.md              # 本文件
├── ci/                    # CI/CD 脚本
│   ├── ci.py
│   └── git_create_tag_auto.py
├── deploy/                # 部署/启动脚本
│   ├── start.ps1          # Windows 一键启动
│   ├── smoke_test.sh      # 冒烟测试
│   ├── deploy-kg-storage.ps1  # KG 存储服务部署
│   └── Gray-Warmup.ps1    # 灰度预热
├── tests/                 # 测试执行脚本
│   ├── Run-T10-AllTests.ps1
│   ├── Run-T11-AllTests.ps1
│   ├── Run-T17-EF-All.ps1
│   ├── Run-T17-SDK-All.ps1
│   ├── Run-T19-Regression-706.ps1
│   ├── run-t1-baseline.ps1
│   ├── run-enterprise-final-acceptance.ps1
│   ├── run_enterprise_7gates.ps1
│   ├── verify_tests.ps1
│   ├── verify_tests.sh
│   └── parse_test_report.py
├── validation/            # 验证/校验脚本
│   ├── validate-single-node.js
│   ├── validate_rust_workspace_deps.js
│   ├── verify_tts_rust_fullstack.py
│   ├── tts_rust_vs_py_dsp_regression.py
│   └── fix_pass2.py
└── temp/                  # 临时/探测脚本（可随时清理）
    ├── _auto_dl_cosy2_weights.py
    ├── _check_cosy_env.py
    ├── _probe_voice.py
    ├── _run_e2b.py
    ├── _run_e4.py
    └── _tmp_t10_doc_check.js
```

## 快速使用

### 查看所有服务
```bash
python scripts/server-manage.py list
```

### 查看全量项目目录
```bash
python scripts/server-manage.py list-projects
```

### 查看脚本目录索引
```bash
python scripts/server-manage.py scripts
```

### 启动/停止服务
```bash
python scripts/server-manage.py start xiaobai_voice
python scripts/server-manage.py stop all --force
python scripts/server-manage.py restart api
```

### 启动 Web 管理面板
```bash
python scripts/server-manage.py dashboard --port 3999
```

### 一键启动（预检 → 清残留 → 按拓扑启动 → 面板）
```bash
python scripts/server-manage.py bootstrap --with-dashboard
```

## 已注册服务（5 个）

| 服务 Key | 名称 | 端口 | 类型 | 自动启动 |
|----------|------|------|------|----------|
| xiaobai_voice | 小白语音服务（ASR + TTS） | 30010 | Python | ✅ |
| api | API 后端服务（Rust mox-gateway） | 8080 | Rust | ❌ |
| frontend | 用户前端界面（Vite + Vue3） | 3020 | Node | ❌ |
| melody2score | 旋律转谱服务（WebUI） | 8012 | Python | ❌ |
| primiflow | PrimiFlow 低代码拓扑引擎 | 8000 | Python | ❌ |

## 项目目录清单（20 项）

通过 `python scripts/server-manage.py list-projects` 查看全量清单，包含：
- **5 个可启动服务**（core_platform, frontend_ui, xiaobai_voice, melody2score, primiflow）
- **2 个库/SDK**（mox_dualrpc, business_court_docs）
- **13 个测试产物目录**（t10~t25, market-games, vendor-eval 等）

## 配置文件

所有服务配置和项目目录清单存储在仓库根的 `platform_config.json`：
- `services` — 服务定义（启动命令、端口、依赖、标签）
- `project_registry` — 全量项目目录清单
- `script_catalog` — scripts/ 目录分类索引

新增服务只需在 `platform_config.json` 的 `services` 中添加条目，`server-manage.py` 自动识别。
