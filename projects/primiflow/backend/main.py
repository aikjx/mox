"""PrimiFlow MVP 后端（FastAPI 单服务，离线可跑）。

运行：
    pip install fastapi uvicorn
    uvicorn main:app --reload --port 8000
访问：http://localhost:8000/  （自动托管 web/index.html）
"""
from __future__ import annotations

import uuid
from pathlib import Path
from typing import Any

from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel

from engine import Params, Topology, build_trace, generate_docs, generate_topology, regularize

WEB_DIR = Path(__file__).resolve().parent.parent / "web"

app = FastAPI(title="PrimiFlow MVP", version="R1")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

# ---- 内存存储（生产换 PostgreSQL + pgvector）----
projects: dict[str, dict] = {}
topologies: dict[str, Topology] = {}
artifacts: dict[str, list[dict]] = {}
traces: dict[str, dict] = {}
assets: list[dict] = []  # 冻结资产 Q


# ---- 请求模型 ----
class ProjectReq(BaseModel):
    name: str
    k: float = 0.7
    t: float = 0.3
    c: float = 1.0


class MessageReq(BaseModel):
    content: str
    slider: float = 0.3  # 0=稳定优先, 1=探索优先
    c: float = 1.0


class RegularizeReq(BaseModel):
    slider: float = 0.3
    c: float = 1.0


class UpdateGraphReq(BaseModel):
    nodes: list[dict[str, Any]]
    edges: list[dict[str, Any]]
    slider: float | None = None
    c: float | None = None


# ---- 路由 ----
@app.post("/api/projects")
def create_project(req: ProjectReq):
    pid = str(uuid.uuid4())[:8]
    projects[pid] = {"id": pid, "name": req.name, "k": req.k, "t": req.t, "c": req.c}
    return projects[pid]


@app.post("/api/projects/{pid}/messages")
def post_message(pid: str, req: MessageReq):
    if pid not in projects:
        raise HTTPException(404, "project not found")
    params = Params.from_slider(req.slider, req.c)
    topo = generate_topology(pid, req.content, params)
    topologies[topo.id] = topo
    traces[topo.id] = build_trace(topo)
    return {
        "project_id": pid,
        "topology_id": topo.id,
        "domain": topo.domain,
        "topology": topo.to_json(),
        "trace": traces[topo.id],
    }


@app.get("/api/topologies/{tid}")
def get_topology(tid: str):
    if tid not in topologies:
        raise HTTPException(404, "topology not found")
    return topologies[tid].to_json()


@app.post("/api/topologies/{tid}/regularize")
def re_regularize(tid: str, req: RegularizeReq):
    if tid not in topologies:
        raise HTTPException(404, "topology not found")
    params = Params.from_slider(req.slider, req.c)
    topo = regularize(topologies[tid], params)
    topologies[tid] = topo
    traces[tid] = build_trace(topo)
    return topo.to_json()


@app.post("/api/topologies/{tid}/update")
def update_graph(tid: str, req: UpdateGraphReq):
    """画布编辑回写：用当前节点/边更新拓扑，可选重设 κ/τ，重算 ℛ̂。"""
    if tid not in topologies:
        raise HTTPException(404, "topology not found")
    topo = topologies[tid]
    topo.nodes = req.nodes
    topo.edges = req.edges
    if req.slider is not None or req.c is not None:
        params = Params.from_slider(
            req.slider if req.slider is not None else 0.3,
            req.c if req.c is not None else topo.params.c,
        )
    else:
        params = topo.params
    topo = regularize(topo, params)
    topologies[tid] = topo
    traces[tid] = build_trace(topo)
    return topo.to_json()


@app.post("/api/projects/{pid}/generate-docs")
def gen_docs(pid: str):
    # 取该项目最新拓扑
    topo = _latest_topology(pid)
    docs = generate_docs(topo)
    artifacts[pid] = docs
    return {"project_id": pid, "docs": docs}


@app.get("/api/projects/{pid}/artifacts")
def get_artifacts(pid: str):
    return {"project_id": pid, "docs": artifacts.get(pid, [])}


@app.post("/api/topologies/{tid}/freeze")
def freeze(tid: str):
    if tid not in topologies:
        raise HTTPException(404, "topology not found")
    topo = topologies[tid]
    asset = {
        "id": str(uuid.uuid4())[:8],
        "topology_id": tid,
        "name": f"资产-{topo.domain}",
        "domain": topo.domain,
        "graph_json": topo.to_json(),
    }
    assets.append(asset)
    return {"status": "frozen", "asset": asset, "total_assets": len(assets)}


@app.get("/api/assets")
def list_assets():
    return {"assets": assets}


def _latest_topology(pid: str) -> Topology:
    cand = [t for t in topologies.values() if t.project_id == pid]
    if not cand:
        raise HTTPException(404, "no topology for project")
    return cand[-1]


# ---- 托管前端 ----
@app.get("/")
def index():
    return FileResponse(WEB_DIR / "index.html")


app.mount("/", StaticFiles(directory=str(WEB_DIR), html=True), name="static")
