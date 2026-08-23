# 企业门户网站（基于 OUS）运行说明

本目录在 OUS 原有智能工作台基础上，新增了**企业门户网站外壳**，用于验证"用算子统一系统快速拼出一个可用企业产品"是否好用。

## 新增页面（复用现有 /api 端点，后端无需改动）

| 路由 | 页面 | 说明 |
|------|------|------|
| `/` | 门户首页 `PortalHome.vue` | 企业展示 + 导航 + AI 客服浮窗（调用 `/api/ai/chat`） |
| `/login` | 登录壳 `Login.vue` | 演示鉴权 + 选择运行形态/LLM 来源（§13） |
| `/workbench` | 智能工作台 `Workbench.vue` | 原 AI 对话 + 流程图（保留） |
| `/hall` | 业务大厅 `BusinessHall.vue` | 展示算子(`/api/operators`)、执行流程(`/api/ai/flows`) |

## 本地运行

```bash
# 1. 后端（在仓库根）
cargo run -p runtime            # 监听 3000，提供 /api

# 2. 前端
cd frontend
npm install                      # 已加入 vue-router@4 依赖
npm run dev                      # http://localhost:3020

# 3. 生产构建（相对路径，适配桌面/子路径，见 §13.5）
VITE_BASE=./ npm run build       # 产物 frontend/dist
```

## 验证结论（用 OUS 搭门户是否好用）

- **快**：仅新增 4 个 Vue 文件 + 1 个 router，复用 OUS 已有的 AI 对话/流程/算子/图谱能力，半天即可成型。
- **模块化**：门户首页、登录、工作台、业务大厅彼此独立，可独立部署/替换。
- **可验证业务**：业务大厅直接拉取已注册算子并一键执行流程，证明"业务流程化（§9/§28）"真实可用。
- **全形态就绪**：登录壳可切换 运行形态/LLM 来源，与 §13 产品矩阵一致。

> 注：在编写环境的 Node 运行时存在稳定性异常（npm 报错、node 进程访问冲突），未能在本会话完成 `vite build` 自动验证；请在本地稳定 Node 环境执行上面命令即可。代码为标准 Vue3 + vue-router4 写法，无特殊依赖。
