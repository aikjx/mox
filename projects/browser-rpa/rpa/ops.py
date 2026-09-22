from __future__ import annotations

import asyncio
import time
from typing import Dict, Optional

from . import healer, store


def health() -> Dict:
    tasks = store.list_tasks()
    runs = store.list_runs(limit=200)
    passed = sum(1 for r in runs if r.get("status") == "passed")
    failed = sum(1 for r in runs if r.get("status") == "failed")
    return {
        "status": "ok",
        "service": "browser-rpa",
        "uptime_hint": bool(runs),
        "tasks": len(tasks),
        "scheduled": sum(1 for t in tasks if (t.get("schedule") or {}).get("interval_sec")),
        "runs_recent": {"passed": passed, "failed": failed},
        "last_error_rounds": [r.get("run_id") for r in runs if r.get("status") == "failed"][:5],
        "ts": time.time(),
    }


class Scheduler:
    """轻量间隔调度：到点自动执行任务，失败进入自愈闭环，全程事件留痕。"""

    def __init__(self, tick_sec: int = 20):
        self.tick_sec = tick_sec
        self._next_due: Dict[str, float] = {}
        self._task: Optional[asyncio.Task] = None
        self._running_ids = set()

    def _due_tasks(self, now: float) -> list:
        due = []
        for t in store.list_tasks():
            interval = (t.get("schedule") or {}).get("interval_sec")
            if not interval or t["task_id"] in self._running_ids:
                continue
            nxt = self._next_due.get(t["task_id"])
            if nxt is None:
                self._next_due[t["task_id"]] = now + interval
                continue
            if now >= nxt:
                self._next_due[t["task_id"]] = now + interval
                due.append(t)
        return due

    async def tick_once(self, now: float = None) -> int:
        now = now if now is not None else time.time()
        fired = 0
        for t in self._due_tasks(now):
            fired += 1
            asyncio.get_event_loop().run_in_executor(None, self._execute, t["task_id"])
        return fired

    def _execute(self, task_id: str) -> None:
        self._running_ids.add(task_id)
        try:
            spec = store.get_task(task_id) or {}
            heal_conf = spec.get("heal") or {}
            if heal_conf.get("enabled", True):
                result = healer.heal(task_id, max_attempts=heal_conf.get("max_attempts"))
            else:
                from . import runner
                code = store.get_script(task_id)
                r = runner.run_script(code or "")
                store.save_run({"task_id": task_id, "status": "passed" if r["ok"] else "failed",
                                "stderr": r["stderr"]})
                result = {"ok": r["ok"]}
            store.append_event("ops.scheduled_run", {"task_id": task_id, "ok": result.get("ok")})
        except Exception as e:  # 调度器不能因单次异常退出
            store.append_event("ops.error", {"task_id": task_id, "error": str(e)})
        finally:
            self._running_ids.discard(task_id)

    async def run_forever(self) -> None:
        store.append_event("ops.scheduler_started", {"tick_sec": self.tick_sec})
        while True:
            try:
                await self.tick_once()
            except Exception as e:
                store.append_event("ops.error", {"error": str(e)})
            await asyncio.sleep(self.tick_sec)

    def start(self) -> None:
        if self._task is None or self._task.done():
            self._task = asyncio.get_event_loop().create_task(self.run_forever())


scheduler = Scheduler()
