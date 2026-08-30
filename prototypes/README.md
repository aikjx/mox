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

各项目通过相对路径引用 `_shared/` 目录下的字体和 JS 库。
如需新增共享资源，请放入各项目的 `_shared/` 目录。

## 维护规范

1. 原型项目不进入生产部署流程
2. 原型验证完成后，代码应迁移到 `frontend-ui/` 或 `platform/`
3. 废弃的原型在 30 天后清理
