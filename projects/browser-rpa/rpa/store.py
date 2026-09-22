from __future__ import annotations

import json
import time
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional

from . import config


def _now() -> float:
    return time.time()


def _write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    tmp.replace(path)


def _read_json(path: Path) -> Optional[Any]:
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def new_id(prefix: str) -> str:
    return "{}-{}".format(prefix, uuid.uuid4().hex[:8])


def append_event(kind: str, payload: Dict[str, Any]) -> None:
    config.ensure_dirs()
    line = json.dumps({"ts": _now(), "kind": kind, "payload": payload}, ensure_ascii=False)
    with open(config.EVENTS_FILE, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def list_events(limit: int = 100) -> List[Dict[str, Any]]:
    if not config.EVENTS_FILE.exists():
        return []
    lines = config.EVENTS_FILE.read_text(encoding="utf-8").splitlines()
    out = []
    for ln in lines[-limit:]:
        try:
            out.append(json.loads(ln))
        except json.JSONDecodeError:
            continue
    return out


# ---------- 任务 ----------

def save_task(spec: Dict[str, Any]) -> Dict[str, Any]:
    config.ensure_dirs()
    spec.setdefault("task_id", new_id("task"))
    spec["updated_at"] = _now()
    spec.setdefault("created_at", spec["updated_at"])
    spec.setdefault("schema", config.SCHEMA)
    spec.setdefault("script_version", 0)
    spec.setdefault("heal", {"enabled": True, "max_attempts": config.HEAL_MAX_ATTEMPTS})
    _write_json(config.TASKS_DIR / (spec["task_id"] + ".json"), spec)
    return spec


def get_task(task_id: str) -> Optional[Dict[str, Any]]:
    return _read_json(config.TASKS_DIR / (task_id + ".json"))


def list_tasks() -> List[Dict[str, Any]]:
    config.ensure_dirs()
    return [t for t in (_read_json(p) for p in sorted(config.TASKS_DIR.glob("*.json"))) if t]


def update_task(task_id: str, patch: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    spec = get_task(task_id)
    if spec is None:
        return None
    spec.update(patch)
    return save_task(spec)


# ---------- 脚本版本 ----------

def save_script(task_id: str, code: str, source: str, note: str = "") -> int:
    root = config.SCRIPTS_DIR / task_id
    root.mkdir(parents=True, exist_ok=True)
    existing = sorted(int(p.stem[1:]) for p in root.glob("v*.py")) if root.exists() else []
    version = (existing[-1] + 1) if existing else 1
    (root / "v{}.py".format(version)).write_text(code, encoding="utf-8")
    meta = _read_json(root / "meta.json") or []
    meta.append({"version": version, "source": source, "note": note, "ts": _now()})
    _write_json(root / "meta.json", meta)
    update_task(task_id, {"script_version": version})
    append_event("script.saved", {"task_id": task_id, "version": version, "source": source})
    return version


def get_script(task_id: str, version: Optional[int] = None) -> Optional[str]:
    root = config.SCRIPTS_DIR / task_id
    if version is None:
        spec = get_task(task_id)
        version = (spec or {}).get("script_version", 0)
    path = root / "v{}.py".format(version)
    return path.read_text(encoding="utf-8") if path.exists() else None


def script_history(task_id: str) -> List[Dict[str, Any]]:
    return _read_json(config.SCRIPTS_DIR / task_id / "meta.json") or []


# ---------- 流程 ----------

def save_flow(flow: Dict[str, Any]) -> Dict[str, Any]:
    config.ensure_dirs()
    flow.setdefault("flow_id", new_id("flow"))
    flow["updated_at"] = _now()
    _write_json(config.FLOWS_DIR / (flow["flow_id"] + ".json"), flow)
    return flow


def get_flow(flow_id: str) -> Optional[Dict[str, Any]]:
    return _read_json(config.FLOWS_DIR / (flow_id + ".json"))


def list_flows() -> List[Dict[str, Any]]:
    config.ensure_dirs()
    return [f for f in (_read_json(p) for p in sorted(config.FLOWS_DIR.glob("*.json"))) if f]


def delete_flow(flow_id: str) -> bool:
    path = config.FLOWS_DIR / (flow_id + ".json")
    if not path.exists():
        return False
    path.unlink()
    append_event("flow.deleted", {"flow_id": flow_id})
    return True


# ---------- 运行记录 ----------

def save_run(record: Dict[str, Any]) -> Dict[str, Any]:
    config.ensure_dirs()
    record.setdefault("run_id", new_id("run"))
    record["ts"] = _now()
    _write_json(config.RUNS_DIR / (record["run_id"] + ".json"), record)
    append_event("run.finished", {
        "run_id": record["run_id"], "task_id": record.get("task_id"),
        "status": record.get("status"), "version": record.get("script_version"),
    })
    return record


def list_runs(task_id: Optional[str] = None, limit: int = 50) -> List[Dict[str, Any]]:
    config.ensure_dirs()
    runs = [r for r in (_read_json(p) for p in sorted(config.RUNS_DIR.glob("*.json"), reverse=True)) if r]
    if task_id:
        runs = [r for r in runs if r.get("task_id") == task_id]
    return runs[:limit]
