# locale／时间格式化口径常驻闸门（2026-09-29）

对应治理文档 `docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.32。

## 动机

§5.30／§5.31 两轮收口了 14 处"输出跟随访客浏览器 locale"的副本，但普查本身暴露了一个方法级缺陷：
连续三轮只扫 `toLocaleString`，同族的 `toLocaleTimeString`／`toLocaleDateString` 全程隐身。
**普查的覆盖面由针的形状决定，不由绿的轮数决定。** 一次性普查不能防回潮 ⇒ 本轮把它落成常驻门禁。

## 落地面

- `scripts/gate/check-locale-format-outlets.py`（10378 B，新增，未提交）
- `scripts/gate/check-all.ps1` 第 6 步内追加调用（步骤数仍是 7，编号与 `[N/7]` 标签不动）
- `docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` 新增 §5.32（现 1139 行）

## 判据

| 规则 | 判定 | 现库状态 |
|------|------|----------|
| L1 UNPINNED_DATE | 无参 `toLocaleString()` 且接收者是 `new Date`（同行前缀或 `X = new Date(` 别名）⇒ 按**文件 × 次数**登记表棘轮；`extra`/`grown` = ERROR，`gone` = 余账点名 | 登记 3 处 |
| L2 OUTLET_LITERAL | 出口写法串（两条完整形态）只许活在 `utils/time.js` | 0 违规 |
| L3 LOCALE_PIN | 带 locale 首参但非 `zh-CN` ⇒ ERROR | 0 违规 |
| L4 SCAN_SET | 被扫文件数 ≥ 下限（判集塌缩时"0 命中"与"没有缺陷"同形） | 实测 270，下限 100 |
| I1 数字千分位 | 与时间无关的合法无参调用，按名点名不判定 | 3 处 |
| I2 登记口外的带参直写 | 按名点名（task #25 余账由机器列出，不再手抄） | 6 处 |

## 证据

- `reports/data/locale-gate-selftest.txt`：**11/11 PASS，rc=0**。组成 = L1 两种接收者形态 2 枚、L2 两条出口写法 2 枚、L3 1 枚、"合法数据不许红"的反对照 4 枚（登记口自身／数字千分位／pin 到 zh-CN 的日期档／带秒档自写 `hour12`）、空判集正对照 1 枚（仓库根指向不存在目录 ⇒ rc=1）、扫描集分母 1 枚。
- `reports/data/locale-gate-check.txt`：**rc=0**，含 L1 余账明细与 I1/I2 点名。
- 内存内正对照两枚：植入一枚新站点 ⇒ FIRED；喂登记口自己 ⇒ CLEAN。

## 第一版两处闸门缺陷（如实登记，不静默修）

1. `MIN_FILES` 凭手感钉了高值，而分母必须由现场测量决定 ⇒ `--selftest` 立刻红。修法是印出实测值（现每轮打印 270）而不是抬高期望。
2. L2 最初枚举局部片段 `{ hour12: false }`，把 `AdminMonitor.vue:1138` 那处**合法的带秒档自写**误判为 ERROR。零容忍判据下假阳与假阴同价，而假阳的代价是直接把人推向"绕过闸门"。

两处都是靠反对照那次自检发现的，不是靠真库绿。

## 未做／待裁决

- task #24：F1 时间戳余账 3 处（低码回显契约／`AdminHitl` 秒单位／`TaskView` 把展示串写进 `created_at`）。
- task #25：I2 的 6 处带参直写逐个裁决。
- 本轮**未提交任何文件**，等用户点名。不做 `git add -A`／`git commit -a`／`git stash`。
