#!/usr/bin/env python3
import importlib.machinery
import importlib.util
from pathlib import Path
import time
import unittest

resizer_path = Path(__file__).parent / "xiu-resizer"
loader = importlib.machinery.SourceFileLoader("xiu_resizer", str(resizer_path))
spec = importlib.util.spec_from_loader("xiu_resizer", loader)
resizer = importlib.util.module_from_spec(spec)
loader.exec_module(resizer)


class Args:
    daemon = False
    pattern = ""
    match_type = ""
    width = ""
    height = ""
    actions = ""


class TestXiuResizer(unittest.TestCase):
    def setUp(self):
        self.cmd = resizer.Command(Args())

    def test_timeout_tracker_pruning(self):
        now = time.time()
        for i in range(60):
            self.cmd.timeout_tracker[f"old_{i}"] = now - 75
        self.assertEqual(len(self.cmd.timeout_tracker), 60)

        self.assertFalse(self.cmd._is_rate_limited("new_window"))
        self.assertNotIn("old_0", self.cmd.timeout_tracker)
        self.assertIn("new_window", self.cmd.timeout_tracker)
        self.assertEqual(len(self.cmd.timeout_tracker), 1)

    def test_match_window_rule(self):
        r1 = self.cmd._match_window_rule("(Bitwarden Password Manager) - Mozilla Firefox", "")
        self.assertIsNotNone(r1)
        self.assertEqual(r1.name, "(Bitwarden")

        r2 = self.cmd._match_window_rule("Picture-in-Picture", "")
        self.assertIsNotNone(r2)

        r3 = self.cmd._match_window_rule("Random Browser Tab", "")
        self.assertIsNone(r3)

    def test_handle_title_event_fast_no_match(self):
        event = "windowtitle>>1234abcd,Random Unmatched Window Title"
        self.cmd._handle_title_event(event)


if __name__ == "__main__":
    unittest.main()
