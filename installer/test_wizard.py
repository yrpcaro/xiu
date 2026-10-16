#!/usr/bin/env python3
import sys
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch, MagicMock

# Add installer dir to path
sys.path.insert(0, str(Path(__file__).parent))
import deploy
from xiu_install import (
    _wizard, _default_choices, _choice_ids, _summary_lines,
    YAZI_SUITE_IDS, apply_input_preferences, apply_idle_preferences,
)


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

    def test_custom_wizard_flow(self):
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

        # Select answers for Custom wizard flow:
        # profile (custom=1), aur (paru=0), fish (0), config strategy (0), legacy tools (0),
        # terminal (0), editor (0), file_manager (0), browser (0), media_player (0),
        # greeter (0), kb_layout (0), touchpad scroll (0), tap-to-click (0), idle (0),
        # spotify (0), browser live theme (0), chat theme (0)
        select_one_answers = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

        with patch("tui.select_one", side_effect=select_one_answers), \
             patch("tui.select_many", side_effect=[[], [0], [0]]), \
             patch("tui.confirm", return_value=True):
            choices = _wizard(args, info, manifest)

        self.assertEqual(choices["profile"], "custom")
        self.assertEqual(choices["aur_choice"], "paru")
        self.assertEqual(choices["terminal"], "foot")
        self.assertEqual(choices["editor"], "helix")
        self.assertEqual(choices["file_manager"], "dolphin")
        self.assertEqual(choices["browser"], "brave")
        self.assertEqual(choices["media_player"], "mpv")
        self.assertEqual(choices["greeter"], "none")
        self.assertEqual(choices["kb_layout"], "us,ir")
        self.assertTrue(choices["natural_scroll"])
        self.assertTrue(choices["tap_to_click"])
        self.assertEqual(choices["idle_timeout"], "5m")
        self.assertTrue(choices["spicetify"])
        self.assertFalse(choices["browser_theme"])
        self.assertTrue(choices["chat_theme"])

    def test_summary_lines(self):
        info = {
            "family": "arch",
            "existing": {},
        }
        choices = {
            "profile": "custom",
            "file_manager": "dolphin",
            "editor": "helix",
            "browser": "brave",
            "media_player": "mpv",
            "greeter": "sddm",
            "sddm_theme": "washi",
            "kb_layout": "us,ir",
            "idle_timeout": "5m",
            "browser_theme": False,
            "spicetify": True,
            "chat_theme": True,
            "legacy_swap": "clean",
            "fresh_configs": True,
            "grub": False,
            "brave": True,
            "fish": True,
        }
        plan = {
            "native": ["foot", "helix"],
            "aur": ["spicetify-cli"],
            "fallbacks": [],
            "skipped": [],
            "repos": [],
        }
        args = MagicMock()
        args.source = "/tmp/dummy"

        lines = _summary_lines(info, choices, plan, args, do_pkgs=True)
        text = "\n".join(lines)
        self.assertIn("Install 3 packages", text)
        self.assertIn("Default text editor: helix", text)
        self.assertIn("Default web browser: brave", text)
        self.assertIn("Default media player: mpv", text)
        self.assertIn("Keyboard layout: us,ir", text)
        self.assertIn("Idle lock screen timeout: 5m", text)
        self.assertIn("Spotify launcher themed", text)
        self.assertIn("Chat clients", text)

    def test_yazi_selection_pulls_full_suite(self):
        # 1. Verify _choice_ids pulls in the entire YAZI_SUITE_IDS when file_manager == "yazi"
        choices = {"file_manager": "yazi"}
        ids = _choice_ids(choices)
        for expected_id in YAZI_SUITE_IDS:
            self.assertIn(expected_id, ids)
        self.assertIn("termfilechooser", ids)
        self.assertIn("ripdrag", ids)
        self.assertIn("simple-mtpfs", ids)
        self.assertIn("trash-cli", ids)
        self.assertIn("7zip", ids)
        self.assertIn("ffmpegthumbnailer", ids)
        self.assertIn("poppler", ids)
        self.assertIn("resvg", ids)
        self.assertIn("chafa", ids)
        self.assertIn("lazygit", ids)

        # 2. Verify _wizard with yazi selected automatically includes the full suite
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
                {"id": "termfilechooser", "desc": "portal file chooser", "group": "full"},
                {"id": "ripdrag", "desc": "ripdrag", "group": "full"},
            ]
        }

        # Select yazi as default file manager (index 1)
        # 1: profile -> 1 ("custom")
        # 2: aur -> 0
        # 3: fish -> 0
        # 4: config strategy -> 0
        # 5: legacy tools -> 0
        # 6: terminal -> 0
        # 7: editor -> 0
        # 8: file_manager -> 1 ("yazi")
        # 9: browser -> 0
        # 10: media_player -> 0
        # 11: greeter -> 0
        # 12: kb_layout -> 0
        # 13: touchpad scroll -> 0
        # 14: tap-to-click -> 0
        # 15: idle -> 0
        # 16: spotify -> 0
        # 17: browser theme -> 0
        # 18: chat theme -> 0
        select_one_answers = [1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

        with patch("tui.select_one", side_effect=select_one_answers), \
             patch("tui.select_many", side_effect=[[], [], []]), \
             patch("tui.confirm", return_value=True):
            wizard_choices = _wizard(args, info, manifest)

        self.assertEqual(wizard_choices["file_manager"], "yazi")
        # Full YAZI_SUITE_IDS must be present in optional_ids even if deselecting in select_many
        for pkg_id in YAZI_SUITE_IDS:
            self.assertIn(pkg_id, wizard_choices["optional_ids"])


if __name__ == "__main__":
    unittest.main()

