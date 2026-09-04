# AIS / AI 工具参考仓库

本目录存放 MOX 平台调研和参考的第三方 AI 工具、开源项目代码。

> ⚠️ **重要：本目录不纳入 Git 版本控制**
> 
> `ais/` 已在 `.gitignore` 中排除。所有子目录均为独立的 Git 仓库，
> 通过子模块（submodule）或本地克隆方式管理。

## 仓库清单

### AI 编程助手（Code Assistants）

| 仓库 | 来源 | 说明 |
|------|------|------|
| `aider/` | GitHub | AI 结对编程工具 |
| `claude-code/` | GitHub | Anthropics Claude Code CLI |
| `claude-code-rust/` | GitHub | Claude Code 的 Rust 实现 |
| `claw-code/` | GitHub | Claw Code AI 编程工具 |
| `cline/` | GitHub | Cline AI 编程助手 |
| `deepseek-harness/` | GitHub | DeepSeek Harness 测试框架 |
| `gemini-cli/` | GitHub | Google Gemini CLI |
| `openai-codex/` | GitHub | OpenAI Codex CLI |
| `opencode/` | GitHub | Opencode AI 开发工具 |
| `openhands/` | GitHub | OpenHands AI 开发者 |
| `pi/` | GitHub | PI AI 终端工具 |
| `superpowers/` | GitHub | Gemini Superpowers 扩展 |

### AI Agent 框架

| 仓库 | 来源 | 说明 |
|------|------|------|
| `browser-use/` | GitHub | 浏览器自动化 Agent |
| `dify/` | GitHub | Dify LLM 应用开发平台 |
| `hermes-agent/` | GitHub | Hermes Agent 框架 |
| `langchain/` | GitHub | LangChain 框架 |

### 存储与基础设施

| 仓库 | 来源 | 说明 |
|------|------|------|
| `ceph/` | Gitee 镜像 | Ceph 分布式存储 |
| `Cloudreve/` | GitHub | Cloudreve 云盘系统 |
| `juicefs/` | GitHub | JuiceFS 分布式文件系统 |
| `minio/` | GitClone 镜像 | MinIO 对象存储 |
| `nebula/` | GitHub | NebulaGraph 图数据库 |
| `RustFS/` | Gitee 镜像 | RustFS 文件系统 |
| `seaweedfs/` | Gitee 镜像 | SeaweedFS 分布式存储 |

### 其他资源

| 仓库 | 来源 | 说明 |
|------|------|------|
| `awesome-llm-apps/` | GitHub | LLM 应用精选列表 |
| `system-prompts-and-models-of-ai-tools/` | GitHub | AI 工具系统提示词与模型库 |

## 管理方式

### 当前方式：本地克隆（默认）

目前各仓库以本地 Git 克隆方式存在，已通过 `.gitignore` 排除。

```bash
# 克隆单个仓库
git clone <remote-url> ais/<repo-name>
```

### 推荐方式：Git Submodule

如需精确追踪版本并纳入 CI，建议转换为 Git Submodule：

```bash
# 添加子模块
git submodule add <remote-url> ais/<repo-name>

# 初始化所有子模块
git submodule update --init --recursive

# 更新所有子模块
git submodule update --remote
```

### 转换脚本

运行 `scripts/convert-ais-to-submodules.ps1` 可将现有克隆批量转换为子模块。

## 使用规范

1. **只读原则**：不在 ais/ 中直接修改代码，如需修改请 Fork 到自己的仓库
2. **版本固定**：每个参考仓库固定具体 commit，不随意升级
3. **定期清理**：不再参考的项目及时移除，避免目录膨胀
4. **分类管理**：新增项目按上述分类放置，保持目录有序

## 相关文档

- MOX 竞品分析报告：`docs/enterprise/23-竞品mox 模块化系统架构功能对比与可用性判定报告-V1.0.md`
- AI 引擎基准评测：`docs/enterprise/25-AI引擎真实基准评测报告-V1.0.md`
