from __future__ import annotations

import os
from pathlib import Path

PROJECT_DIR = Path(__file__).resolve().parent.parent
DATA_DIR = Path(os.environ.get("RPA_DATA_DIR", PROJECT_DIR / "data"))
SCRIPTS_DIR = DATA_DIR / "scripts"
TASKS_DIR = DATA_DIR / "tasks"
RUNS_DIR = DATA_DIR / "runs"
FLOWS_DIR = DATA_DIR / "flows"
SNAPSHOTS_DIR = DATA_DIR / "snapshots"
EVENTS_FILE = DATA_DIR / "events.jsonl"

HOST = os.environ.get("RPA_HOST", "0.0.0.0")
PORT = int(os.environ.get("RPA_PORT", "30400"))

# LLM：OpenAI 兼容端点；未配置时各模块走离线规则回退
LLM_BASE_URL = os.environ.get("RPA_LLM_BASE_URL", "").rstrip("/")
LLM_API_KEY = os.environ.get("RPA_LLM_API_KEY", "")
LLM_MODEL = os.environ.get("RPA_LLM_MODEL", "gpt-4o-mini")
LLM_TIMEOUT = float(os.environ.get("RPA_LLM_TIMEOUT", "60"))

HEADLESS = os.environ.get("RPA_HEADLESS", "1") != "0"
RUN_TIMEOUT_SEC = int(os.environ.get("RPA_RUN_TIMEOUT", "120"))
HEAL_MAX_ATTEMPTS = int(os.environ.get("RPA_HEAL_MAX_ATTEMPTS", "3"))
SCHEMA = "RPA-TASKSPEC-V1"


def ensure_dirs() -> None:
    for d in (SCRIPTS_DIR, TASKS_DIR, RUNS_DIR, FLOWS_DIR, SNAPSHOTS_DIR):
        d.mkdir(parents=True, exist_ok=True)
