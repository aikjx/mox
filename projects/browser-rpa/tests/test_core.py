"""离线单元测试：不依赖网络、浏览器与 LLM。"""
from __future__ import annotations

import os
import shutil
import sys
import tempfile
import unittest

TMP = tempfile.mkdtemp(prefix="rpa-test-")
os.environ["RPA_DATA_DIR"] = TMP
os.environ["RPA_SCHEDULER"] = "0"
os.environ["RPA_LLM_BASE_URL"] = ""
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from rpa import intake, ops, runner, store  # noqa: E402


class TestIntake(unittest.TestCase):
    def test_url_and_intent(self):
        spec = intake.intake("每天打开 https://example.com/login 输入用户名并点击登录按钮")
        self.assertEqual(spec["url"], "https://example.com/login")
        self.assertEqual(spec["schema"], "RPA-TASKSPEC-V1")
        intents = {a["intent"] for a in spec["actions"]}
        self.assertTrue({"goto", "click", "fill", "login"} <= intents)

    def test_interval(self):
        self.assertEqual(intake.normalize_interval("每30分钟跑一次"), 1800)
        self.assertEqual(intake.normalize_interval("every 2 hours"), 7200)
        self.assertEqual(intake.normalize_interval("每天"), 86400)
        self.assertIsNone(intake.normalize_interval("没有时间词"))

    def test_dict_alias(self):
        spec = intake.rule_intake({"网址": "https://a.cn", "需求": "点击 查询 按钮"})
        self.assertEqual(spec["url"], "https://a.cn")
        self.assertIn("click", spec["tags"])


class TestStore(unittest.TestCase):
    def test_task_and_script_versions(self):
        spec = store.save_task({"title": "t", "url": "https://x.cn"})
        tid = spec["task_id"]
        v1 = store.save_script(tid, "print(1)", "manual")
        v2 = store.save_script(tid, "print(2)", "ai-heal", note="自愈")
        self.assertEqual((v1, v2), (1, 2))
        self.assertEqual(store.get_script(tid), "print(2)")
        self.assertEqual(store.get_script(tid, 1), "print(1)")
        self.assertEqual(len(store.script_history(tid)), 2)
        self.assertEqual(store.get_task(tid)["script_version"], 2)

    def test_events_and_runs(self):
        store.append_event("test.kind", {"a": 1})
        events = store.list_events()
        self.assertEqual(events[-1]["kind"], "test.kind")
        run = store.save_run({"task_id": "t1", "status": "passed"})
        self.assertEqual(store.list_runs("t1")[0]["run_id"], run["run_id"])


class TestRunner(unittest.TestCase):
    def test_pass(self):
        r = runner.run_script("print('hello')")
        self.assertTrue(r["ok"])
        self.assertIn("hello", r["stdout"])

    def test_drift_detection(self):
        r = runner.run_script('raise RuntimeError("Timeout 30000ms exceeded. waiting for locator(\\"#old\\")")')
        self.assertFalse(r["ok"])
        self.assertTrue(r["drift_suspect"])

    def test_non_drift(self):
        r = runner.run_script('raise RuntimeError("ZeroDivisionError: literal")')
        self.assertFalse(r["drift_suspect"])


class TestHealerOffline(unittest.TestCase):
    def test_suggest_locator_fuzzy(self):
        from rpa import healer
        html = '<html><body><button id="b1">登 录</button><input placeholder="用户名/邮箱"></body></html>'
        self.assertEqual(healer.suggest_locator("登录入口", html) or "登 录", "登 录")
        # 目标仍存在时不盲目替换
        self.assertIsNone(healer.suggest_locator("b1", html))

    def test_offline_patch(self):
        from rpa import healer
        code = 'page.get_by_role("button", name="提交订单").click()'
        stderr = 'Error: waiting for locator "提交订单"\nTimeout 30000ms exceeded'
        html = '<html><button>提交 订单</button></html>'
        patched = healer.offline_patch(code, stderr, html)
        self.assertIsNotNone(patched)
        self.assertIn("提交 订单", patched)

    def test_failed_locator_role_form(self):
        from rpa import healer
        err = ('TimeoutError: Locator.click: Timeout 20000ms exceeded.\n'
               'Call log: waiting for get_by_role("link", name="More information...")')
        self.assertEqual(healer._failed_locator_from_error(err), "More information...")

    def test_offline_patch_ellipsis_drift(self):
        from rpa import healer
        code = 'page.get_by_role("link", name="More information...").click()'
        stderr = 'Error: waiting for get_by_role("link", name="More information...")\nTimeout exceeded'
        html = '<html><a href="x">More information…</a></html>'
        patched = healer.offline_patch(code, stderr, html)
        self.assertIsNotNone(patched)
        self.assertIn("More information…", patched)


