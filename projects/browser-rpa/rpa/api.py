from __future__ import annotations

import os
from typing import Any, Dict, List, Optional

from fastapi import FastAPI, HTTPException
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel

from . import config, devtest, flow, healer, intake, ops, recorder, runner, store

app = FastAPI(title="browser-rpa", version="0.1.0",
              description="浏览器 RPA 单容器：录制 Python 脚本 · AI 自愈 · 智能运维 · 需求归一化")


class IntakeIn(BaseModel):
    requirement_text: str


class TaskIn(BaseModel):
    requirement: Optional[Any] = None
    spec: Optional[Dict[str, Any]] = None


class ScriptIn(BaseModel):
    code: str


class RunIn(BaseModel):
    heal: bool = True


class RecordIn(BaseModel):
    url: Optional[str] = None
    headed: bool = True


def _task_or_404(task_id: str) -> Dict[str, Any]:
    spec = store.get_task(task_id)
    if spec is None:
        raise HTTPException(404, "任务不存在: " + task_id)
    return spec


@app.on_event("startup")
def _startup() -> None:
    config.ensure_dirs()
    if os.environ.get("RPA_SCHEDULER", "1") != "0":
        ops.scheduler.start()


@app.get("/health")
def health() -> Dict:
    return ops.health()


@app.post("/intake")
def do_intake(body: IntakeIn) -> Dict:
    return intake.intake(body.requirement_text)


@app.get("/tasks")
def tasks() -> list:
    return store.list_tasks()


@app.post("/tasks")
def create_task(body: TaskIn) -> Dict:
    if body.spec:
        spec = dict(body.spec)
        spec.setdefault("schema", config.SCHEMA)
    elif body.requirement is not None:
        spec = intake.intake(body.requirement)
    else:
        raise HTTPException(400, "需要 requirement 或 spec")
    return store.save_task(spec)


@app.get("/tasks/{task_id}")
def task_detail(task_id: str) -> Dict:
    spec = _task_or_404(task_id)
    return {**spec, "script_history": store.script_history(task_id)}


@app.post("/tasks/{task_id}/record/start")
def record_start(task_id: str, body: RecordIn) -> Dict:
    _task_or_404(task_id)
    result = recorder.start(task_id, url=body.url or (_task_or_404(task_id).get("url") or ""),
                            headed=body.headed)
    if not result.get("ok"):
        raise HTTPException(409, result.get("error"))
    return result


@app.post("/tasks/{task_id}/record/stop")
def record_stop(task_id: str) -> Dict:
    _task_or_404(task_id)
    result = recorder.stop(task_id)
    if not result.get("ok"):
        raise HTTPException(409, result.get("error"))
    return result


@app.put("/tasks/{task_id}/script")
def upload_script(task_id: str, body: ScriptIn) -> Dict:
    _task_or_404(task_id)
    compile(body.code, "<uploaded>", "exec")  # 语法校验
    return {"script_version": store.save_script(task_id, body.code, source="manual", note="接口上传")}


@app.get("/tasks/{task_id}/script")
def get_script(task_id: str, version: Optional[int] = None) -> Dict:
    _task_or_404(task_id)
    code = store.get_script(task_id, version)
    if code is None:
        raise HTTPException(404, "脚本不存在")
    return {"task_id": task_id, "version": version, "code": code}


@app.post("/tasks/{task_id}/run")
def run_task(task_id: str, body: RunIn) -> Dict:
    spec = _task_or_404(task_id)
    if body.heal and (spec.get("heal") or {}).get("enabled", True):
        return healer.heal(task_id, (spec.get("heal") or {}).get("max_attempts"))
    code = store.get_script(task_id)
    if code is None:
        raise HTTPException(404, "无脚本，请先录制或上传")
    result = runner.run_script(code)
    record = store.save_run({"task_id": task_id, "status": "passed" if result["ok"] else "failed",
                             "script_version": spec.get("script_version"), "stderr": result["stderr"]})
    return {**result, "run_id": record["run_id"]}


@app.post("/tasks/{task_id}/heal")
def heal_task(task_id: str) -> Dict:
    spec = _task_or_404(task_id)
    return healer.heal(task_id, (spec.get("heal") or {}).get("max_attempts"))


