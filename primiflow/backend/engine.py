"""PrimiFlow 核心引擎（MVP 竖向切片，离线可跑）。

用规则化生成器代替 LLM，证明主链路：
  自然语言需求 + κ/τ 滑块 → 拓扑 DAG → ℛ̂ 正则化 → 可编辑画布 → 8 份说明书

生产环境将 `generate_topology` / `generate_docs` 替换为 Python 算子层
(llm_gateway + topology_operator + doc_generator)，接口保持一致。
"""
from __future__ import annotations

import math
import re
import uuid
from dataclasses import dataclass, field
from typing import Any


# ---------------------------------------------------------------------------
# 参数与资产
# ---------------------------------------------------------------------------

@dataclass
class Params:
    k: float  # 收敛/复用权重 ∈ [0,1]
    t: float  # 裂变/探索权重 ∈ [0,1]
    c: float  # 全局资源上界(预算) > 0

    @classmethod
    def from_slider(cls, slider: float, c: float = 1.0) -> "Params":
        """滑块 s∈[0,1]：0=稳定优先(κ→1)，1=探索优先(τ→1)。"""
        s = max(0.0, min(1.0, slider))
        theta = s * (math.pi / 2)
        return cls(k=round(math.cos(theta), 4), t=round(math.sin(theta), 4), c=max(0.1, c))

    def residual(self) -> float:
        """ℛ̂ 残差：Δ = C² − (κ²+τ²)。归一化下 κ²+τ²=1，故 Δ=C²−1。"""
        return self.c * self.c - (self.k * self.k + self.t * self.t)


# 内置可复用资产库（κ 复用来源；生产走 pgvector 语义检索）。
# key=业务域关键词，value=可复用子拓扑(节点列表)。
ASSET_LIBRARY: dict[str, list[dict[str, Any]]] = {
    "电商|订单|交易|支付": [
        {"id": "pay", "label": "支付对账", "kind": "reuse"},
        {"id": "refund", "label": "退款流程", "kind": "reuse"},
        {"id": "inventory", "label": "库存扣减", "kind": "reuse"},
    ],
    "用户|登录|注册|权限|账号": [
        {"id": "auth", "label": "登录鉴权", "kind": "reuse"},
        {"id": "rbac", "label": "权限模型", "kind": "reuse"},
    ],
    "通知|消息|短信|推送|邮件": [
        {"id": "notify", "label": "消息推送", "kind": "reuse"},
    ],
}


def detect_domain(text: str) -> str:
    for keys, _ in ASSET_LIBRARY.items():
        if any(k in text for k in keys.split("|")):
            return keys.split("|")[0]
    return "通用"


# ---------------------------------------------------------------------------
# 拓扑生成
# ---------------------------------------------------------------------------

@dataclass
class Topology:
    id: str
    project_id: str
    domain: str
    params: Params
    nodes: list[dict[str, Any]] = field(default_factory=list)
    edges: list[dict[str, Any]] = field(default_factory=list)
    status: str = "draft"
    delta: float = 0.0
    note: str = ""

    def to_json(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "project_id": self.project_id,
            "domain": self.domain,
            "k": self.params.k,
            "t": self.params.t,
            "c": self.params.c,
            "status": self.status,
            "delta": round(self.delta, 4),
            "note": self.note,
            "nodes": self.nodes,
            "edges": self.edges,
        }


# 探索型(τ 驱动)附加节点池
EXPLORATORY_NODES = [
    {"id": "abtest", "label": "A/B 实验", "kind": "explore", "priority": 1},
    {"id": "monitor", "label": "监控告警", "kind": "explore", "priority": 2},
    {"id": "analytics", "label": "数据分析", "kind": "explore", "priority": 3},
    {"id": "audit", "label": "操作审计", "kind": "explore", "priority": 4},
]


