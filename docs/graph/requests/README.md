# 新需求判重入口（P9 先判重后立项）

> 本目录是「新需求进入系统的唯一入口」。任何新需求在立项前，必须先在此放入一份规格文件，
> 由关图 CI 门禁（`tools/guantu_gate.py` step6）自动判重，杜绝重复造系统。

## 工作方式

CI 会对本目录下每个 `*.json` 执行：

```bash
info-graph dedup --graph graph.enterprise.json --spec docs/graph/requests/<你的需求>.json --fail-on-new
```

判定结果三类：

| 判定 | 含义 | CI 行为 | 应采取的动作 |
|---|---|---|---|
| `reuse` | 候选能力节点与关系边**全部**已存在于关图 | ✅ 放行 | 直接编排现有能力，**不写新代码** |
| `incremental` | 部分能力已存在 | ✅ 放行 | 在既有子图上**局部扩展**，避免重造系统 |
| `new` | 关图中无任何对应能力 | ❌ **阻断** | 人工确认确有必要后，才允许新立项 |

`similarity = 已命中能力节点数 / 候选能力节点总数`；`reuse` 还额外要求关系边零缺失
（能力都在但连接方式不同，说明是新的组合方式，判为 `incremental`）。

## 规格格式

```json
{
  "id": "REQ-2026-001",
  "name": "需求名称（支持中文）",
  "capabilities": [
    { "kind": "CodeFile", "key": "crates/xuanji-system/src/lib.rs" }
  ],
  "edges": [
    { "from": "CodeFile:a.rs", "to": "CodeFile:b.rs", "kind": "Reference" }
  ]
}
```

- `kind` 取值同关图节点类型：`CodeFile` / `Interface` / `Function` / `Business` / `Data` /
  `Config` / `Doc` / `Script` / `ScheduleTask` / `Dependency` / `Runtime` / `Requirement`。
- `key` 为节点路径（相对仓库根），最终节点 id 为 `Kind:key`。
- `edges.kind` 取值：`Reference` / `Dependency` / `ReadWrite` / `Call` / `Deploy` / `Bind` / `Schedule`。
- 文件请存为 **UTF-8**；带 BOM 也可（工具已容忍），但不推荐。

## 查证现有能力

不确定图里有什么，先查：

```bash
info-graph build --root . --out graph.json
info-graph query --graph graph.json --kind CodeFile --name xuanji
```

## 判定为 new 之后

1. 先确认真的没有相近能力（换关键词再查一遍 `query`）；
2. 确有必要立项 → 在 `docs/graph/guantu.req.json` 增加 REQ 根与六维绑定；
3. 需求落地后，把本目录下对应规格文件移除或更新（其判定自然会变为 `reuse`）。
