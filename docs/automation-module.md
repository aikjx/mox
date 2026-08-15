# AI 自动化中枢（AI Automation Hub）

> 需求驱动的端到端闭环：对话 → 业务处理流程图 + 功能逻辑细节 + 关联关系 + 权限 → 自动代码 → 自动测试 → 沙箱实跑异常自动修复 → 回写保存 → 可继续编辑。

## 解决的问题

用户用一句话描述业务（如"做一个商城，有商品、购物车、下单、支付、退货"），系统自动：

1. **生成业务处理流程图**：由 `ai-agent::requirement_compiler` 把需求编译为 `SystemBlueprint`（功能点 + 实体 + 数据流依赖网 + 流程图 `FlowDefinition`）。
2. **生成全维度处理逻辑细节**：每个功能点渲染为带 `try/except` 兜底的 Python 函数；所有关联关系（实体字段、依赖顺序）落到流程图连线与 SQL 建表。
3. **自动推导权限（RBAC）**：从功能点的"动作 × 实体"推导角色-权限映射（如「下单」→ `order:create`），`customer`/`merchant`/`admin` 多角色，可直接被 `runtime::rbac_middleware` 消费。
4. **自动生成自动化代码**：Python 主流程 + SQL DDL + Vue 前端骨架，落盘为可编辑资产。
5. **自动测试**：针对生成代码生成冒烟测试（`AutoTestGen`）。
6. **异常自动分析修复并回写**：生成的 Python 在受控沙箱（可配置 `OUS_PYTHON`）实跑，捕获 traceback 后：
   - 规则兜底：对 `KeyError`/`ImportError`/`ZeroDivisionError` 等生成确定性补丁（`.get(..., None)`、`try/except ImportError`、分母判零）。
   - LLM 兜底：规则未命中且配置了大模型时，调用 `ai_agent::LLMClient` 生成修复代码。
   - 修复后**回写**到流程图 `Script` 节点与代码资产，并重新实跑验证。
7. **保存在 AI 自动化里，可继续编辑**：资产持久化到 `$OUS_HOME/automation/<id>.json`，前端对话页支持"继续对话迭代"（refine）与直接编辑代码/流程图并回写。

## 架构

```
requirements_compiler (ai-agent)
        │  SystemBlueprint (功能点/实体/流程图)
        ▼
generate_code_from_blueprint (automation.rs) ──► GeneratedCode (py/sql/vue)
RbacDeriver (flow-ai::automation)            ──► RolePermission[]
AutoTestGen (flow-ai::automation)            ──► AutoTest[]

        ▼ 落盘到 automation_asset.rs (持久化)

   POST /api/automation/chat  生成
   POST /api/automation/:id/refine  继续对话迭代
   PUT  /api/automation/:id  保存前端编辑（代码/流程图）
   POST /api/automation/:id/run  沙箱实跑 + 异常自动修复回写
   GET  /api/automation/:id/permissions  查看推导的 RBAC
   GET  /api/automation  列出资产
```

> 为避免 `automation` 与 `market` 形成循环依赖，共享资产模型与持久化抽到独立的 `automation_asset` 模块（单向被依赖）。

## 关键文件

| 文件 | 职责 |
|------|------|
| `crates/runtime/src/automation.rs` | 编排 + REST API + 中文标识符映射 + 沙箱实跑 + 修复回写 |
| `crates/runtime/src/automation_asset.rs` | `AutomationAsset` 模型 + 文件持久化（独立模块，防循环依赖） |
| `crates/flow-ai/src/automation.rs` | 纯逻辑：`RbacDeriver` / `ErrorAnalyzer` / `AutoTestGen` / `patch_flow_with_fix` |
| `crates/ai-agent/src/requirement_compiler.rs` | 需求 → 蓝图 → 流程图 |
| `frontend/src/views/AutomationView.vue` | 对话式 UI（流程图/Mermaid/权限表/代码编辑/实跑修复） |

## 运行配置

| 环境变量 | 说明 | 默认 |
|----------|------|------|
| `OUS_PYTHON` | 沙箱 Python 解释器命令 | `python3` |
| `OUS_HOME` | 资产存储根目录 | `.ous` |
| `OUS_API_TOKEN` | 受保护接口的 Bearer Token | 无（启用鉴权时必填） |

## 异常修复分类覆盖

规则兜底：`KeyError` / `ImportError` / `ZeroDivisionError`。
LLM 兜底：其余类别（NameError / TypeError / ValueError / SyntaxError 等）由大模型生成修复代码。
环境错误（`OUS_PYTHON` 缺失，exit_code 9009）不触发代码修复，仅提示配置。
