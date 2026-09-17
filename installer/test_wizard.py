#!/usr/bin/env python3
import sys
from pathlib import Path
import unittest
from unittest.mock import patch, MagicMock

# Add installer dir to path
sys.path.insert(0, str(Path(__file__).parent))
from xiu_install import _wizard, _default_choices


class TestWizard(unittest.TestCase):
    def test_wizard_keys(self):
        args = MagicMock()
        args.full = False
        args.sddm = False
        args.brave = False

        info = {
            "family": "arch",
            "aur_helper": "yay",
            "bootloader": "systemd-boot",
            "root_cmd": "sudo",
            "init": "systemd",
        }

        manifest = {
            "packages": [
                {"id": "yazi", "desc": "yazi file manager", "group": "full"},
                {"id": "brave", "desc": "brave browser", "group": "full"},
            ]
        }

        with patch("tui.select_one", return_value=0), \
             patch("tui.select_many", return_value=[0]), \
             patch("tui.confirm", return_value=True):
            choices = _wizard(args, info, manifest)

        self.assertIn("fish", choices)
        self.assertTrue(choices["fish"])
        self.assertIn("brave", choices)
        self.assertIn("file_manager", choices)
        self.assertIn("legacy_swap", choices)
        self.assertIn("grub", choices)
        self.assertIn("sddm_theme", choices)
        self.assertIn("browser_theme", choices)


if __name__ == "__main__":
    unittest.main()