@app.get("/runs")
def runs(task_id: Optional[str] = None, limit: int = 50) -> list:
    return store.list_runs(task_id, limit)


@app.get("/runs/{run_id}")
def run_detail(run_id: str) -> Dict:
    import json
    from pathlib import Path
    path = Path(config.RUNS_DIR / (run_id + ".json"))
    if not path.exists():
        raise HTTPException(404, "运行记录不存在")
    return json.loads(path.read_text(encoding="utf-8"))


@app.get("/events")
def events(limit: int = 100) -> list:
    return store.list_events(limit)


# ---------- 流程模块化 / 测试开发 ----------

class FlowIn(BaseModel):
    name: str
    description: Optional[str] = ""
    steps: Optional[List[Dict[str, Any]]] = None


class FlowPatchIn(BaseModel):
    name: Optional[str] = None
    description: Optional[str] = None
    steps: Optional[List[Dict[str, Any]]] = None
    tags: Optional[List[str]] = None


class ImportScriptIn(BaseModel):
    code: str


class AttachTaskIn(BaseModel):
    task_id: str
    headless: bool = True


class FlowTestIn(BaseModel):
    start: int = 0
    stop_at: Optional[int] = None
    headless: bool = True


def _flow_or_404(flow_id: str) -> Dict[str, Any]:
    f = flow.get(flow_id)
    if f is None:
        raise HTTPException(404, "流程不存在: " + flow_id)
    return f


@app.get("/flow-modules")
def flow_modules() -> list:
    return flow.modules_spec()


@app.get("/flows")
def flows_list() -> list:
    return flow.list_flows()


@app.post("/flows")
def flows_create(body: FlowIn) -> Dict:
    f = flow.create(body.name, body.description or "", body.steps)
    store.append_event("flow.created", {"flow_id": f["flow_id"], "name": f["name"]})
    return f


@app.get("/flows/{flow_id}")
def flows_get(flow_id: str) -> Dict:
    return _flow_or_404(flow_id)


@app.put("/flows/{flow_id}")
def flows_update(flow_id: str, body: FlowPatchIn) -> Dict:
    _flow_or_404(flow_id)
    patch = {k: v for k, v in body.dict().items() if v is not None}
    return flow.update(flow_id, patch)


@app.delete("/flows/{flow_id}")
def flows_delete(flow_id: str) -> Dict:
    _flow_or_404(flow_id)
    return {"ok": flow.delete(flow_id)}


@app.post("/flows/{flow_id}/import-script")
def flows_import(flow_id: str, body: ImportScriptIn) -> Dict:
    _flow_or_404(flow_id)
    steps = flow.import_script(body.code)
    f = flow.update(flow_id, {"steps": steps})
    store.append_event("flow.imported", {"flow_id": flow_id, "steps": len(steps)})
    return {"steps": len(steps), "flow": f}


@app.get("/flows/{flow_id}/script")
def flows_script(flow_id: str, headless: bool = True) -> Dict:
    f = _flow_or_404(flow_id)
    return {"code": flow.compile_flow(f, headless=headless)}


@app.post("/flows/{flow_id}/attach-task")
def flows_attach(flow_id: str, body: AttachTaskIn) -> Dict:
    f = _flow_or_404(flow_id)
    if store.get_task(body.task_id) is None:
        raise HTTPException(404, "任务不存在: " + body.task_id)
    code = flow.compile_flow(f, headless=body.headless)
    version = store.save_script(body.task_id, code, source="flow",
                                note="来自流程 {}".format(flow_id))
    return {"task_id": body.task_id, "script_version": version}


@app.post("/flows/{flow_id}/test")
def flows_test(flow_id: str, body: FlowTestIn) -> Dict:
    f = _flow_or_404(flow_id)
    report = devtest.test_flow(f, headless=body.headless, start=body.start, stop_at=body.stop_at)
    store.append_event("flow.tested", {"flow_id": flow_id, "ok": report.get("ok"),
                                       "steps": len(report.get("results", []))})
    return report


# 静态管理台挂载于最后：/api 路由优先匹配，/ 落到 console
_WEB_DIR = config.PROJECT_DIR / "web"
if _WEB_DIR.exists():
    app.mount("/", StaticFiles(directory=str(_WEB_DIR), html=True), name="console")
