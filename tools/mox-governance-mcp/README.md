# mox-governance-mcp

把 infotopograph（璇玑）仓库的两项 CI 门禁脚本，包装成标准 **MCP（Model Context Protocol）** 服务器，
让 AI 编码助手在**编码当场**就能查询权威端口、校验漂移、体检断链，而不是等到 CI 失败再返工。

| 文件 | 作用 |
|---|---|
| `server.py` | MCP 服务器实现（stdio + JSON-RPC 2.0），零第三方依赖 |
| `manifest.json` | 工具/资源清单与安全边界，供 IDE / Agent 平台登记 |
| `SKILL.md` | Skill 描述（渐进式指令），供 AI 编码助手按场景加载 |

底层门禁脚本保持为唯一事实源，本服务器不复制规则，只负责编排与返回：

- `scripts/verify-ports.py` —— 端口漂移校验，规范见 `docs/api/PORT-REGISTRY.md`（PORT-REGISTRY-001）
- `scripts/check-doc-links.py` —— 文档链接校验（DOC-GOV-ARC-V1.0 §7）

## 为什么零依赖

只用 Python 标准库（实测兼容 CPython 3.8.8），stdio 直讲 JSON-RPC：CI 容器、Windows 开发机、离线环境都能即开即用，
不引入 SDK 版本漂移。同时在题意上更"可审计"——工具契约、安全边界全部落在一个可读的源文件里。

## 提供的能力

### Tools

| 工具 | 说明 | 只读 |
|---|---|---|
| `mox_port_lookup` | 按端口号精确查 / 按服务名关键字模糊查权威端口注册表，返回分类与是否禁止复用 | 是 |
| `mox_port_verify` | 执行端口漂移校验：退役端口被活跃引用、`platform_config.json` 与注册表不一致、未登记端口 | 是 |
| `mox_doc_links_check` | 执行文档链接校验：Markdown 链接、HTML `href`/`src`、反引号路径引用 | 是 |
| `mox_ci_gate` | 自定义多步流程：一次串联上述两项门禁，输出统一通过与失败清单 | 是 |

### Resources

| URI | 内容 |
|---|---|
| `mox://governance/port-registry` | 全量端口按分类输出 Markdown，可直接注入模型上下文 |
| `mox://governance/gates` | 门禁脚本清单、职责与规范出处 |

## 接入方式

任意支持 MCP stdio 的客户端，加入一条服务器配置即可（把路径替换为本仓库实际绝对路径）：

```json
{
  "mcpServers": {
    "mox-governance": {
      "command": "python",
      "args": ["d:/a10/aikjx/gitcode/infotopograph/tools/mox-governance-mcp/server.py"],
      "env": { "MOX_REPO_ROOT": "d:/a10/aikjx/gitcode/infotopograph" }
    }
  }
}
```

Windows 若默认 `python` 不是 3.8+，把 `command` 换成解释器的完整路径。
`MOX_REPO_ROOT` 可省略，缺省取 `server.py` 上溯两级的仓库根。

## 命令行自检

```
python tools/mox-governance-mcp/server.py --describe    # 打印工具/资源清单
python tools/mox-governance-mcp/server.py --selftest    # 本地自检（含真实脚本调用与子进程 stdio 端到端）
python tools/mox-governance-mcp/server.py               # 以 MCP 服务器运行（阻塞读 stdin）
```

`--selftest` 覆盖：initialize / tools-list / 工具入参 schema / 端口命中与未命中 / 关键字查询 /
资源读取 / 未知方法错误码 / 未知工具隔离 / 真实门禁脚本调用 / stdio 子进程端到端。

## 典型调用返回（节选）

`mox_port_lookup {"port": 3080}`：

```json
{
  "found": true,
  "port": 3080,
  "category": "RUNTIME",
  "service": "api（Rust 网关 mox-server，唯一对外 HTTP 入口）",
  "deprecated": false,
  "reusable": false
}
```

`mox_ci_gate {}`：

```json
{
  "ok": true,
  "repo": "d:/a10/aikjx/gitcode/infotopograph",
  "gates": { "port_registry": { "passed": true, "error_count": 0, "warn_count": 0 },
             "doc_links": { "passed": true, "broken_count": 0 } },
  "failures": [],
  "summary": "治理体检通过：端口无漂移、文档无断链，可以提交/发布。"
}
```

## 安全边界

1. 仅可执行 `scripts` 白名单内的两个门禁脚本；
2. `subprocess` 恒为 `shell=False`，外部入参只映射为受控布尔/枚举，不拼接命令行；
3. 单次执行超时默认 600 秒（本机实测端口全仓扫描约 130 秒）、硬上限 900 秒；
4. 仓库根只接受环境变量显式指定且通过存在性校验的目录，缺省为本仓库根。

## 与既有 CI 的关系

本 MCP 是 CI 门禁的**前移**：同样的脚本、同样的规则，只是触发点从"push 之后"提前到"编码之中"。
落地规则以 `scripts/` 下的脚本为准，本目录不做规则分叉。
