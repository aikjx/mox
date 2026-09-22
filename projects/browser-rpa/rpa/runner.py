from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Dict

from . import config

# 判定为「页面变动类」失败的特征（自愈触发条件）
DRIFT_SIGNATURES = (
    "Timeout", "waiting for locator", "strict mode violation", "no element found",
    "ElementNotFound", "not found", "Execution context was destroyed",
    "Target closed", "unknown option", "Selector ",
)


def run_script(code: str, env_extra: Dict[str, str] = None) -> Dict:
    """子进程执行 Python RPA 脚本，返回 {ok, exit_code, stdout, stderr, drift_suspect}。"""
    with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False, encoding="utf-8") as f:
        f.write(code)
        path = f.name
    try:
        proc = subprocess.run(
            [sys.executable, path],
            capture_output=True, text=True, timeout=config.RUN_TIMEOUT_SEC,
            env=_env(env_extra), cwd=str(Path(path).parent),
        )
        out, err, rc = proc.stdout, proc.stderr, proc.returncode
    except subprocess.TimeoutExpired:
        out, err, rc = "", "RUN TIMEOUT after {}s".format(config.RUN_TIMEOUT_SEC), -9
    finally:
        Path(path).unlink(missing_ok=True)
    text = (out or "") + (err or "")
    return {
        "ok": rc == 0, "exit_code": rc, "stdout": out[-8000:], "stderr": err[-8000:],
        "drift_suspect": rc != 0 and any(s.lower() in text.lower() for s in DRIFT_SIGNATURES),
    }


def _env(extra: Dict[str, str] = None) -> Dict[str, str]:
    import os
    env = dict(os.environ)
    env.setdefault("PYTHONIOENCODING", "utf-8")
    if extra:
        env.update(extra)
    return env
