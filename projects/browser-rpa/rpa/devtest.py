from __future__ import annotations

import time
from typing import Any, Dict, List, Optional

from . import config, flow as flow_mod

# 步骤级测试执行引擎：与 compile 共用同一份模块代码生成（单一事实源），
# 逐步 exec 编译产物，page 由外部注入（真实 Playwright 或测试替身）。


def run_step(step: Dict[str, Any], page: Any) -> Dict[str, Any]:
    code = flow_mod.MODULES[step["module"]]["compile"](step.get("args", {}) or {})
    started = time.time()
    result = {"step_id": step.get("id"), "module": step["module"], "ok": True, "error": "",
              "code": code, "duration_ms": 0}
    try:
        exec(compile(code, "<flow-step>", "exec"), {"page": page})
    except Exception as e:
        result["ok"] = False
        result["error"] = "{}: {}".format(type(e).__name__, e)
    finally:
        result["duration_ms"] = int((time.time() - started) * 1000)
    return result


def test_flow(flow: Dict[str, Any], headless: bool = True, start: int = 0,
              stop_at: Optional[int] = None) -> Dict[str, Any]:
    """真实分步调试：同一浏览器会话内从 start 逐步执行，失败即停并截图。"""
    steps = flow.get("steps", [])
    results: List[Dict[str, Any]] = []
    pw = None
    page = None
    try:
        from playwright.sync_api import sync_playwright
        pw = sync_playwright().start()
        browser = pw.chromium.launch(headless=headless or config.HEADLESS)
        page = browser.new_page()
    except Exception as e:
        return {"ok": False, "results": [], "error": "浏览器启动失败: {}".format(e)}
    try:
        for i, step in enumerate(steps):
            if i < start:
                continue
            if not step.get("enabled", True):
                results.append({"step_index": i, "step_id": step.get("id"),
                                "module": step.get("module"), "ok": True, "skipped": True})
                continue
            r = run_step(step, page)
            r["step_index"] = i
            if not r["ok"] and page is not None:
                try:
                    shot = config.SNAPSHOTS_DIR / "devtest-{}-{}.png".format(
                        flow.get("flow_id", "x"), int(time.time()))
                    config.ensure_dirs()
                    page.screenshot(path=str(shot))
                    r["screenshot"] = str(shot)
                except Exception:
                    pass
            results.append(r)
            if not r["ok"] or (stop_at is not None and i >= stop_at):
                break
        ok = all(r.get("ok", True) for r in results)
        return {"ok": ok, "results": results, "steps_total": len(steps)}
    finally:
        try:
            if page is not None:
                page.context.browser.close()
        except Exception:
            pass
        try:
            if pw is not None:
                pw.stop()
        except Exception:
            pass
