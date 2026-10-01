# `toLocale*` 全家族普查 → 时钟档/日期档 10 处副本收口（2026-09-29）

权威叙述见 `docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.31。本页记动作与数字来源。

## 仪器的盲区（本轮最值钱的一条）

前三轮时间普查（task #7、§5.27、§5.30）的 needle 都只写 `toLocaleString`。本轮换成整家族 `.toLocale(String|DateString|TimeString)(` 后实测 20 处，其中 **14 处带参调用从未被任何一轮扫到过**，而它们恰好落在联盟工作台、`stores/alliance.store.js`、`modules/expert-alliance/model/rank.js` 这些本目标的核心代码上。结论：**普查的覆盖面由 needle 的形状决定，不由绿灯的轮数决定。**

## 改动

| 文件 | 变化 |
|------|------|
| `src/utils/time.js` | ＋`formatClockMinute(ts = Date.now(), invalid='Invalid Date')`、＋`formatDateStamp(...)`（选项收进 `CLOCK_OPTS` 常量） |
| `src/utils/index.js` | barrel 补两档再导出 |
| `composables/workspace/useAlliance.js` | 时钟档 ×1 → `formatClockMinute()`（−15 B） |
| `stores/alliance.store.js` | ×3 → 同上（−133 B） |
| `views/workspace/ExpertWorkspaceView.vue` | ×2 → 同上（−74 B） |
| `views/project/Workbench.vue` | 时钟 ×1 ＋日期 ×2（−40 B） |
| `modules/expert-alliance/model/rank.js` | 日期 ×1（+21 B，出口名比原表达式长） |
| `src/utils/locale-clock-outlet.test.js` | 新钉 8 例 |

这一族原本就写死 `zh-CN` ⇒ **零版面变更**，纯去重复；坏值仍印 `Invalid Date`（副本原行为）。

## 证据

- `reports/data/locale-family-census.txt`：全家族 20 处坐标（pin/unpin 分列）。
- `reports/data/locale-clock-manifest.txt`：5 个文件的写前/写后 sha＋字节差、三把 needle 余量复算（OPTS 1／日期调用 1／时钟调用 3＝出口 1＋待裁决 2）、HM 整串残留 0。
- `reports/data/locale-clock-suite.txt`：`src/utils` 全目录 5 文件 / **57 例全绿**（含存量 `time.test.js` 24 例）。
- `reports/data/locale-clock-mutation-witness.txt`：基线 3 文件 / 21 例，MB1–MB5 全 CAUGHT，还原 sha 3/3 IDENTICAL；MB5 附带 `vite build` rc=1。
- `reports/data/locale-clock-{gov,deadref,build}.txt`：ERROR=0／`deadRefFiles=0 verdict=PASS`／`rc=0`（37.15 s）。
- `reports/data/locale-clock-collapse.txt`：收口驱动器的崩溃 traceback（原样留存，见下）。

## 两起自伤（如实）

1. 电池 MB1 期望 1–2 例红，实测 3——第三枚来自棘轮 A 也吃下该改动（同一针打红两条独立通道），不是漏判，但预测确实错了。
2. **收口驱动器在所有源文件写完之后、写清单之前崩溃**（bytes 拼 str 的 TypeError）。后果是"活落盘了但没有账"。没有重跑驱动器（那会把已改图像再套一遍），而用独立测量脚本拿写前图像与磁盘现像对账补写 manifest，脚本明文不改写源文件。流程规则：**账要在落盘之前写完**；清单代码本身要进验证阶段跑最小样例。

## 余账（只分类不动手，task #24／#25）

时钟带秒 2 处（`AdminMonitor.vue:1138` 有 `hour12:false`、`Workbench.vue:157` 无 pin）；`ExpertEnterprisePanel.vue:680` 的 `toLocaleString('zh-CN')` 无 `hour12` pin；数字千分位 3 处合法；§5.30 的 F1 时间戳 3 处。共 9 处。

未提交，等用户点名文件。不做 `git add -A`／`git commit -a`／`git stash`。
