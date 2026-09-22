from __future__ import annotations

import difflib
import re
import time
from typing import Dict, List, Optional

import httpx

from . import config, llm, runner, store

# 匹配 codegen 产出的定位器调用，如 page.get_by_role("button", name="登 录")
LOCATOR_RE = re.compile(
    r"""(get_by_role|get_by_text|get_by_label|get_by_placeholder|locator)\((.*?)\)""")
ATTR_RE = re.compile(r"""(name|label|placeholder|text|role)="(.*?)"|'(.*?)'""")

HEAL_SYSTEM = (
    "你是浏览器 RPA 脚本修复专家。给定一段 Playwright(Python) 脚本、执行失败的报错、以及相关页面的"
    "最新 HTML 快照，请修复失效的定位器/等待逻辑（页面元素可能改名、换 id、换层级）。"
    "保持业务语义与整体结构不变，只做出错处的最小修复。只输出完整修复后的 Python 代码，用 ```python 围栏包裹。"
)


def fetch_snapshot(url: str) -> str:
    """抓取页面 HTML 快照供 AI 参照；优先 Playwright 渲染，回退静态 GET。"""
    if not url:
        return ""
    try:
        from playwright.sync_api import sync_playwright
        with sync_playwright() as p:
            browser = p.chromium.launch(headless=config.HEADLESS)
            page = browser.new_page()
            page.goto(url, timeout=30000, wait_until="domcontentloaded")
            html = page.content()
            browser.close()
        return html[:200000]
    except Exception:
        try:
            resp = httpx.get(url, timeout=15, follow_redirects=True,
                             headers={"User-Agent": "Mozilla/5.0 rpa-healer"})
            return resp.text[:200000]
        except Exception:
            return ""


def _failed_locator_from_error(stderr: str) -> Optional[str]:
    # Playwright 报错形如 waiting for get_by_role("link", name="登 录") / waiting for locator "#x"，
    # 取 waiting 行最后一个引号参数作为失效定位目标（role 等首个参数不是页面文案）
    m = re.search(r"""waiting for (?:locator|get_by_\w+)[^\n]*""", stderr or "")
    if m:
        quoted = re.findall(r"""["']([^"']{1,120})["']""", m.group(0))
        if quoted:
            return quoted[-1]
        bare = re.search(r"""locator\s+["'](?P<loc>[^"']+)["']""", m.group(0))
        if bare:
            return bare.group("loc")
    m = re.search(r"""Error: (?:locator|page\.[\w.]+\((?P<q>["'])(?P<loc>.*?)(?P=q))""", stderr or "")
    return m.group("loc") if m else None


def _attributes_from_html(html: str) -> List[str]:
    keys = ("id", "name", "placeholder", "aria-label", "title")
    out = []
    for k in keys:
        out += [m.group(1) for m in re.finditer(k + r'="([^"]{1,80})"', html)]
    out += [re.sub(r"\s+", " ", t).strip()[:60]
            for t in re.findall(r"<(?:a|button|label|span)[^>]*>([^<>]{1,60})</", html)]
    return [a for a in out if a.strip()]


def suggest_locator(old: str, html: str) -> Optional[str]:
    """离线回退修复：在最新快照中找与失效定位器最接近的属性值（difflib 模糊匹配）。"""
    if not html or not old:
        return None
    candidates = _attributes_from_html(html)
    if not candidates:
        return None
    old_core = re.sub(r"^(#|\.|\[.*?\])", "", old).strip()
    exact = [c for c in candidates if c == old_core]
    if exact:
        return None  # 定位目标还在，问题可能在别处，交给人工/AI
    matches = difflib.get_close_matches(old_core, candidates, n=1, cutoff=0.4)
    if not matches:
        substr = [c for c in candidates if old_core and (old_core in c or c in old_core)]
        return substr[0] if substr else None
    return matches[0]


