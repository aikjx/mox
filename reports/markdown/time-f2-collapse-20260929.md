# 时间口径第二批：F2 族 8 处副本收成一把出口（2026-09-28）

权威叙述：`docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.27。本文件只放本轮工件清单与状态。

## 1. 改了什么

单出口：`frontend-ui/src/utils/time.js` 新增 `formatDateTimeLocale(ts)`（可解析且 t>0 → `new Date(t).toLocaleString('zh-CN', { hour12: false })`；坏值 → `null`），经 `frontend-ui/src/utils/index.js:11` 由 barrel 再导出。

八个收口点（调用形态统一为 `formatDateTimeLocale(d) ?? <该站点原有空态>`）：

| 文件 | 空态回落 |
|------|---------|
| `views/admin/panels/AdminDepartment.vue` | `'Invalid Date'` |
| `views/admin/panels/AdminRole.vue` | `'Invalid Date'` |
| `views/admin/panels/AdminUser.vue` | `'Invalid Date'` |
| `views/admin/panels/AdminMonitor.vue` | `String(ts)` |
| `views/expert/panels/ExpertOrchestratorPanel.vue` | `'Invalid Date'`（此文件为 CRLF，按字节改写未归一行尾） |
| `views/project/Dashboard.vue` | `String(ts)` |
| `modules/expert-alliance/components/ExpertBookingPanel.vue` | `iso` |
| `modules/expert-alliance/model/display.js` | `raw` |

外加一条结案：`views/project/ProjectsView.vue` 模板调用 `projectMemberCount(p)` 却没有绑定（函数在 `utils/projectMember.utils.js`，有 12 例测试）⇒ 补 `import { projectMemberCount } from '@/utils'`。这是渲染期 ReferenceError，不是风格问题。

## 2. 本轮工件（`reports/data/`，尺寸为 `stat` 实测）

| 工件 | 字节 | 内容 |
|------|-----:|------|
| `time-family-census-after.txt` | 1,616 | 本轮现测的 F1/F1-数/F2/F3/F4 分布与坐标 |
| `time-f2-mutation-witness.txt` | 2,253 | M1–M5 逐枚判决 + fired 测试名 + `-> restored … sha=` |
| `barrel-and-deadref-mutation-witness.txt` | 1,024 | M6/M7 两条岛屿对照 + 还原证明 |
| `barrel-fix-manifest.txt` | 775 | 9 个文件 before→after 尺寸与 sha256 前 12 位 |
| `time-f2-build2.txt` | 19,532 | 收口后 `vite build` 全量资产表 |
| `gov-after.txt` | 1,042 | `check-frontend-module.py` 收口后输出（E6 = 0 处 ✓） |
| `deadrefs-after.txt` | 90 | `check-sfc-dead-refs.mjs`：`deadRefFiles=0 verdict=PASS` |
| `time-f2-suite.txt` | 299 | `26 files / 151 tests passed` |

驱动与前像（**未落库＝没牙，不要接 CI**）：`D:\tmp\time-f2-collapse2.py`、`D:\tmp\time-f2-mut.py`、`D:\tmp\barrel-fix.py`、`D:\tmp\barrel-mut.py`、`D:\tmp\time-family-census-after.py`；前像目录 `D:\tmp\time-f2-pre3\`（5 个 `.before` + `manifest.txt`）、上一批 3 个在 `D:\tmp\time-f2-pre\`。

## 3. 两处有意的行为差（不是等价改写）

1. `t ≤ 0`（epoch 0／1970 前）：旧写法印 `1970/1/1 08:00:00`，新出口回 `null` ⇒ 站点回落到自己的空态文案。推理依据是这些字段全部来自后端 `now_iso()`；**没有现场行级证据**。
2. epoch 毫秒写成数字串（`'1758000000000'`）：旧写法 `new Date(串)` 得 `Invalid Date` 并直接印出，新出口先落毫秒数 ⇒ 印正常日期。

## 4. 门禁读数

`vite build` rc=0；`check-frontend-module.py` rc=0（一度 8 处 E6 ERROR：本单元先写了 `@/utils/time` 深路径，被 §4 规矩打回 barrel）；`check-sfc-dead-refs.mjs` verdict=PASS（上轮 FAIL 的存量清零）；`check-doc-links.py` rc=1 属存量（WARN 总数 120 与基线一致，本文件未新增悬空）。体积：全量 raw `6696.8→6697.1 kB`、`main 217.88→217.93 kB`；gzip 列本轮解析失败，未与 §6 的 gzip 口径混算。

## 5. 没拿到的证据 / 余账

- **真机渲染零证据**：`:3020`／`:3080`／`:3001` 本轮全 DOWN。#22 仍欠。
- F1 无参 7 处：改了会换版面（宿主 locale 决定），需产品口径。扫描器把 3 处数字千分位 `n.toLocaleString()` 也认进 F1 ⇒ 仪器要按接收者类型分叉，已在 §5.27.1 登记。
- F3 选项集 4 处：不并；F4 手工拼接 6 处文件命中：其中 `ExpertPlazaView` 被 #20 挡。
- 未提交：本轮 9 个源文件 + 1 个新测试 + 文档 + 8 份工件，全部只在磁盘。**按仓库规矩不做 `git add -A`/`stash`，等用户点名。**
