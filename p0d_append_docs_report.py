# -*- coding: utf-8 -*-
import io

REPORT = r"D:\a10\aikjx\gitcode\infotopograph\docs\expert-alliance\_verification\docs-verification-report.md"

ADD = """

---

## 9. P0-D WS 文档幻影修订（2026-09-29 第三轮）

12-innovation-roadmap.md §1.3 表第 3 行后半（#25 / 10-D3）。本轮对 01/02/03/06 四个 V1.0 目标态文档
**只插入补记、不改原文**（保持历史原貌 + 显式标注被取代），与 08/INDEX/CURRENT 已有的幻影标注闭环。

### 修订动作（每份文档：标题下横幅 + 章节末尾补记）

| 文档 | 横幅（标题下） | ⚠️ 补记插入位置 | 原文 /ws/v1 表述 |
|---|---|---|---|
| 01-prd.md | :11 | F-04 DAG 执行引擎表格后（:81） | :75 `进度推送 WebSocket/SSE 实时推送节点级进度` |
| 02-architecture.md | :12 | §1.2 六层架构代码块后（:64） | :29 L6 接入层 `REST / WebSocket / SSE` |
| 02-architecture.md | （同上横幅） | §2.2 进程间调用链代码块后（:123） | :111 `↓ REST/WebSocket` |
| 03-business-flow.md | :11 | §4 节点执行代码块后（:178） | :170 `[5]进度推送 WebSocket/SSE 推送节点状态` |
| 06-api-spec.md | :12 | §4 WebSocket 接口表格后（:184） | :182 `/ws/v1/experts/tasks/:task_id/progress` |

### 补记统一文案

> ⚠️ **V1.1 核对补记（2026-09-29）**：本文此处所述的 `/ws/v1/*` WebSocket 推送在实现中不存在
> （全 crate `WebSocketUpgrade` 零命中）；实时性由 SSE `GET /api/alliance/tasks/:id/logs/stream` 承担。
> 以 CURRENT-ARCHITECTURE.md V1.1 为准。

横幅统一文案：
> 状态：V1.0 目标态，部分结论已被 CURRENT-ARCHITECTURE.md V1.1 取代（见文中 ⚠️ 补记）。

### 插入方式与校验

- Python 脚本 `p0d_patch_docs.py`（UTF-8 读写，保留原换行），按锚点字符串定位插入，幂等。
- 首跑后人工读回：01:81 / 02:64,123 / 03:178 / 06:184 共 **5 处补记 + 4 处横幅**，无乱码、无重复插入。
- 02 文档因含两处 WS 表述，首跑脚本幂等逻辑（全文含标记即跳过）误吞第二处，已用 Edit 在 §2.2 代码块后手工补插（:123），并复核。
- 原文一字未改（01:75、02:29,111、03:170、06:182 的 WebSocket 字样原样保留，仅在其所在表格/代码块后追加补记）。

"""

with io.open(REPORT, "r", encoding="utf-8") as f:
    s = f.read()

marker = "## 9. P0-D WS 文档幻影修订"
if marker in s:
    print("already patched, skip")
else:
    s = s.replace("*报告完。*", ADD + "*报告完。*")
    with io.open(REPORT, "w", encoding="utf-8", newline="") as f:
        f.write(s)
    print("appended OK")
