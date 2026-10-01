# F5 自写时间副本收口（2026-09-29）

权威叙述在 `docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.36；本页只放读数与出处。

## 判决

- 普查起点：**13 处自写 `formatTime/relativeTime` 定义**（`src/utils/time.test.js` 的 `KNOWN_COPIES` 求和，扫描集 349 个文件）。
- 本轮收口 **5 处**：`model/display.js` 的 `relativeTime` 委托给出口 `relativeTimeText`；
  `AdminDepartment.vue` / `AdminRole.vue` / `AdminUser.vue` / `ExpertOrchestratorPanel.vue` 四处收成
  `import { formatDateTimeLocaleOr as formatTime } from '@/utils'`。
- 余 **8 处登记不并**（每处一份"档不同"的根据，逐行见 §5.36 的表）。
- 读数出处：`reports/data/f5-family-census-20260929.txt`（驱动器 `D:/tmp/f5-family.mjs`，**未落库＝没牙**，含一处 awk 尾组丢行的自记补正）。

## 版面账（只有两笔，都是修坏值）

1. 会话"最近活跃度"：epoch 毫秒数字串输入下由空白变成 `N 分钟前`（旧副本 `Date.parse` 在该形状上恒空转）。
2. 三个 Admin 面板 + 编排面板的时间列：同形状下由 `Invalid Date` 变成可读时刻。

## 牙齿

`reports/data/f5-mutation-witness-20260929.txt`：变异 6 枚，判据不符 0 枚；
M1–M5 各自 RED，C1（`relativeTimeHint`）GREEN；被碰四文件还原逐字节 MATCH。

## 门禁

`reports/data/f5-gates-20260929.txt` + 普查文件末尾：全量 vitest **76 文件 / 1016 例 / rc=0**，
`vite build` rc=0，`check-locale-format-outlets.py --check` rc=0（L1 2 处 / L4 270 / selftest PASS），
`check-frontend-module.py` ERROR=0，`check-sfc-dead-refs.mjs` PASS。

## 未结

- 第六档（"坏值原样回显"）要不要加：`display.js formatTime` + `FlowDetailDialog.vue fmtTime` 两处撞上。
- `AdminSso.vue:194` 印服务端 UTC 挂钟（与全站本地挂钟差一个偏移），**已定位未修**，修法要选档。
- 一律未提交。`utils/time.js`、`utils/time.test.js`、`utils/f4-hand-assembly.test.js` 为 `??`＝无 git 恢复源
  （预像在 `D:/tmp/f5-pre/`）；`display.js` 与四个面板为 `MM`（第一列属并发作者的暂存）。
