# infotopograph 项目长期约定

## 在 Git Bash 中编译 Rust（必须）

本机工具链是 `x86_64-pc-windows-msvc`，但 Git Bash 的 `/usr/bin/link`（GNU coreutils）
会抢在 MSVC `link.exe` 之前被 rustc 找到，导致
`link: extra operand '...rcgu.o'`——这是 GNU link 的报错格式，不是 MSVC 的 LNKxxxx。

编译前必须先设置（可在单行命令里 export，shell 状态不持久）：

```bash
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64/link.exe"
export LIB="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/lib/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/um/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/ucrt/x64"
```

缺 `LIB` 会报 `LNK1181: 无法打开输入文件"kernel32.lib"`。
VS 装在 `Program Files (x86)`（不是 `Program Files`），SDK 版本 `10.0.19041.0`。

**Windows MSVC 完整编译环境**（须设 PATH/INCLUDE/LIB 全套，cc-rs 需要 cl.exe）：

```bash
export PATH="/c/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64:/c/Program Files (x86)/Windows Kits/10/bin/10.0.19041.0/x64:$PATH"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64/link.exe"
export LIB="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/lib/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/um/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/ucrt/x64"
export INCLUDE="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/include;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/ucrt;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/um;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/shared"
```

仅设 `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` 仍会失败（cc-rs 找不到 cl.exe）。

**Bash 多行 export 转义不稳**（含括号路径、续行符会被 eval 解析失败），用脚本文件：
`.workbuddy/_build_check.sh` / `_run_*.sh`。Bash 工具的 `command` 单行直接传会报
`syntax error near unexpected token '('`。

**Defender 干扰**：`target/` 频繁被实时保护持锁（.rmeta / .d / .fingerprint）。
根治用 `CARGO_TARGET_DIR=D:/cargo-target` + `CARGO_INCREMENTAL=0`。本仓库 git 跟踪 target/，
隔离目录不影响 git 状态。

## 架构门禁

- 入口：`tools/architecture_gate.py`（仓库根执行），同时跑 `arch_test.py` 与
  `architecture_constraint_test.py` 两套规则，任何 P0/P1 都产生退出码 1。
- 报告输出：`reports/architecture/module-governance-YYYYMMDD.json`。
- God Module 阈值 10，统计口径是 `cargo metadata` 的**声明依赖**，含 dev/build/optional，
  脚本 docstring 自述"不代表单个部署产物的依赖图"。因此 dev 依赖同样计入扇出。
- `tools/contract_meta_check.py`：静态校验 CRATE_ID/ENGINE_NAME/CRATE_META 契约，
  替代了原先需要 extern crate 16 个成员的 Rust 测试。

## 依赖治理原则（已确立）

不为降低扇出数字而移动依赖。能力下沉前先核对目标 crate 的依赖重量：
把 `wasmer`、`rusqlite` 这类重运行时塞进轻量 SDK/契约层，危害大于扇出超标。
跨域能力的正确归处是运行时代理到领域服务，而非编译期直连。

## 联盟域架构要点（开发专家联盟）

- 13 个 crate 分层：api / 3 proto / 6 core / 2 sdk / 2 svc
- 治理核心 `mox_optimize(&FlowGraph, &GovernContext) -> GovernanceReport`（`mox-ai-expert-svc`）
  含 14 维专家评估 + 闸门 + 璇玑算法验证
- LLM 路径（gateway `experts_collaboration::generate_expert_answer`）原本只走
  `ConsultReport.vetoed` 自评 + `parse_veto` 文本子串匹配，**未实际进入 `mox_optimize`**。
- 第十二轮补 `llm_governance.rs`（后验文本→FlowGraph 映射），把 LLM 路径接到 `mox_optimize`。
  接线点：`map_report_to_answer` 在已有 `report.vetoed` 拦截后、构造 JSON 前。
- 敏感度判定 SSOT：`mox-ai-expert-svc::sensitivity::{is_sensitive_domain,
  is_desensitized, is_production, is_sensitive_leak, is_production_or_sensitive_write}`。
  资源 URI 形式：`<scheme>:<env>/<domain>/<entity>`（scheme=db/pii/var/file/url/http），
  敏感域关键字：citizen_/pii/id_card/phone/bank_card；生产环境前缀：prod/production/main。
- FlowGraph 类型从 `mox_ai_flow_svc::model::*` 取（`mox_ai_flow_svc` re-exports
  `mox_ai_flow_core::*`），使用 `FlowNode::task` / `with_access` / `with_tag` 构造。
