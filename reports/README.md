# Reports / 报告产出物

本目录存放项目的各类报告产出物。

## 目录结构

```
reports/
├── html/              # HTML 可视化报告（交互式、带图表）
│   ├── _shared/       # ⚠️ 不存在，共享资源在上一级 reports/_shared/
│   ├── kg-centric-architecture/    # 知识图谱中心架构报告
│   ├── directory-audit-report/      # 目录审计报告
│   └── code-evaluation-report/      # 代码评估报告
├── markdown/          # Markdown 格式报告
│   ├── P0P1-优化专题报告-V1.1.md
│   ├── ai_benchmark_report.md
│   ├── 质量联盟总报告-V1.0.md
│   └── ...
├── data/              # 报告数据文件（JSON、日志等原始数据）
│   ├── *.json
│   ├── *.log
│   └── ...
└── _shared/           # HTML 报告共享资源
    ├── fonts/         # 字体文件（InstrumentSans、JetBrainsMono、ArsenalSC 等）
    └── js/            # JS 库（echarts、mermaid）
```

## 放置规则

| 内容类型 | 放置位置 | 说明 |
|---------|---------|------|
| HTML 可视化报告 | `html/<报告名>/` | 自带 `assets/`（独有资源），共享资源引用 `../_shared/` |
| Markdown 报告 | `markdown/` | 文档类报告、分析报告、验收报告 |
| 报告数据 | `data/` | JSON 结果、日志、基准测试数据等 |
| 共享字体/JS | `_shared/` | 所有 HTML 报告共用，不要各自复制 |

## 新增 HTML 报告规范

1. 在 `html/` 下新建目录（命名：`<报告名>-report` 或 `<报告名>`）
2. 报告 HTML 文件放在该目录根
3. 独有资源（如 `assets/charts.js`）放在项目内 `assets/` 目录
4. **字体和 echarts/mermaid 等公共库必须引用 `../_shared/`**，不要自带副本
5. 引用示例：
   ```html
   <link rel="stylesheet" href="../_shared/fonts/...">
   <script src="../_shared/js/echarts.min.js"></script>
   <script src="../_shared/js/mermaid.min.js"></script>
   ```

## 与 docs/ 的区别

- `reports/` — 产出物（报告、审计结果、评估数据），有明确的"发布"属性
- `docs/` — 过程文档（需求、设计、架构、ADR、工作汇报），偏向记录和说明
