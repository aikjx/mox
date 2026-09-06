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
