"""Verify actual registry coverage and repeatable generation without losing the non-HTTP appendix."""
import re
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class ApiRegistryGenerationTest(unittest.TestCase):
    def test_complete_stable_registry_preserves_manual_appendix(self):
        document = ROOT / "docs/API-REGISTRY.md"
        before = document.read_text(encoding="utf-8")
        marker = "## \u9644\uff1a\u975e HTTP \u5165\u53e3"
        self.assertIn(marker, before)
        appendix = before[before.index(marker):].rstrip()
        command = [sys.executable, str(ROOT / "scripts/doc/gen-api-registry.py")]
        subprocess.run(command, cwd=ROOT, check=True)
        generated = document.read_text(encoding="utf-8")
        self.assertEqual(generated[generated.index(marker):].rstrip(), appendix)
        source = (ROOT / "platform/gateway/mox-platform-gateway-svc/src/actuator.rs").read_text(encoding="utf-8")
        declared = int(re.search(r"pub static ROUTES: \[ApiRoute; (\d+)\]", source).group(1))
        registered = re.findall(r'\br\(\s*"([^"]+)"\s*,\s*"(?:GET|POST|PUT|PATCH|DELETE|ANY|HEAD|OPTIONS)"', source)
        rendered = re.findall(r"^\| `([^`]+)` \|", generated, re.MULTILINE)
        self.assertEqual(len(rendered), declared)
        self.assertEqual(len(set(rendered)), declared)
        self.assertEqual(set(rendered), set(registered))
        subprocess.run(command, cwd=ROOT, check=True)
        self.assertEqual(document.read_text(encoding="utf-8"), generated)


if __name__ == "__main__":
    unittest.main()
