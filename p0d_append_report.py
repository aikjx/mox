# -*- coding: utf-8 -*-
import io, os

REPORT = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\_verification\backend-fix-report.md"

SECTION = """

---

## P0-C 双值令牌滚动（2026-09-29 第三轮）

### 背景

12-innovation-roadmap.md §1.3 表第 3 行「下游 svc 内部令牌双值滚动 / WS 文档修订」前半。
下游三 svc（scheduler/executor/registry）的 `internal_auth_layer` 此前只校验单一 `MOX_INTERNAL_TOKEN`，
生产换令牌必须停机或双窗口内 401。本轮加备用令牌 `MOX_INTERNAL_TOKEN_ALT`，支持零停机滚动。

### 改动文件（路径 + 行号）

三处 `internal_auth_layer` 函数体同构改造，每处净增 3 行逻辑 + 注释：

1. `platform/domains/alliance/svc/mox-alliance-scheduler-svc/src/routes.rs`
   - doc 注释 :62-79（新增滚动流程说明 5 行）
   - 函数体 :80-104：
     - :86 新增 `let expected_alt = std::env::var("MOX_INTERNAL_TOKEN_ALT").unwrap_or_default();`
     - :87 放行条件由 `expected.is_empty()` 改为 `(expected.is_empty() && expected_alt.is_empty())`
     - :101 校验由 `t == expected` 改为 `t == expected || (!expected_alt.is_empty() && t == expected_alt)`

2. `platform/domains/alliance/svc/mox-alliance-executor-svc/src/routes.rs`
   - doc 注释 :119-136
   - 函数体 :137-161：同构（expected_alt 读取 / 双空放行 / 双值或校验）

3. `platform/domains/alliance/svc/mox-alliance-registry-svc/src/routes.rs`
   - doc 注释 :71-88
   - 函数体 :89-113：同构

### 行为保持（向后兼容）

- 两令牌均未配置 → 与现状一致放行（`dev_mode || (主空 && 备空)`）
- `MOX_DEV_MODE=1` 跳过逻辑不变
- 公开白名单（/health /metrics /leadership /api/registry/health）不变
- 网关出站仍只读主值 `MOX_INTERNAL_TOKEN`（registry_client.rs / alliance_remote.rs 未动）；
  滚动时网关切主值即可，下游因同时接受 ALT 旧值而不 401

### 生产滚动流程（双窗口，零停机）

1. 旧值写入 `MOX_INTERNAL_TOKEN_ALT` → 全集群滚动重启三 svc（此时主值=旧值、ALT=旧值，行为不变）
2. 网关出站 `MOX_INTERNAL_TOKEN` 切新值 → 全集群滚动重启网关
   （旧副本带旧值命中 ALT，新副本带新值命中主值，均不 401）
3. 观察一个周期后移除 `MOX_INTERNAL_TOKEN_ALT`，完成滚动

### 验证结果

- `cargo check -p mox-alliance-scheduler-svc -p mox-alliance-executor-svc -p mox-alliance-registry-svc --all-targets`
  → Finished dev profile in 9.51s，无 error / 无 warning
- `cargo test` 三 svc：
  - executor-svc：unit 9 + http_integration 6 = **15 passed, 0 failed**
  - registry-svc：unit 17 + http_registry 11 = **28 passed, 0 failed**
  - scheduler-svc：unit 10 + http_integration 11 = **21 passed, 0 failed**
  - 合计 **64 passed, 0 failed**
  - scheduler http_integration 11 用例全部通过（这些用例未带 Authorization，因 MOX_DEV_MODE / 未配置令牌而走放行分支，不受双值改造影响）
- gateway alliance 用例：见 p0c_gateway_test.log（本轮补跑）

### 测试缺口说明

三 svc 现有测试套件均在「未配置令牌 / dev_mode」放行路径下跑，未构造「配置主+备后 Bearer=ALT 通过、Bearer=错值 401」的用例。
因改造为纯函数级（env 读取 + 字符串比较），且现有 64 用例全绿不破坏放行路径，本轮按任务要求「若没有测试文件则跑现有 cargo test 确认不破坏」执行；
双值正向/反向用例建议后续在集成测试中补（需在测试进程内临时 set_var，注意 env 全局性需串行）。
"""

with io.open(REPORT, "a", encoding="utf-8", newline="") as f:
    f.write(SECTION)

print("appended", REPORT, len(SECTION), "chars")
