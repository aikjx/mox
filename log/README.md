# `log/` — 标准产物归档目录

本目录统一收纳 OUS 工程运行期生成的**一次性产物**（lint / 测试 / 合规 / 图导出）。
已加入 `.gitignore`（`/log/`），不入库；仅保留 `.gitkeep` 与本文档维持目录结构。

## 目录结构

```
log/
├── README.md                  # 本文件：归档说明 + 各产物溯源
├── .gitkeep
├── clippy/                    # cargo clippy 全工作区 lint 报告
│   ├── clippy_report_1.txt    # 首次全工作区 clippy 抓取（含依赖 Checking 阶段）
│   ├── clippy_report_2.txt    # 单 crate clippy 抓取（mox-system 等告警详情）
│   ├── clippy_report_3.txt    # 多 crate 增量 clippy 抓取
│   ├── clippy_report_4.txt    # 全工作区 clippy 终态（runtime 收尾 + 汇总）
│   ├── clippy_run.log         # clippy 运行日志（编译阶段流水）
│   └── tmp_clippy.log         # 临时 clippy 日志（primiflow-core example 编译失败残留）
├── test/                      # cargo test 全工作区测试运行日志
│   ├── enterprise_test.log    # 全工作区最终回归（734 passed / 0 failed / 6 ignored）
│   ├── test_all.txt           # 全量测试抓取（多 crate 编译+运行流水）
│   ├── test_final.txt         # 终态测试抓取
│   ├── test_log.txt           # 依赖编译阶段日志
│   ├── test_aiagent_baseline.txt  # ai-agent crate 单测基线（workflow/requirement 用例）
│   ├── test_syslog.txt        # 测试运行 syslog 阶段（依赖编译）
│   ├── test_syslog2.txt       # 测试运行 syslog 阶段（mox-system 收尾）
│   ├── test_xs.txt            # 测试运行（依赖编译阶段 xs 子集）
│   ├── test_xs2.txt           # mox-system 测试收尾（Finished test profile）
│   └── test_xs3.txt           # mox-system 测试收尾（Finished test profile，二次）
├── compliance/                # 合规/治理测试
│   ├── test-tr-4-compliance.js        # T4 依赖治理合规脚本（根目录散落副本，412 行变体）
│   └── test-tr-4-compliance.out.json  # T4 合规输出（17 个 Cargo.toml 依赖扫描结果）
└── graph/                     # 知识图谱 / 架构图导出
    └── graph.enterprise.json  # 企业级图谱导出（nodes/edges，CI 门禁生成，基线 .guantu_baseline.json 需入库）
```

## 溯源：各产物如何生成

| 产物 | 生成命令 | 说明 |
|------|----------|------|
| `clippy/*` | `cargo clippy --workspace` | 全工作区 lint 门禁（lib+bins） |
| `test/*` | `cargo test --workspace --no-fail-fast` | 全工作区回归测试（含 e2e/enterprise/gap） |
| `compliance/test-tr-4-compliance.js` | 手搓 node 脚本 | T4 依赖治理：TR 4.1 外部 crate 行数 ≤1 / TR 4.2 reqwest 版本 0.12.x |
| `compliance/*.out.json` | `node test-tr-4-compliance.js` | 上述脚本输出 |
| `graph/graph.enterprise.json` | 图谱导出工具 | 企业级知识图谱（CI 关图门禁生成物） |

## 规范约定

1. **命名**：`{类别}_{描述}.{ext}`，同一命令多次运行加 `_N` 序号（如 `clippy_report_2.txt`）。
2. **不入库**：整个 `/log/` 在 `.gitignore` 中忽略；需要长期留存的基线（如 `.guantu_baseline.json`）放在仓库根而非本目录。
3. **清理**：本目录为一次性产物，可随时 `rm -rf log/*` 重建；勿将源码/配置放入。
4. **合规脚本**：`test-tr-4-compliance.js` 的权威版本位于 `platform/backend-node/test/`（286 行）；根目录这份 412 行变体为散落副本，已归档至此供回溯，不覆盖权威版。
