"""Exercise the actual registry and mutations of its invariants, without service doubles."""
import copy
import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("capabilities", Path(__file__).parents[1] / "registry/enterprise_capabilities.py")
catalog = importlib.util.module_from_spec(spec)
spec.loader.exec_module(catalog)


class RegistryTests(unittest.TestCase):
    def test_actual_registry_and_deterministic_generation(self):
        rows = catalog.validate(catalog.load())
        outputs = catalog.render(rows)
        self.assertEqual(outputs, catalog.render(copy.deepcopy(rows)))
        for row in rows:
            self.assertIn(f"## {row['id']} ", outputs["MODULES.md"])
            for requirement in row["requirements"]:
                self.assertIn(requirement["id"], outputs["MODULES.md"])

    def test_requirement_ids_do_not_silently_overwrite_each_other(self):
        data = catalog.load()
        requirements = data["modules"][0]["requirements"]
        requirements[1]["id"] = requirements[0]["id"]
        with self.assertRaisesRegex(ValueError, "explicit and unique"):
            catalog.validate(data)

    def test_duplicate_identifiers_are_rejected(self):
        data = catalog.load()
        data["modules"][1]["id"] = data["modules"][0]["id"]
        with self.assertRaisesRegex(ValueError, "unique"):
            catalog.validate(data)

    def test_unknown_and_cyclic_dependencies_are_rejected(self):
        for dependency in ["EM-99", "EM-02"]:
            data = catalog.load()
            data["modules"][0]["depends_on"] = [dependency]
            with self.assertRaises(ValueError):
                catalog.validate(data)

    def test_missing_or_escaping_paths_are_rejected(self):
        for path in ["../outside", "platform/nonexistent-enterprise-capability"]:
            data = catalog.load()
            data["modules"][0]["owners"] = [path]
            with self.assertRaisesRegex(ValueError, "path"):
                catalog.validate(data)

    def test_uncovered_code_domain_is_rejected(self):
        data = catalog.load()
        for row in data["modules"]:
            row["domains"] = [d for d in row["domains"] if d != "voice"] or ["platform"]
        with self.assertRaisesRegex(ValueError, "Uncovered"):
            catalog.validate(data)


if __name__ == "__main__":
    unittest.main()
