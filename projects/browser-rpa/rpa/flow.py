from __future__ import annotations

import json
import re
from typing import Any, Dict, List, Optional

from . import config, store


def _q(s: Any) -> str:
    return json.dumps("" if s is None else str(s), ensure_ascii=False)

# 模块注册表：流程 = 有序步骤，每步引用一个模块。新增模块只需在此登记。
# compile: args → Playwright(Python) 代码行（操作变量 page）。
MODULES: Dict[str, Dict[str, Any]] = {
    "goto": {
        "label": "打开网址", "params": {"url": "str"},
        "compile": lambda a: 'page.goto({}, timeout=30000)'.format(_q(a.get("url", ""))),
        "import_re": r"""page\.goto\(["'](?P<url>[^"']+)["']""",
    },
    "click": {
        "label": "点击元素", "params": {"locator": "{method:role|text|label|placeholder|css, expr, role?, name?}"},
        "compile": lambda a: '{}.click(timeout=15000)'.format(_loc(a.get("locator", {}))),
    },
    "fill": {
        "label": "输入文本", "params": {"locator": "同 click", "value": "str"},
        "compile": lambda a: '{}.fill({}, timeout=15000)'.format(_loc(a.get("locator", {})), _q(a.get("value", ""))),
    },
    "press": {
        "label": "按键", "params": {"key": "str", "locator": "可选，缺省为整页"},
        "compile": lambda a: ('page.keyboard.press({})'.format(_q(a.get("key", "Enter")))
                              if not a.get("locator") else
                              '{}.press({})'.format(_loc(a["locator"]), _q(a.get("key", "Enter")))),
    },
    "wait": {
        "label": "等待", "params": {"ms": "int"},
        "compile": lambda a: 'page.wait_for_timeout({})'.format(int(a.get("ms", 1000))),
        "import_re": r"""page\.wait_for_timeout\((?P<ms>\d+)\)""",
    },
    "assert_text": {
        "label": "断言文本", "params": {"locator": "同 click", "expected": "str"},
        "compile": lambda a: ('assert {} in {}.inner_text(timeout=10000), "断言失败: " + {}'.format(
            _q(a.get("expected", "")), _loc(a.get("locator", {})), _q(a.get("expected", "")))),
    },
    "extract": {
        "label": "抓取文本", "params": {"locator": "同 click", "save_as": "str"},
        "compile": lambda a: 'print("[extract] {} = " + {}.inner_text(timeout=10000))'.format(
            a.get("save_as", "field"), _loc(a.get("locator", {}))),
    },
    "screenshot": {
        "label": "截图", "params": {"path": "str"},
        "compile": lambda a: 'page.screenshot(path={})'.format(_q(a.get("path", "screen.png"))),
    },
    "custom": {
        "label": "自定义代码", "params": {"code": "str（单行/多行 Playwright 语句）"},
        "compile": lambda a: str(a.get("code", "pass")),
    },
}

_METHOD_BUILDERS = {
    "role": lambda l: ('page.get_by_role({}, name={})'.format(_q(l.get("role", "button")), _q(l.get("name", "")))
                       if l.get("name") else 'page.get_by_role({})'.format(_q(l.get("role", "button")))),
    "text": lambda l: 'page.get_by_text({})'.format(_q(l.get("expr", ""))),
    "label": lambda l: 'page.get_by_label({})'.format(_q(l.get("expr", ""))),
    "placeholder": lambda l: 'page.get_by_placeholder({})'.format(_q(l.get("expr", ""))),
    "css": lambda l: 'page.locator({})'.format(_q(l.get("expr", ""))),
}


def _loc(locator: Dict[str, Any]) -> str:
    method = locator.get("method", "css")
    builder = _METHOD_BUILDERS.get(method)
    if builder is None:
        return _METHOD_BUILDERS["css"](locator)
    return builder(locator)


def modules_spec() -> List[Dict[str, Any]]:
    return [{"module": k, "label": v["label"], "params": v["params"]} for k, v in MODULES.items()]


def compile_step(step: Dict[str, Any]) -> str:
    if not step.get("enabled", True):
        return "# [skip] " + step.get("module", "?")
    mod = MODULES.get(step.get("module", ""))
    if mod is None:
        return "# 未知模块: {}".format(step.get("module"))
    try:
        code = mod["compile"](step.get("args", {}) or {})
    except Exception as e:
        return "# 编译失败({}): {}".format(step.get("module"), e)
    return "\n".join("    " + ln for ln in code.splitlines())


