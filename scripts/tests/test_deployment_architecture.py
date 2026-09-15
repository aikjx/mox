#!/usr/bin/env python3
"""MOX 默认部署与端口分配的最小架构契约。"""

import importlib.util
import json
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[2]
GATEWAY_PORT = 3080
OPTIONAL_DOMAIN_PORTS = {
    "mox-kg-server": 3411,
    "mox-cloud-server": 3412,
    "mox-iam-server": 3413,
    "mox-kb-server": 3414,
}


def load_port_verifier():
    path = REPO / "scripts" / "verify-ports.py"
    spec = importlib.util.spec_from_file_location("verify_ports", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class DeploymentArchitectureTest(unittest.TestCase):
    def test_registered_mox_ports_use_normalized_ranges(self):
        verifier = load_port_verifier()
        self.assertEqual(verifier.check_port_ranges(), [])
        self.assertEqual(verifier.CANONICAL[GATEWAY_PORT][0], "RUNTIME")
        for port in OPTIONAL_DOMAIN_PORTS.values():
            self.assertEqual(verifier.CANONICAL[port][0], "ANCILLARY")
            self.assertLessEqual(3000, port)
            self.assertLessEqual(port, 3999)
        self.assertEqual(verifier.CANONICAL[8080][0], "THIRD")
        for retired in (8101, 8102, 8103, 8104):
            self.assertEqual(verifier.CANONICAL[retired][0], "DEPRECATED")

    def test_default_runtime_has_one_gateway_process(self):
        config = json.loads(
            (REPO / "platform_config.json").read_text(encoding="utf-8-sig")
        )
        self.assertEqual(config["services"]["api"]["port"], GATEWAY_PORT)
        self.assertNotIn("mox-kb-server", config["services"])

        start = (REPO / "scripts" / "start-mox-enterprise.ps1").read_text(
            encoding="utf-8-sig"
        )
        self.assertIn(f'-Port {GATEWAY_PORT}', start)
        self.assertNotIn('Start-One -Name "mox-kb-server"', start)

    def test_kubernetes_exposes_only_the_gateway(self):
        manifest = (REPO / "deploy" / "k8s" / "base" / "mox-platform.yaml").read_text(
            encoding="utf-8-sig"
        )
        self.assertIn("name: mox-gateway", manifest)
        self.assertIn(f"containerPort: {GATEWAY_PORT}", manifest)
        self.assertIn("path: /health", manifest)
        self.assertNotIn("name: mox-kg-server", manifest)
        self.assertNotIn("name: mox-cloud-server", manifest)
        self.assertNotIn("name: mox-iam-server", manifest)
        self.assertNotIn("name: mox-kb-server", manifest)

    def test_compose_build_contexts_exist(self):
        compose = (REPO / "docker-compose.yml").read_text(encoding="utf-8-sig")
        llm_context = REPO / "projects" / "llm-inference-svc"
        self.assertTrue(llm_context.is_dir())
        self.assertIn("context: ./projects/llm-inference-svc", compose)
        self.assertNotIn("context: ./platform/services/llm-inference-svc", compose)


if __name__ == "__main__":
    unittest.main()
