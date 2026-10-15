#!/usr/bin/env python3
import sys
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch, MagicMock

# Add installer dir to path
sys.path.insert(0, str(Path(__file__).parent))
import deploy
from xiu_install import _wizard, _default_choices, apply_input_preferences, apply_idle_preferences


class TestWizard(unittest.TestCase):
    def test_quick_install_defaults(self):
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
                {"id": "spotify", "desc": "spotify launcher", "group": "full"},
                {"id": "dolphin", "desc": "dolphin", "group": "full"},
            ]
        }

        with patch("tui.select_one", return_value=0), \
             patch("tui.select_many", return_value=[0]), \
             patch("tui.confirm", return_value=True):
            choices = _wizard(args, info, manifest)

        self.assertEqual(choices["profile"], "quick")
        self.assertEqual(choices["terminal"], "foot")
        self.assertEqual(choices["file_manager"], "dolphin")
        self.assertEqual(choices["editor"], "helix")
        self.assertEqual(choices["browser"], "brave")
        self.assertEqual(choices["media_player"], "mpv")
        self.assertTrue(choices["fish"])
        self.assertTrue(choices["brave"])
        self.assertEqual(choices["legacy_swap"], "clean")
        self.assertFalse(choices["grub"])
        self.assertEqual(choices["sddm_theme"], "washi")
        self.assertFalse(choices["browser_theme"])
        self.assertTrue(choices["fresh_configs"])
        self.assertTrue(choices["chat_theme"])
        self.assertTrue(choices["spicetify"])
        self.assertEqual(choices["kb_layout"], "us,ir")
        self.assertTrue(choices["natural_scroll"])
        self.assertTrue(choices["tap_to_click"])
        self.assertEqual(choices["idle_timeout"], "5m")
        self.assertIn("spotify", choices["optional_ids"])
        self.assertIn("dolphin", choices["optional_ids"])
        self.assertIn("yazi", choices["optional_ids"])
        self.assertIn("micro", choices["optional_ids"])

    def test_apply_input_preferences(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            hypr_modules = Path(tmpdir) / "hypr" / "modules"
            hypr_modules.mkdir(parents=True)
            input_lua = hypr_modules / "input.lua"
            input_lua.write_text("""hl.config({
    input = {
        kb_layout          = "us",
        follow_mouse       = 1,
        touchpad = {
            natural_scroll = true,
        },
    },
})""")
            with patch.object(deploy, "CONFIG_ROOT", Path(tmpdir)):
                choices = {
                    "kb_layout": "us,ir",
                    "natural_scroll": False,
                    "tap_to_click": True,
                }
                ok, detail = apply_input_preferences(choices, dry=False)
                self.assertTrue(ok)
                content = input_lua.read_text()
                self.assertIn('kb_layout          = "us,ir"', content)
                self.assertIn('kb_variant         = ",winkeys"', content)
                self.assertIn("natural_scroll = false", content)
                self.assertIn("tap_to_click   = true", content)

                # Now switch to us and tap_to_click false
                choices2 = {
                    "kb_layout": "us",
                    "natural_scroll": True,
                    "tap_to_click": False,
                }
                ok, detail = apply_input_preferences(choices2, dry=False)
                self.assertTrue(ok)
                content2 = input_lua.read_text()
                self.assertIn('kb_layout          = "us"', content2)
                self.assertNotIn("kb_variant", content2)
                self.assertIn("natural_scroll = true", content2)
                self.assertIn("tap_to_click   = false", content2)

    def test_apply_idle_preferences(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            hypr_dir = Path(tmpdir) / "hypr"
            hypr_dir.mkdir(parents=True)
            with patch.object(deploy, "CONFIG_ROOT", Path(tmpdir)):
                choices = {"idle_timeout": "10m"}
                ok, detail = apply_idle_preferences(choices, dry=False)
                self.assertTrue(ok)
                idle_conf = (hypr_dir / "hypridle.conf").read_text()
                self.assertIn("timeout = 600", idle_conf)
                self.assertIn("timeout = 660", idle_conf)


if __name__ == "__main__":
    unittest.main()
