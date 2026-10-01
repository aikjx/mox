# -*- coding: utf-8 -*-
"""
P0-D (2026-09-29): 对 01/02/03/06 四个 V1.0 文档做 WebSocket 幻影补记。
- 标题下方插入状态横幅
- 每处 /ws/v1 相关表述所在章节/表格/代码块末尾插入 ⚠️ 补记
- 幂等：已存在补记则跳过，不重复插入
- 只读改这四个文件，原文不动
"""
import io, os, sys

BASE = r"D:\a10\aikjx\gitcode\infotopograph\docs\expert-alliance"

BANNER = "> 状态：V1.0 目标态，部分结论已被 CURRENT-ARCHITECTURE.md V1.1 取代（见文中 ⚠️ 补记）。"

WARN = (
    "> ⚠️ **V1.1 核对补记（2026-09-29）**：本文此处所述的 `/ws/v1/*` WebSocket 推送在实现中不存在"
    "（全 crate `WebSocketUpgrade` 零命中）；实时性由 SSE `GET /api/alliance/tasks/:id/logs/stream` 承担。"
    "以 CURRENT-ARCHITECTURE.md V1.1 为准。"
)

def read(p):
    with io.open(p, "r", encoding="utf-8", newline="") as f:
        return f.read()

def write(p, s):
    with io.open(p, "w", encoding="utf-8", newline="") as f:
        f.write(s)

def insert_after(text, anchor, addition, tag, already_marker):
    """在 anchor 首次出现后插入 addition；若已含 already_marker 则原样返回。"""
    if already_marker in text:
        print(f"  [SKIP] {tag}: 已存在补记")
        return text, False
    idx = text.find(anchor)
    if idx < 0:
        raise SystemExit(f"  [ERR] {tag}: 锚点未找到 -> {anchor[:60]!r}")
    end = idx + len(anchor)
    # 在 anchor 后插入：保持 anchor 后的换行风格
    new = text[:end] + "\n\n" + addition + text[end:]
    print(f"  [OK ] {tag}")
    return new, True

def patch_doc(fname, title_anchor, inserts):
    p = os.path.join(BASE, fname)
    s = read(p)
    orig = s
    print(f"=== {fname} ===")
    # 1) 横幅：标题下方
    banner_marker = "见文中 ⚠️ 补记"
    if banner_marker in s:
        print(f"  [SKIP] 横幅已存在")
    else:
        idx = s.find(title_anchor)
        if idx < 0:
            raise SystemExit(f"  [ERR] 标题锚点未找到: {title_anchor!r}")
        end = idx + len(title_anchor)
        s = s[:end] + "\n\n" + BANNER + s[end:]
        print(f"  [OK ] 状态横幅")
    # 2) 各补记
    for anchor, tag in inserts:
        s, _ = insert_after(s, anchor, WARN, tag, "V1.1 核对补记（2026-09-29）")
    if s != orig:
        write(p, s)
        print(f"  -> 已写回 {p}")
    else:
        print(f"  -> 无改动")
    print()

# 01-prd.md: F-04 表格末行
patch_doc(
    "01-prd.md",
    "# 专家联盟产品需求文档（PRD）",
    [
        ("| 对应代码 | executor-core/ + 网关 experts_orchestration.rs |",
         "F-04 进度推送表格后补记"),
    ],
)

# 02-architecture.md: §1.2 六层架构代码块末 + §2.2 调用链代码块末
patch_doc(
    "02-architecture.md",
    "# 专家联盟系统总体架构",
    [
        ("└─────────────────────────────────────────────────────────────┘\n```",
         "§1.2 L6 接入层（REST/WebSocket/SSE）代码块后补记"),
        ("底层微服务（AI/图谱/搜索/存储等）\n```",
         "§2.2 调用链 ↓ REST/WebSocket 代码块后补记"),
    ],
)

# 03-business-flow.md: §4 节点执行代码块末
patch_doc(
    "03-business-flow.md",
    "# 专家联盟业务处理流程",
    [
        ("输出：全部节点执行结果\n```",
         "§4 [5]进度推送 WebSocket/SSE 代码块后补记"),
    ],
)

# 06-api-spec.md: §4 WebSocket 接口表格后
patch_doc(
    "06-api-spec.md",
    "# 专家联盟 API 接口规范",
    [
        ("| `/ws/v1/experts/tasks/:task_id/progress` | 任务执行进度推送 |",
         "§4 /ws/v1 表格后补记"),
    ],
)

print("DONE")
