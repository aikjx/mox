import unittest
from architecture_gate import combine


class ReleaseDecisionTests(unittest.TestCase):
    def test_clean_normal_graph_cannot_hide_declared_graph_violation(self):
        report = combine({"normal": {"findings": []}, "declared": {"findings": [
            {"level": "P1", "description": "contract depends on implementation"}]}})
        self.assertFalse(report["passed"])
        self.assertEqual(report["counts"]["P1"], 1)
        self.assertEqual(report["findings"][0]["checker"], "declared")

    def test_warnings_remain_visible_without_failing_the_release(self):
        report = combine({"declared": {"findings": [{"level": "P2", "description": "migration debt"}]}})
        self.assertTrue(report["passed"])
        self.assertEqual(report["counts"]["P2"], 1)


if __name__ == "__main__":
    unittest.main()
