# Prototypes / 原型演示项目

本目录存放 MOX 平台的 HTML 原型、演示页面和设计稿。

> ⚠️ 注意：这些是原型/演示项目，**不是生产代码**。
> 生产代码位于 `platform/`（后端）和 `frontend-ui/`（前端）。

## 项目列表

| 项目 | 说明 |
|------|------|
| `chat-project-generator/` | 对话式项目生成器原型 |
| `data-vis/` | 全维分析流程可视化原型 |
| `expert-alliance-cyber/` | 专家联盟赛博风格设计原型 |
| `expert-alliance-design/` | 专家联盟设计系统原型 |
| `kg-workflow-guide/` | 知识图谱工作流向导原型 |
| `mox-enterprise-optimization/` | MOX 企业级优化展示原型 |

## 共享资源

所有原型共享统一的 `prototypes/_shared/` 目录（字体、echarts、mermaid 等）。
各原型 HTML 通过 `../_shared/` 相对路径引用共享资源。

**新增共享资源规则：**
- 公共字体 → 放入 `_shared/fonts/`
- 公共 JS 库 → 放入 `_shared/js/`
- 各原型独有的资源（如 assets/charts.js）→ 放在各自项目目录内，不要放入 `_shared/`
- **不要在各原型目录内新建 `_shared/` 副本**，统一用根级 `_shared/`

## 维护规范

1. 原型项目不进入生产部署流程
2. 原型验证完成后，代码应迁移到 `frontend-ui/` 或 `platform/`
3. 废弃的原型在 30 天后清理
