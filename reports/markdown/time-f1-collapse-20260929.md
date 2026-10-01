# F1 族时间口径归一：4 份逐字符相同 `fmtTime` 副本收口到单出口（2026-09-29）

权威叙述见 `docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.30。本页只记本轮做了什么、数字挂在哪份产物上。

## 一句话

`toLocaleString()` 无参写法让同一张表在 zh-CN 浏览器和 en-US 浏览器印出两种字面量；4 个后台面板各写了一份逐字符相同的副本（块体 sha256 前 12 位 `4117530d3121` × 4），本轮收成 `utils/time.js` 的 `formatDateTimeLocaleOr`。

## 改动

| 文件 | 变化 |
|------|------|
| `src/utils/time.js` | ＋`formatDateTimeLocaleOr(ts, empty = '-')`：falsy→占位符，解析失败→`Invalid Date` |
| `src/utils/index.js` | barrel 补再导出 |
| `src/views/admin/panels/{AdminAccess,AdminAudit,AdminConfig,AdminMenu}.vue` | 各删 1 份副本体，换 1 行别名导入；各 −59 B |
| `src/utils/time-f1-outlet.test.js` | 新钉 6 例 |

模板调用点（5 处 `fmtTime(`）一行没改。

## 两条实测事实

- 副本的 `catch { return String(t) }` 是死支：坏串不抛，`new Date(坏串)` 格式化返回字符串 `Invalid Date`。⇒ 收口保留"坏值印 Invalid Date"的可见契约，不做"改进"。
- 无参写法的两处显示 delta：负 epoch 旧印 1969 年（新档 `Invalid Date`）、数字串旧印 `Invalid Date`（新档解析成真日期，与 §5.27 同一 delta）。

## 余账 6 处，分类不动手

`reports/data/time-f1-split.txt`：时间戳 3（`widgetRegistry.js` 自带回显契约／`AdminHitl.vue` 秒制单位／`TaskView.vue` 把显示串写进 `created_at` 数据）＋ 数字千分位 3（`AdminLlm.vue` ×2、`ExpertPlazaView.vue`，与时间无关的合法用途）。task #24 登记前三处。

## 证据与数字来源

- `reports/data/time-f1-collapse.txt`：收口前 4 份副本 sha、各文件 before/after 字节、收口后余量 6 处。
- `reports/data/time-f1-suite.txt`：13 例全绿（本钉 6 ＋ F2 钉 7）。
- `reports/data/time-f1-mutation-witness.txt`：基线绿 + 6 枚变异全 CAUGHT，MU5 带出 `deadref rc=1 deadRefFiles=1 verdict=FAIL`，MU6 带出 `build rc=1` 点名消费者。
- `reports/data/time-f1-gov.txt` / `time-f1-deadref.txt` / `time-f1-build.txt`：门禁 ERROR=0／`deadRefFiles=0 verdict=PASS`／`vite build rc=0`。
- 还原他证：独立 hashlib 复算 4/4 与被碰文件备份 IDENTICAL（电池输出内）。

## 本轮自伤（如实）

电池 v1 用 `open(path,'wb').write(fragment)` 把片段当整文件写，将**未入库**的 `src/utils/time.js` 削成 53 字节——`?? ` 状态文件没有 git 图像可退。恢复源只剩本轮早先的整文件 Read 转写；重建后 13 例全绿，其中 F2 那枚棘轮要求 `utils/time.js` 恰好 1 次命中被禁字面量，等价于把内容又过了一遍关。修法已进电池 v2：开局把每个被碰文件整份备份到新目录，目录已存在即拒绝运行。

未提交。等用户点名文件（含同日前三件累计：11 个源文件、4 个新测试、1 个新 `_kernel` 模块、文档、20+ 份工件）。不做 `git add -A`／`git commit -a`／`git stash`。