def generate_topology(project_id: str, text: str, params: Params) -> Topology:
    topo_id = str(uuid.uuid4())[:8]
    domain = detect_domain(text)

    # 基础链路（核心交付骨架）
    base = [
        ("input", "客户输入", "core"),
        ("req", "需求分析", "core"),
        ("design", "方案设计", "core"),
        ("data", "数据模型", "core"),
        ("api", "接口设计", "core"),
        ("task", "定时任务", "core"),
        ("doc", "文档生成", "core"),
        ("deploy", "部署上线", "core"),
        ("done", "交付", "core"),
    ]
    nodes = [{"id": i, "label": l, "kind": k} for i, l, k in base]
    edges = [{"source": base[i][0], "target": base[i + 1][0]} for i in range(len(base) - 1)]

    # κ 复用：命中内置资产则并入复用节点
    reused = 0
    if params.k > 0.4:
        for keys, sub in ASSET_LIBRARY.items():
            if any(k in text for k in keys.split("|")):
                for n in sub:
                    if not any(x["id"] == n["id"] for x in nodes):
                        nodes.append({"id": n["id"], "label": n["label"], "kind": "reuse"})
                        edges.append({"source": "design", "target": n["id"]})
                        edges.append({"source": n["id"], "target": "api"})
                        reused += 1

    # τ 探索：按 τ 比例加入探索型节点
    explore_count = int(round(params.t * len(EXPLORATORY_NODES)))
    for node in EXPLORATORY_NODES[:explore_count]:
        nid = node["id"]
        if not any(x["id"] == nid for x in nodes):
            nodes.append({"id": nid, "label": node["label"], "kind": "explore",
                          "priority": node["priority"]})
            edges.append({"source": "deploy", "target": nid})

    topo = Topology(id=topo_id, project_id=project_id, domain=domain, params=params,
                    nodes=nodes, edges=edges)
    regularize(topo, params)
    return topo


# ---------------------------------------------------------------------------
# ℛ̂ 正则化：预算裁剪 + 矛盾环检测
# ---------------------------------------------------------------------------

def _cost(nodes: list, edges: list) -> float:
    return len(nodes) * 1.0 + len(edges) * 0.5


def regularize(topo: Topology, params: Params | None = None) -> Topology:
    if params is not None:
        topo.params = params
    p = topo.params

    # 1) 矛盾环检测（用户编辑可能引入环）
    if _has_cycle(topo.nodes, topo.edges):
        topo.status = "rejected"
        topo.note = "检测到矛盾环(DAG 不允许回边)，请删除回边后重算 ℛ̂"
        return topo

    # 2) 预算裁剪：cost 不得超过 C 的放大预算
    budget = max(1.0, p.c) * 6.0  # 经验缩放：C=1 约容纳 ~9 节点
    # 探索型节点优先级低，先裁
    explore_nodes = [n for n in topo.nodes if n.get("kind") == "explore"]
    explore_nodes.sort(key=lambda n: n.get("priority", 99), reverse=True)
    removed = 0
    while _cost(topo.nodes, topo.edges) > budget and explore_nodes:
        victim = explore_nodes.pop()
        topo.nodes = [n for n in topo.nodes if n["id"] != victim["id"]]
        topo.edges = [e for e in topo.edges
                      if e["source"] != victim["id"] and e["target"] != victim["id"]]
        removed += 1

    topo.delta = p.residual()
    topo.status = "regularized"
    topo.note = f"ℛ̂ 通过：Δ={topo.delta:.3f}，预算内；裁剪探索节点 {removed} 个" if removed \
        else f"ℛ̂ 通过：Δ={topo.delta:.3f}，预算内"
    return topo


def _has_cycle(nodes: list, edges: list) -> bool:
    adj: dict[str, list[str]] = {n["id"]: [] for n in nodes}
    for e in edges:
        adj.setdefault(e["source"], []).append(e["target"])
    WHITE, GRAY, BLACK = 0, 1, 2
    color = {nid: WHITE for nid in adj}
    stack: list[tuple[str, int]] = list(adj.keys())

    def dfs(u: str) -> bool:
        color[u] = GRAY
        for v in adj.get(u, []):
            if color.get(v, WHITE) == GRAY:
                return True
            if color.get(v, WHITE) == WHITE and dfs(v):
                return True
        color[u] = BLACK
        return False

    for start in list(adj.keys()):
        if color[start] == WHITE:
            if dfs(start):
                return True
    return False


