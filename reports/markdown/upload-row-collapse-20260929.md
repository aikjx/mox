# C 族私有词汇副本收口：工作台上传占位行（2026-09-28）

权威叙述：`docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.28。本文件只放分族读数、工件清单与状态。承 `time-f2-collapse-20260929.md`（同一天的前一件）。

## 1. 尺子先造：按内容分组而不是按名字

`reports/data/vocab-census.py`（形状普查）＋ `reports/data/vocab-dup.py`（sha 分组）在 `frontend-ui/src` 非测试源里扫自写"码→中文"映射，扫出 **45 条 / 22 个名字**（完整读数 `reports/data/vocab-dup.txt`）：

- **(a) 真副本 1 组 × 2 站点** — `views/workspace/panels/CollaborationPanel.vue:418` 与 `views/workspace/panels/FilePanel.vue:106` 的 `newFile` 占位行；同处两文件的 `getFileType`（574 B）、`formatFileSize`（210 B）函数体也逐字符全等。⇒ task #12 记的"C 族 2 处"被现测复核成立。
- **(b) 同键集不同文案 2 组** — `ExpertEnterprisePanel.vue:548/553`（策略名 vs 释义）、`BrowserView.vue:319/324`（档位标签 vs 说明）。**不并**：那是两档版面。
- **(c) 键集重叠 ≥50% 共 11 对** — 跨域同键名（`completed/running/…`），**不算重复**；机械合并会把两个域的口径焊死。

## 2. 改动

新增 `frontend-ui/src/modules/_kernel/upload-row.js`（1,628 B）：`getFileType`／`formatFileSize`／`makeUploadRow(file)` 三个出口，键与文案由驱动器从原副本**字节级照搬**（不在控制台重打中文，避免 GBK 显示层把好字节写坏）。

两个面板各 −892 B：删掉本地两份定义，`handleBeforeFileUpload` 收敛为 `const newFile = makeUploadRow(file)` ＋原有 `emit`/提示。行为零改动。

新测试 `frontend-ui/src/modules/_kernel/upload-row.test.js`（5 例：八分支／档位与零值文案／六键六值（`vi.setSystemTime(1790510400000)` 数字纪元，避免时区）／两面板接线且不留本地副本／棘轮：`'f-' + Date.now()` 只许活在登记口）。

## 3. 变异体（`reports/data/upload-row-mutation-witness.txt`）

| 变异体 | fired | 判决 |
|--------|------:|------|
| MU1 登记口 `f-`→`upload-` | 2（预注册预测 1） | 等价例 + 棘轮同红：改登记口会把 needle 从登记口移走。**预测错在把棘轮当成只看总数**——与 §5.27.3 M1 同一课 |
| MU2 在 `FilePanel` 注释里植一条 needle | 1 | 只有棘轮红 ⇒ 它确实读注释 |

还原：两文件 sha `1f2f4ac63d44`／`fb363afb20c0` 与 `-> restored` 行一致；前像 `D:\tmp\upload-row-pre\`（`.before` + `manifest.txt`，驱动器拒绝覆盖已存在的前像目录）。首跑电池因 `SRC` 少退一级而 UNEXPECTED 中止，磁盘零改动，修判据后重跑。

## 4. 门禁（`reports/data/`）

| 读数 | 值 |
|------|---|
| `vitest run src/views src/modules src/utils` | `Tests 843 passed (843)`（`upload-row-suite.txt`，236 B；该次 `tail -4` 把 `Test Files` 行截掉了，故只报用例数） |
| `npx vite build` | `rc=0`，`✓ built in 28.33s`（`upload-row-build.txt`） |
| `check-frontend-module.py` | `rc=0`，`E6 = 0 处 ✓`、`E7 全部可解析 ✓`（`upload-row-gov.txt`） |
| `check-sfc-dead-refs.mjs` | `deadRefFiles=0 verdict=PASS`（`upload-row-deadref.txt`） |
| `check-doc-links.py` | `rc=1` 属存量，WARN 120 与基线一致，本轮文档未新增悬空 |

## 5. 没拿到的证据 / 余账

- 真机渲染零证据：`:3020`／`:3080`／`:3001` 本轮全 DOWN（#22）。
- (b) 两组、(c) 十一对＝口径分裂，动手前要有人裁决每域词表的家。
- `uploader: '我'`／`time: '刚刚'` 的乐观占位只在登记口内收敛成一份，**没有**改成真实归属；回读覆盖那条链未验。
- 普查尺子只认"键值全等"⇒ 两域同键同值而语义不同时会误报成副本；本轮靠 (c) 档的"重叠但不同值"挡住，方向已登记。
- 未提交：2 个面板 + 1 个新 `_kernel` 文件 + 1 个新测试 + 该文档（工作树 1,039 行 vs 索引副本）+ 本目录 7 份工件 + 2 份脚本 + 前两件工件。**按仓库规矩不做 `git add -A`/`stash`，等用户点名。**
