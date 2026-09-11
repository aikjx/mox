import unittest
from unittest.mock import patch

import architecture_constraint_test as gate


class ArchitectureConstraintTests(unittest.TestCase):
    def test_current_layout_and_windows_paths(self):
        cases = {
            "platform/domains/alliance/core/scheduler/Cargo.toml": ("core", "alliance"),
            "platform/domains/alliance/proto/common/Cargo.toml": ("foundation", "alliance"),
            "platform/domains/base/mox-base-store-core/Cargo.toml": ("foundation", "platform"),
            "platform/domains/cloud/sdk/client/Cargo.toml": ("sdk", "cloud"),
            "platform/gateway/runtime/Cargo.toml": ("application", "platform"),
        }
        for path, expected in cases.items():
            with self.subTest(path=path):
                self.assertEqual(gate.classify_manifest(path), expected)
                self.assertEqual(gate.classify_manifest("D:\\repo\\" + path.replace("/", "\\")), expected)

    def test_core_cannot_depend_on_service(self):
        with patch.dict(gate.CRATE_LAYERS, {"core": "core", "svc": "service"}, clear=True):
            violations = gate.detect_layer_violations({"core": ["svc"], "svc": []})
        self.assertEqual([v.level for v in violations], ["P1"])

    def test_contracts_and_sdks_are_not_internal_cross_domain_edges(self):
        layers = {"source": "service", "contract": "foundation", "client": "sdk", "internal": "core"}
        domains = {"source": "ai", "contract": "kg", "client": "kg", "internal": "kg"}
        with patch.dict(gate.CRATE_LAYERS, layers, clear=True), patch.dict(gate.CRATE_DOMAINS, domains, clear=True):
            violations, count = gate.detect_cross_domain({"source": ["contract", "client", "internal"]})
        self.assertEqual(count, 1)
        self.assertIn("internal", violations[0].description)

    def test_cycles_remain_blocking(self):
        self.assertTrue(gate.detect_cycles({"a": ["b"], "b": ["a"]}))
        self.assertEqual(gate.detect_cycles({"a": ["b"], "b": []}), [])


if __name__ == "__main__":
    unittest.main()
