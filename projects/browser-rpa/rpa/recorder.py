from __future__ import annotations

import subprocess
import sys
import tempfile
import threading
from pathlib import Path
from typing import Dict, Optional

from . import store

# 录制会话表：task_id -> {"proc": Popen, "out": Path}
_sessions: Dict[str, Dict] = {}
_lock = threading.Lock()


def start(task_id: str, url: str = "", headed: bool = True) -> Dict:
    """启动 playwright codegen 录制，退出时产出 Python 脚本。

    codegen 需要有头浏览器；容器内由 Xvfb 提供显示（见 Dockerfile）。
    """
    with _lock:
        if task_id in _sessions:
            return {"ok": False, "error": "已有录制会话在运行"}
        out_file = Path(tempfile.mkdtemp(prefix="rpa-rec-")) / "recorded.py"
        cmd = [sys.executable, "-m", "playwright", "codegen", "--target", "python",
               "--output", str(out_file)]
        if not headed:
            cmd += ["--no-viewport"]
        if url:
            cmd.append(url)
        proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
        _sessions[task_id] = {"proc": proc, "out": out_file}
        store.append_event("record.started", {"task_id": task_id, "url": url})
        return {"ok": True, "pid": proc.pid, "out": str(out_file)}


def stop(task_id: str) -> Dict:
    with _lock:
        session = _sessions.pop(task_id, None)
    if not session:
        return {"ok": False, "error": "无录制会话，请先调用 record/start"}
    proc: subprocess.Popen = session["proc"]
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
    code = session["out"].read_text(encoding="utf-8") if session["out"].exists() else ""
    if not code.strip():
        store.append_event("record.empty", {"task_id": task_id})
        return {"ok": False, "error": "录制产物为空（可能未产生任何操作）"}
    version = store.save_script(task_id, code, source="codegen", note="录制导入")
    store.append_event("record.stopped", {"task_id": task_id, "script_version": version})
    return {"ok": True, "script_version": version, "chars": len(code)}


def running(task_id: str) -> bool:
    with _lock:
        return task_id in _sessions
