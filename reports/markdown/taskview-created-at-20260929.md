# TaskView `created_at` 写侧污染收口（2026-09-29）

治理文档：`docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.33。上游单元：§5.30／§5.31／§5.32（同一口径族的显示侧与常驻门禁已收口，本轮拔的是"下一枚针"）。

## 缺陷

`frontend-ui/src/views/project/TaskView.vue` 乐观写入列表行时执行
`created_at: new Date().toLocaleString()` —— 把浏览器 locale 的**展示串**写进**数据字段**，并覆盖了 `createTask()` 的服务端回显。

为什么这不只是显示问题（读代码得到的 wire 真相）：

| 事实 | 出处 |
|------|------|
| `TaskItem.created_at: String`（服务端存字符串） | `platform/gateway/mox-platform-gateway-svc/src/misc.rs:35` |
| 列表默认按 `created_at` **字典序**排序 | 同文件 `:332`（`b.created_at.cmp(&a.created_at)`） |
| 服务端造值用 RFC3339 秒级 + `Z` | 同文件 `:122`（`to_rfc3339_opts(SecondsFormat::Secs, true)`） |

⇒ `2026/9/29 08:01:51` 与 `2026-09-28T23:59:10Z` 混在同一列里，该行的排序位置永久错位。

## 改动（3 个文件）

- `views/project/TaskView.vue`：写侧 → `created_at: newTask.created_at || new Date().toISOString()`；展示侧 → `{{ formatDateTimeLocaleOr(currentTask.created_at) }}`；新增 `import { formatDateTimeLocaleOr } from '@/utils'`（barrel，深路径会被门禁 E6 判 ERROR）。
- `scripts/gate/check-locale-format-outlets.py`：`EXPECTED_UNPINNED` 删掉 `views/project/TaskView.vue` 条目（清零删条目，不写 0）。
- `src/utils/time-f1-outlet.test.js`：`REGISTERED` 同步删条目；`REGISTERED_TOTAL` 从手抄数字改为对台账求和。
- 新增钉子 `src/views/project/taskview-created-at.test.js`（5 例）。

展示版面：新建行仍显示同一本地挂钟时刻（那正是出口的职责）；变化在字段内部值回到 RFC3339，且服务端已有值时不再被客户端时刻顶掉。

## 两本台账 & advisory 通道的盲区

同一事实存在两处登记（闸门的 `EXPECTED_UNPINNED` 与 F1 棘轮的 `REGISTERED`）。删站点后两本同时红＝见证。
关键发现：闸门对"条目该删"只打印 `[INFO]`／`[SHRANK]`，**rc 仍为 0** ⇒ 腐烂台账在 CI 里静默。钉子第 5 例做交叉核对（已清条目不许在、其余条目指向的文件必须仍有该形状），由 MT4 证明它有牙。

## 证据（`reports/data/taskview-*.txt`）

- `taskview-suite.txt`：`src/views/project` + `src/utils` 全量 **62 例 / rc=0**
- `taskview-gate.txt`：常驻闸门 **rc=0**，`无参时间格式化余账 2 处＝登记表`（TaskView 由机器列出，不再手抄）
- `taskview-build.txt`：`vite build` **rc=0**，`TaskView-9JQHWEoZ.js 12.02 kB`
- `taskview-mutation-witness.txt`：基线 62 例 rc=0 + 闸门 rc=0，**变异 5/5 CAUGHT、还原 5/5 IDENTICAL（sha 复验）**

| 枚 | 变异 | 结果 |
|----|------|------|
| MT1 | 写侧退回展示串 | 3 例红 **+ 闸门 rc=1**（唯一进 CI 的一枚） |
| MT2 | 展示侧退回直出 | 1 例红（展示侧通道） |
| MT3 | 撤 barrel 导入 | 1 例红 |
| MT4 | 把已清条目加回闸门台账 | 1 例红，**闸门 rc=0** ⇒ 补上 advisory 盲区 |
| MT5 | F1 台账计数写错 | 棘轮 1 例红 |

其余门禁：`check-frontend-module.py` ERROR=0、`check-sfc-dead-refs.mjs` `deadRefFiles=0 structFiles=0 verdict=PASS`。

## 本轮自伤（如实登记）

1. 我写的第 4 例把期望钉成 UTC 日（`2026-09-28`），而出口渲染本地挂钟（+08:00 ⇒ 该瞬间是 09-29）⇒ **假红**。改成钉"形状 + 指回同一瞬间"。教训：口径出口的期望不许假设时区。
2. `time-f1-outlet.test.js` 与 `utils/time.js` 同为**未跟踪文件**（无 git 恢复源）。本轮对它用的是 Edit（不会截断），且 3 行反向改动可复原，但**没留字节前像** —— 违反既有流程记忆"先备份每个被碰文件"。下轮起：未跟踪文件也要先 `cp -p` 再动。
3. 驱动 `D:\tmp\tv-mut.py` 未落库 ⇒ 没牙，别接 CI；它的输出已作为 `taskview-mutation-witness.txt` 落档。

## 余账

- task #24 从 3 处变 **2 处**：`modules/admin-lowcode/engine/widgetRegistry.js`（低码列原样回显契约）、`views/admin/panels/AdminHitl.vue`（秒制单位）—— 两者都是展示契约差异。
- task #25：I2 的 6 处带参直写逐个裁决。
- 本轮**未提交任何文件**，未触碰 git 索引（`TaskView.vue` 第一列的 `M` 是并发作者的暂存）。