def compile_flow(flow: Dict[str, Any], headless: bool = True) -> str:
    steps = flow.get("steps", [])
    body = "\n".join(compile_step(s) for s in steps)
    return (
        "# 由 browser-rpa 流程管理台编译 · flow={fid} v{ver} · {n} 步\n"
        "from playwright.sync_api import sync_playwright\n\n\n"
        "def run(playwright) -> None:\n"
        "    browser = playwright.chromium.launch(headless={hl})\n"
        "    page = browser.new_page()\n"
        "{body}\n"
        "    browser.close()\n\n\n"
        "with sync_playwright() as playwright:\n"
        "    run(playwright)\n"
    ).format(fid=flow.get("flow_id", "?"), ver=flow.get("version", 1), n=len(steps),
             hl="True" if headless else "False", body=body)


# ---------- 脚本 → 流程（导入既有录制脚本，实现流程化） ----------

_IMPORT_PATTERNS = [
    ("goto", r"""page\.goto\(["'](?P<url>[^"']+)["']""", lambda g: {"url": g["url"]}),
    ("wait", r"""page\.wait_for_timeout\((?P<ms>\d+)\)""", lambda g: {"ms": int(g["ms"])}),
    ("click", r"""page\.get_by_role\(["'](?P<role>[^"']+)["'](?:,\s*name=["'](?P<name>[^"']*)["'])?\)\.click""",
     lambda g: {"locator": {"method": "role", "role": g["role"], "name": g.get("name", "")}}),
    ("click", r"""page\.get_by_text\(["'](?P<expr>[^"']+)["']\)\.click""",
     lambda g: {"locator": {"method": "text", "expr": g["expr"]}}),
    ("click", r"""page\.locator\(["'](?P<expr>[^"']+)["']\)\.click""",
     lambda g: {"locator": {"method": "css", "expr": g["expr"]}}),
    ("fill", r"""page\.get_by_label\(["'](?P<expr>[^"']+)["']\)\.fill\(["'](?P<value>[^"']*)["']""",
     lambda g: {"locator": {"method": "label", "expr": g["expr"]}, "value": g["value"]}),
    ("fill", r"""page\.get_by_placeholder\(["'](?P<expr>[^"']+)["']\)\.fill\(["'](?P<value>[^"']*)["']""",
     lambda g: {"locator": {"method": "placeholder", "expr": g["expr"]}, "value": g["value"]}),
    ("fill", r"""page\.get_by_role\(["'](?P<role>[^"']+)["'](?:,\s*name=["'](?P<name>[^"']*)["'])?\)\.fill\(["'](?P<value>[^"']*)["']""",
     lambda g: {"locator": {"method": "role", "role": g["role"], "name": g.get("name", "")}, "value": g["value"]}),
    ("fill", r"""page\.locator\(["'](?P<expr>[^"']+)["']\)\.fill\(["'](?P<value>[^"']*)["']""",
     lambda g: {"locator": {"method": "css", "expr": g["expr"]}, "value": g["value"]}),
]


_BOILERPLATE = re.compile(
    r"^(import\b|from\b|def\b|with\b|return\b|assert\b\s|#"
    r"|\s*(browser|context|page)\s*=\s*\w+"
    r"|context\.close\(\)|page\.close\(\)|browser\.close\(\)|\})")


def import_script(code: str) -> List[Dict[str, Any]]:
    """把录制/手工 Python 脚本尽量拆解为流程步骤；不可识别的浏览器操作行归入 custom 模块。"""
    steps: List[Dict[str, Any]] = []
    leftovers: List[str] = []
    for ln in code.splitlines():
        s = ln.strip()
        if not s or _BOILERPLATE.match(s):
            continue
        matched = False
        for module, pat, build in _IMPORT_PATTERNS:
            m = re.search(pat, s)
            if m:
                if leftovers:
                    steps.append(_mk("custom", {"code": "\n".join(leftovers)}))
                    leftovers = []
                steps.append(_mk(module, build(m.groupdict()), note=s[:80]))
                matched = True
                break
        if not matched and s.startswith(("page.", "context.", "browser.", "expect(")):
            leftovers.append(s)
    if leftovers:
        steps.append(_mk("custom", {"code": "\n".join(leftovers)}))
    return steps


def _mk(module: str, args: Dict[str, Any], note: str = "") -> Dict[str, Any]:
    return {"id": store.new_id("st"), "module": module, "args": args, "enabled": True, "note": note}


# ---------- CRUD ----------

def create(name: str, description: str = "", steps: List[Dict] = None) -> Dict[str, Any]:
    return store.save_flow({
        "schema": "RPA-FLOW-V1", "name": name, "description": description,
        "steps": steps or [], "version": 1, "tags": [],
    })


def list_flows() -> List[Dict[str, Any]]:
    return store.list_flows()


def get(flow_id: str) -> Optional[Dict[str, Any]]:
    return store.get_flow(flow_id)


def update(flow_id: str, patch: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    flow = store.get_flow(flow_id)
    if flow is None:
        return None
    flow.update(patch)
    flow["version"] = flow.get("version", 1) + 1
    return store.save_flow(flow)


def delete(flow_id: str) -> bool:
    return store.delete_flow(flow_id)
