# F4 手工拼接档收口（2026-09-29）

权威条文：`docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.35。本文件只是当轮读数快照。

## 判决

| 项 | 值 | 印数器 |
| --- | --- | --- |
| 收口前站点（含测试文件口径） | 348 文件 / 10 处 / 6 种字段组合 | `D:\tmp\f4-census.py`（未落库＝没牙）→ `reports/data/f4-census-20260929.txt` |
| 收口后站点（与闸门 L4 同口径） | 270 文件 / 6 处 / 5 种 | 同上 `--no-tests` → `reports/data/f4-census-after-20260929.txt` |
| 本轮并掉 | 3 处（`MessageBubble.vue:944`、`useWorkspaceData.js:42`、`FlowDetailDialog.vue:183`） | 写前图像见证 + 棘轮台账缺席 |
| 登记为"档不同不并" | 5 处（毫秒档／相对档尾巴 M-D／locale 日期档／MM-DD HH-mm／M-D HH-mm） | `src/utils/f4-hand-assembly.test.js` 台账 |
| 版面变更 | 1 处且有意：`t≤0` 由印 `1970-01-01 08:00` 改为空态文案 `—` | 同上第 3 例 |
| 坏值契约 | `FlowDetailDialog` 的 `String(s)` 原样回显保留在调用方（出口不提供回显） | 同上第 4 例 |

## 见证与牙齿

- 相等性**不靠我抄写**：旧写法由写前图像（`D:\tmp\f4-pre\`）按正则找回整行、`new Function` 就地造函数，6 个正时刻全 EQ ⇒ `reports/data/f4-preimage-witness-20260929.txt`（rc=0）。
- 同一台仪器喂**收口后**的盘必须报"锚点行命中 0 ⇒ 无效见证"且 rc=1 ⇒ `reports/data/f4-instrument-control-20260929.txt`。
- 变异电池 6/6：F4-M1 站点退回自写式→棘轮红；M2 空态 `'—'`→`'-'`→契约红；M3 走 `@/utils/time` 深路径→barrel 红；M4 出口内部月份丢 `+1`→相等性红；M5 退回 `nowTime()` 别名→"不留纯别名"红；反对照 C1（读字段但无拼接痕迹）必须不红且真的不红。5 个被碰文件逐字节还原=IDENTICAL ⇒ `reports/data/f4-mutation-witness-20260929.txt`。

## 门禁（本轮末次实测）

| 门禁 | 结果 |
| --- | --- |
| 全量 vitest | 76 文件 / 1008 例 / rc=0（上轮 75/1000；本单元 +1 文件 +8 例） |
| `vite build`（rollup，改导出/导入的唯一验收命令） | rc=0，38.96 s |
| `check-frontend-module.py` | ERROR=0（三处均走 barrel，深路径 0） |
| `check-sfc-dead-refs.mjs` | files=128 checked=126 deadRefFiles=0 verdict=PASS |
| `check-locale-format-outlets.py` | rc=0（270 文件，L1 余账 2 处、I2 3 处本轮未动）；`--selftest` 11/11 PASS |
| `check-doc-links.py` | rc=1 为存量（120 WARN，与本轮前值同数）；本文件唯一 WARN 在 :137（旧悬空 `FRONTEND-MODULE.md`），§5.35 未引入新悬空 |

## 仪器教训（已并入记忆）

普查器与棘轮的**判集口径不一致**会让收口看起来像没做：含测试文件的旧口径下 10 处→10 处（新测试里为现推旧写法而写的副本抵掉了真删的 3 处）。另踩一次"分母凭手感钉"：下限写成 `>300`（按普查器 348 估）而针的判集是 270 ⇒ 自检立刻红，改回"印实测值＋塌缩下限 100"。

## 未结

- 本轮改动**全部未提交**；`utils/time.js`、`scripts/gate/check-locale-format-outlets.py`、`src/utils/f4-hand-assembly.test.js` 为未跟踪文件＝无 git 恢复源，写前图像只在 `D:\tmp\f4-pre\`。入库需用户点名文件（task #19）。
- 时间/locale 口径待裁决余账 5 处（task #25）＝L1 2 处（低码回显契约、`AdminHitl` 秒制单位）+ I2 3 处；F4 这 5 处不并不属该 5 处，属"档不同"已结案登记。