def offline_patch(code: str, stderr: str, html: str) -> Optional[str]:
    """把报错里失效的定位器文本替换为快照中模糊匹配到的新值。"""
    failed = _failed_locator_from_error(stderr)
    if not failed:
        m = LOCATOR_RE.search(code or "")
        if not m:
            return None
        am = re.search(r"""["']([^"']+)["']""", m.group(2) or "")
        failed = am.group(1) if am else None
    if not failed:
        return None
    new = suggest_locator(failed, html)
    if not new or new == failed:
        return None
    patched = code.replace('"' + failed + '"', '"' + new + '"') \
                  .replace("'" + failed + "'", "'" + new + "'")
    return patched if patched != code else None


def ai_patch(code: str, stderr: str, html_by_url: Dict[str, str]) -> Optional[str]:
    if not llm.available():
        return None
    snapshots = "\n\n".join("### {} 快照\n{}".format(u, h[:60000]) for u, h in html_by_url.items() if h)
    user = "脚本：\n```python\n{}\n```\n\n报错：\n{}\n\n{}".format(code, stderr[-4000:], snapshots)
    reply = llm.chat([{"role": "system", "content": HEAL_SYSTEM}, {"role": "user", "content": user}])
    if not reply:
        return None
    m = re.search(r"```(?:python)?\s*(.*?)```", reply, re.S)
    fixed = m.group(1) if m else reply
    fixed = fixed.strip() + "\n"
    try:
        compile(fixed, "<healed>", "exec")
    except SyntaxError:
        return None
    return fixed if fixed != code else None


def urls_in(code: str) -> List[str]:
    return list(dict.fromkeys(re.findall(r"https?://[^\s'\"]+", code or "")))[:5]


def heal(task_id: str, max_attempts: int = None) -> Dict:
    """自愈闭环：执行→失败且疑似页面变动→抓快照→AI/离线改脚本→存新版→复跑。"""
    max_attempts = max_attempts if max_attempts is not None else config.HEAL_MAX_ATTEMPTS
    attempts: List[Dict] = []
    for i in range(max_attempts):
        code = store.get_script(task_id)
        if code is None:
            return {"ok": False, "error": "无脚本可执行，请先录制", "attempts": attempts}
        result = runner.run_script(code)
        record = store.save_run({"task_id": task_id, "status": "passed" if result["ok"] else "failed",
                                 "script_version": (store.get_task(task_id) or {}).get("script_version"),
                                 "stderr": result["stderr"], "heal_round": i})
        attempts.append({"round": i, "ok": result["ok"], "run_id": record["run_id"]})
        if result["ok"]:
            store.append_event("heal.converged", {"task_id": task_id, "round": i})
            return {"ok": True, "attempts": attempts, "run_id": record["run_id"]}
        if not result["drift_suspect"]:
            store.append_event("heal.aborted", {"task_id": task_id, "reason": "非页面变动类失败"})
            return {"ok": False, "error": "失败原因疑似非页面变动，不触发自愈", "attempts": attempts}
        html_by_url = {u: fetch_snapshot(u) for u in urls_in(code)}
        config.ensure_dirs()
        for u, h in html_by_url.items():
            snap_path = config.SNAPSHOTS_DIR / "{}-{}.html".format(task_id, int(time.time()))
            snap_path.write_text(h, encoding="utf-8")
        fixed = ai_patch(code, result["stderr"], html_by_url)
        source = "ai-heal"
        if fixed is None:
            fixed = offline_patch(code, result["stderr"], " ".join(html_by_url.values()))
            source = "offline-heal"
        if fixed is None:
            store.append_event("heal.no_patch", {"task_id": task_id, "round": i})
            return {"ok": False, "error": "未能生成修复补丁", "attempts": attempts}
        version = store.save_script(task_id, fixed, source=source, note="第{}轮自愈".format(i + 1))
        store.append_event("heal.patched", {"task_id": task_id, "round": i, "script_version": version})
    return {"ok": False, "error": "达到自愈次数上限仍未通过", "attempts": attempts}
