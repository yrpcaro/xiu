#!/usr/bin/env python3
"""
Test xiu resizer CLI command.
"""
import subprocess
import unittest


class TestXiuResizerCLI(unittest.TestCase):
    def test_resizer_help(self):
        res = subprocess.run(["xiu", "resizer", "--help"], capture_output=True, text=True)
        self.assertEqual(res.returncode, 0)
        self.assertIn("resizer", res.stdout.lower())

    def test_resizer_empty_args(self):
        res = subprocess.run(["xiu", "resizer"], capture_output=True, text=True)
        self.assertEqual(res.returncode, 0)
        self.assertIn("use --daemon to start", res.stdout)


if __name__ == "__main__":
    unittest.main()