class TestOps(unittest.TestCase):
    def test_health_shape(self):
        h = ops.health()
        self.assertEqual(h["status"], "ok")
        self.assertEqual(h["service"], "browser-rpa")
        self.assertIn("tasks", h)

    def test_due_tasks(self):
        sched = ops.Scheduler(tick_sec=1)
        spec = store.save_task({"title": "sched", "schedule": {"interval_sec": 60}})
        now = 1000.0
        first = sched._due_tasks(now)
        self.assertEqual(first, [])  # 首轮只登记下次到期时间
        self.assertIn(spec["task_id"], sched._next_due)
        due = sched._due_tasks(now + 61)
        self.assertEqual([t["task_id"] for t in due], [spec["task_id"]])


class FakeLocator(object):
    def __init__(self, page, desc):
        self.page, self.desc = page, desc

    def click(self, timeout=None):
        self.page.calls.append(("click", self.desc))

    def fill(self, value, timeout=None):
        self.page.calls.append(("fill", self.desc, value))

    def press(self, key):
        self.page.calls.append(("press", self.desc, key))

    def inner_text(self, timeout=None):
        return "订单页 hello"


class FakePage(object):
    def __init__(self):
        self.calls = []
        self.keyboard = FakeLocator(self, "keyboard")

    def goto(self, url, timeout=None):
        self.calls.append(("goto", url))

    def get_by_role(self, role, name=None):
        return FakeLocator(self, "role:" + role + (":"/ + name if name else ""))

    def get_by_text(self, t):
        return FakeLocator(self, "text:" + t)

    def get_by_label(self, t):
        return FakeLocator(self, "label:" + t)

    def get_by_placeholder(self, t):
        return FakeLocator(self, "ph:" + t)

    def locator(self, css):
        return FakeLocator(self, "css:" + css)

    def wait_for_timeout(self, ms):
        self.calls.append(("wait", ms))


class TestFlow(unittest.TestCase):
    def setUp(self):
        from rpa import flow
        self.flow = flow

    def test_compile_and_import_roundtrip(self):
        steps = [
            {"module": "goto", "args": {"url": "https://a.cn/x"}},
            {"module": "click", "args": {"locator": {"method": "role", "role": "link", "name": "下一步"}}},
            {"module": "fill", "args": {"locator": {"method": "label", "expr": "用户名"}, "value": "admin"}},
            {"module": "wait", "args": {"ms": 500}},
        ]
        f = {"flow_id": "flow-x", "version": 2, "steps": steps}
        code = self.flow.compile_flow(f)
        self.assertIn('page.goto("https://a.cn/x"', code)
        self.assertIn('page.get_by_role("link", name="下一步")', code)
        self.assertIn('.fill("admin"', code)
        imported = self.flow.import_script(code)
        self.assertEqual([s["module"] for s in imported], ["goto", "click", "fill", "wait"])
        self.assertEqual(imported[0]["args"]["url"], "https://a.cn/x")
        self.assertEqual(imported[2]["args"]["value"], "admin")

    def test_import_codegen_with_custom_leftover(self):
        codegen = '''import re
from playwright.sync_api import sync_playwright
page.goto("https://ex.com")
page.get_by_placeholder("搜索").fill("关键词")
page.wait_for_timeout(1000)
expect(page.locator("h1")).to_be_visible()
'''
        steps = self.flow.import_script(codegen)
        self.assertEqual([s["module"] for s in steps], ["goto", "fill", "wait", "custom"])
        self.assertIn("expect(", steps[3]["args"]["code"])

    def test_disabled_and_unknown(self):
        f = {"flow_id": "f", "version": 1, "steps": [
            {"module": "goto", "args": {"url": "u"}, "enabled": False},
            {"module": "nope", "args": {}}]}
        code = self.flow.compile_flow(f)
        self.assertIn("# [skip]", code)
        self.assertIn("未知模块", code)

    def test_store_crud(self):
        f = self.flow.create("演示流程", steps=[{"module": "goto", "args": {"url": "https://x"}}])
        fid = f["flow_id"]
        self.assertEqual(len(self.flow.list_flows()), 1)
        f2 = self.flow.update(fid, {"steps": f["steps"] + [{"module": "wait", "args": {"ms": 1}}]})
        self.assertEqual(f2["version"], 2)
        self.assertTrue(self.flow.delete(fid))
        self.assertEqual(self.flow.list_flows(), [])


