# 带秒时钟档与 I2 六处直写的实测（2026-09-29）

治理文档：`docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.34。上游：§5.30／§5.31／§5.32／§5.33。

## 先量后动

§5.32 的常驻闸门把"登记口外的带参 `toLocale*` 直写"按名点名 6 处（I2 通道）。本轮先把每处的**选项对象从源码里按大括号配对截出**、在 Node 中求值（脚本 `D:\tmp\i2-census.mjs`，未落库⇒没牙），对三个样本（含午夜与一个跨 UTC 日的瞬间）判"与既有出口档是否逐字符相同"，再决定是否收口。

结论（读数见 `reports/data/clock-second-mutation-witness.txt` 基线段与各钉子）：

- `ExpertEnterprisePanel.vue:680` 的 `now.toLocaleString('zh-CN')` ≡ 现有出口 `formatDateTimeLocale(Or)` ⇒ **零版面变更收口**。
- `AdminMonitor.vue:1138`（pin 了 `hour12:false`）与 `Workbench.vue:158`（只 pin `'zh-CN'`）输出**完全相同** ⇒ 真重复副本对，新增第三档 `formatClockSecond`。
- `AdminLlm.vue:647`（`MM/DD HH:mm:ss`）、`MarketView.vue:608`（`YYYY/MM/DD HH:mm`）、`BrowserView.vue:586`（`YYYY/MM/DD HH:mm:ss`）三者字段集互不相同 ⇒ **档不同不并**，机械替换会改版面。

**纠正 §5.32 的分类**：`zh-CN` 在这台 V8 上默认即 24 小时制 ⇒ "未 pin `hour12`"不是显示缺陷，而是把契约押在 locale 默认值上。收口后 pin 成为出口契约。

## 改动（5 个源文件 + 2 个测试）

- `src/utils/time.js`：新增 `CLOCK_SEC_OPTS` 与 `formatClockSecond(ts = Date.now(), invalid = 'Invalid Date')`。
- `src/utils/index.js`：barrel 增加该名字。
- `views/admin/panels/AdminMonitor.vue`、`views/project/Workbench.vue`：带秒时钟改走 `formatClockSecond()`。
- `views/expert/panels/ExpertEnterprisePanel.vue`：`diagnosticTime` 改走 `formatDateTimeLocaleOr(now)`，新增 barrel 导入。
- 新增钉子 `src/utils/clock-second-outlet.test.js`（6 例：两写法等价／形状两冒号／坏值与改名／EEP 等价／带秒字面量棘轮／3 站点接线）。
- `src/utils/locale-clock-outlet.test.js`：时钟棘轮 B 从"出口 1 + 待裁决 2"改成"全库只剩登记口（time.js 内 2 处）"。
- `src/utils/time-locale-outlet.test.js`：接线判据不再钉整句 import（见下）。
- `scripts/gate/check-locale-format-outlets.py`：L2 出口写法清单加入带秒选项字面量（含 docstring 同步）。

展示版面变化：**0**。

## 撞上的第三本台账

`time-locale-outlet.test.js` 用 `toContain("import { formatDateTimeLocale } from '@/utils'")` 判 8 个收口点的接线，
我在同一句 import 里加一个同伴名字就把它判红了。判据过窄：接线要钉的是"这个名字经由 barrel 进来"。
改成 `/import \{[^}]*\bformatDateTimeLocale\b[^}]*\} from '@\/utils'/`。
**教训：台账越像整句原文，跨单元的合法改动越容易撞红。**（同族：§5.33 的 `REGISTERED_TOTAL` 手抄数字、§5.32 的 `file:line` 坐标。）

## 门禁与电池

- 基线全量 vitest **75 文件 / 1000 例 / rc=0**；`vite build` rc=0（28.12 s）；`check-frontend-module.py` ERROR=0；`check-sfc-dead-refs.mjs` verdict=PASS；locale 闸门 rc=0 且 **I2 6→3**。
- 变异电池 **5/5 CAUGHT、5/5 restored=IDENTICAL（sha 复验）**：
  MC1 丢 `second`（4 例红）／MC2 `hour: 'numeric'`（3 例红）／MC3 站点退回自写（2 例红：新档接线针 + 时钟棘轮 B）／
  MC4 barrel 撤名字（3 例红 **+ `vite build` rc=1**，rollup 点名 `Workbench.vue (12:28): "formatClockSecond" is not exported`）／
  MC5 EEP 退回未 pin 整串（1 例红）。
- **诚实登记盲区**：MC1/MC2/MC3/MC5 的闸门 rc 仍为 0——闸门审"谁在登记口外直写"，不审出口内部行为；出口内部靠 `src/utils` 的相等性钉子。MC3/MC5 时闸门唯一反应是 I2 计数 3→4 的 INFO 点名（advisory 无牙，牙在测试里，与 §5.33 MT4 同型）。

## 备份与余账

六个被碰文件的字节前像备在 `D:\tmp\i2-pre\`（sha1 前缀：`time.js 2c784387…`、`index.js 787f1197…`、`AdminMonitor 46795aae…`、`Workbench 3be1dd20…`、`ExpertEnterprisePanel 990e73fd…`、`locale-clock-outlet.test ce18e2e5…`）；本轮所有变异还原后 sha 与之一致。

余账：时间/locale 待裁决直写由 9 处变 **5 处**＝L1 2 处（低码回显契约、`AdminHitl` 秒制单位）+ I2 3 处（本轮判"档不同、不并"）。task #25 已按此改写。

本轮**未提交任何文件**，未触碰 git 索引。