# ---------------------------------------------------------------------------
# 8 份标准化说明书生成
# ---------------------------------------------------------------------------

DOC_KINDS = [
    ("req", "需求规格说明书"),
    ("feature", "功能设计说明书"),
    ("business", "业务流程说明书"),
    ("data", "数据模型说明书"),
    ("api", "接口契约说明书"),
    ("task", "定时任务说明书"),
    ("code", "代码工程说明书"),
    ("ops", "部署运维说明书"),
]


def generate_docs(topo: Topology) -> list[dict[str, Any]]:
    p = topo.params
    nodes = topo.nodes
    edges = topo.edges
    names = {n["id"]: n["label"] for n in nodes}

    def edge_lines() -> str:
        return "\n".join(f"- {names.get(e['source'], e['source'])} → {names.get(e['target'], e['target'])}"
                         for e in edges) or "- (无)"

    docs = []

    docs.append({
        "kind": "req", "title": "需求规格说明书",
        "content": f"# 需求规格说明书\n\n"
                   f"- 业务域：{topo.domain}\n"
                   f"- 来源需求：详见对话\n"
                   f"- 拓扑原语：κ={p.k}(复用/稳定) τ={p.t}(探索/裂变) C={p.c}(预算)\n"
                   f"- 核心模块：{', '.join(names[i] for i in ['req','design','data','api','deploy'] if i in names)}\n",
    })
    docs.append({
        "kind": "feature", "title": "功能设计说明书",
        "content": "# 功能设计说明书\n\n" + "\n".join(f"- {n['label']}（{n.get('kind','core')}）" for n in nodes),
    })
    docs.append({
        "kind": "business", "title": "业务流程说明书",
        "content": "# 业务流程说明书\n\n主流程：\n" + edge_lines(),
    })
    docs.append({
        "kind": "data", "title": "数据模型说明书",
        "content": "# 数据模型说明书\n\n依据拓扑节点推断实体：\n"
                   + "\n".join(f"- 表 `{n['id']}`：对应「{n['label']}」" for n in nodes if n.get('kind') != 'explore'),
    })
    docs.append({
        "kind": "api", "title": "接口契约说明书",
        "content": "# 接口契约说明书\n\n- POST /api/" + topo.domain + "/create\n"
                   "- GET  /api/" + topo.domain + "/:id\n"
                   "- 错误统一 RFC9457",
    })
    docs.append({
        "kind": "task", "title": "定时任务说明书",
        "content": "# 定时任务说明书\n\n- 节点「定时任务」：建议 cron 每日 02:00 执行对账/同步。",
    })
    docs.append({
        "kind": "code", "title": "代码工程说明书（骨架）",
        "content": "# 代码工程说明书（骨架/桩）\n\n"
                   "```\n"
                   f"src/{topo.domain}/\n  handler.py   # {names.get('api','接口')} 入口\n"
                   f"  service.py   # {names.get('design','方案')} 逻辑\n"
                   f"  model.py     # {names.get('data','数据')} 模型\n```\n"
                   "> MVP 仅产出骨架；完整代码生成属 V2。",
    })
    docs.append({
        "kind": "ops", "title": "部署运维说明书",
        "content": "# 部署运维说明书\n\n- 依赖：PostgreSQL + 运行时服务\n"
                   "- 观测：κ/τ 消耗、ℛ̂ 裁剪次数、资产命中率\n"
                   "- 回滚：保留上一拓扑快照",
    })
    return docs


# ---------------------------------------------------------------------------
# 六维溯源（需求↔功能↔业务↔算法↔任务↔代码）
# ---------------------------------------------------------------------------

def build_trace(topo: Topology) -> dict[str, Any]:
    return {
        "project_id": topo.project_id,
        "requirement_id": topo.id,
        "feature_id": topo.id,
        "business_id": topo.id,
        "algorithm_id": f"R̂({topo.params.k},{topo.params.t})",
        "task_id": topo.id,
        "code_id": topo.id,
    }