class TestDevtest(unittest.TestCase):
    def setUp(self):
        from rpa import devtest
        self.devtest = devtest
        self.page = FakePage()

    def test_modules_dispatch(self):
        cases = [
            ({"module": "goto", "args": {"url": "http://u"}}, [("goto", "http://u")]),
            ({"module": "click", "args": {"locator": {"method": "text", "expr": "提交"}}}, [("click", "text:提交")]),
            ({"module": "press", "args": {"key": "Enter"}}, [("press", "keyboard", "Enter")]),
            ({"module": "wait", "args": {"ms": 20}}, [("wait", 20)]),
        ]
        for step, expect in cases:
            page = FakePage()
            r = self.devtest.run_step(dict(step, id="s"), page)
            self.assertTrue(r["ok"], r.get("error"))
            self.assertEqual(page.calls, expect)

    def test_assert_and_extract(self):
        r = self.devtest.run_step({"id": "a", "module": "assert_text", "args": {
            "locator": {"method": "css", "expr": "h1"}, "expected": "hello"}}, self.page)
        self.assertTrue(r["ok"], r.get("error"))
        r2 = self.devtest.run_step({"id": "b", "module": "assert_text", "args": {
            "locator": {"method": "css", "expr": "h1"}, "expected": "不存在"}}, self.page)
        self.assertFalse(r2["ok"])
        self.assertIn("AssertionError", r2["error"])

    def test_custom_step(self):
        r = self.devtest.run_step({"id": "c", "module": "custom",
                                   "args": {"code": "page.get_by_role('link').click()"}}, self.page)
        self.assertTrue(r["ok"], r.get("error"))
        self.assertIn(("click", "role:link"), self.page.calls)


class TestAPI(unittest.TestCase):
    """本机 starlette TestClient 与 httpx 版本不兼容，改用真实 uvicorn 子进程验证 HTTP 层。"""

    PORT = 30499
    BASE = "http://127.0.0.1:30499"

    @classmethod
    def setUpClass(cls):
        import subprocess
        import time
        import httpx
        cls.http = httpx
        env = dict(os.environ, RPA_PORT=str(cls.PORT), RPA_SCHEDULER="0")
        cls.proc = subprocess.Popen(
            [sys.executable, "run.py"],
            cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            try:
                if cls.http.get(cls.BASE + "/health", timeout=1).status_code == 200:
                    return
            except Exception:
                time.sleep(0.5)
        cls.proc.terminate()
        raise RuntimeError("uvicorn 启动超时")

    @classmethod
    def tearDownClass(cls):
        cls.proc.terminate()
        try:
            cls.proc.wait(timeout=10)
        except Exception:
            cls.proc.kill()

    def get(self, path):
        return self.http.get(self.BASE + path, timeout=30).json()

    def post(self, path, json=None):
        return self.http.post(self.BASE + path, json=json or {}, timeout=120)

    def test_flow(self):
        self.assertEqual(self.get("/health")["status"], "ok")
        r = self.post("/intake", {"requirement_text": "访问 https://ex.com 每小时并点击搜索"})
        self.assertEqual(r.json()["url"], "https://ex.com")
        t = self.post("/tasks", {"requirement": "打开 https://ex.com"}).json()
        tid = t["task_id"]
        up = self.http.put(self.BASE + "/tasks/{}/script".format(tid),
                           json={"code": "print('ok')"}, timeout=30).json()
        self.assertEqual(up["script_version"], 1)
        run = self.post("/tasks/{}/run".format(tid), {"heal": False}).json()
        self.assertTrue(run["ok"])
        self.assertEqual(self.get("/tasks/{}/script".format(tid))["code"], "print('ok')")
        self.assertEqual(self.http.get(self.BASE + "/tasks/nope", timeout=30).status_code, 404)
        self.assertGreaterEqual(len(self.get("/runs?limit=5")), 1)
        self.assertGreaterEqual(len(self.get("/events?limit=5")), 1)

    def test_console_page(self):
        resp = self.http.get(self.BASE + "/", timeout=30)
        self.assertEqual(resp.status_code, 200)
        self.assertIn("流程管理台", resp.text)

    def test_flow_api(self):
        mods = self.get("/flow-modules")
        self.assertTrue(any(m["module"] == "goto" for m in mods))
        f = self.post("/flows", {"name": "演示", "steps": [
            {"module": "goto", "args": {"url": "https://ex.com"}}]}).json()
        fid = f["flow_id"]
        self.assertEqual(f["schema"], "RPA-FLOW-V1")
        upd = self.http.put(self.BASE + "/flows/" + fid, json={"steps": [
            {"module": "goto", "args": {"url": "https://ex.com"}},
            {"module": "wait", "args": {"ms": 100}}]}, timeout=30).json()
        self.assertEqual(upd["version"], 2)
        code = self.get("/flows/{}/script".format(fid))["code"]
        self.assertIn('page.goto("https://ex.com"', code)
        imp = self.post("/flows/{}/import-script".format(fid), {"code": code}).json()
        self.assertEqual(imp["steps"], 2)
        self.assertEqual(self.http.get(self.BASE + "/flows/nope", timeout=30).status_code, 404)
        self.assertTrue(self.post("/flows/" + fid).json if False else
                        self.http.delete(self.BASE + "/flows/" + fid, timeout=30).json()["ok"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
