#!/usr/bin/env python3
"""
Verification test suite for NixOS configuration compatibility and Quick Setup defaults.
Asserts that nix/deploy-set.nix, nix/test.nix, nix/home-manager.nix, nix/nixos.nix,
nix/configuration.nix, and flake.nix maintain full declarative parity across the repo.
"""
import os
import re
import sys
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "installer"))

import deploy


class TestNixCompatibility(unittest.TestCase):
    def setUp(self):
        self.nix_dir = REPO_ROOT / "nix"
        self.deploy_set_nix = self.nix_dir / "deploy-set.nix"
        self.test_nix = self.nix_dir / "test.nix"
        self.home_manager_nix = self.nix_dir / "home-manager.nix"
        self.nixos_nix = self.nix_dir / "nixos.nix"
        self.configuration_nix = self.nix_dir / "configuration.nix"
        self.flake_nix = REPO_ROOT / "flake.nix"

    def test_all_nix_files_exist(self):
        for path in (
            self.deploy_set_nix,
            self.test_nix,
            self.home_manager_nix,
            self.nixos_nix,
            self.configuration_nix,
            self.flake_nix,
        ):
            self.assertTrue(path.is_file(), f"Expected {path} to exist")

    def test_deploy_set_mirrors_installer_deploy_set(self):
        txt = self.deploy_set_nix.read_text()
        for name, src, dest in deploy.DEPLOY_SET:
            self.assertIn(
                f'name = "{name}";',
                txt,
                f"deploy-set.nix missing deploySet entry for name '{name}'",
            )
            self.assertIn(
                f'dest = "{dest}";',
                txt,
                f"deploy-set.nix missing dest '{dest}' for name '{name}'",
            )

    def test_never_link_mirrors_preserved(self):
        txt = self.deploy_set_nix.read_text()
        for p in deploy.PRESERVED:
            self.assertIn(
                f'"{p}"',
                txt,
                f"deploy-set.nix neverLink missing preserved file '{p}'",
            )

    def test_deploy_set_paths_exist_in_repo(self):
        for name, src, dest in deploy.DEPLOY_SET:
            path = REPO_ROOT / "configs" / src
            self.assertTrue(
                path.exists(),
                f"Source path configs/{src} does not exist in repo",
            )

    def test_test_nix_expected_roots(self):
        txt = self.test_nix.read_text()
        m = re.search(r"expectedRoots\s*=\s*\[(.*?)\];", txt, re.DOTALL)
        self.assertIsNotNone(m, "expectedRoots not found in nix/test.nix")
        roots_in_test = set(m.group(1).replace('"', "").split())

        for name, src, dest in deploy.DEPLOY_SET:
            root = dest if dest == "kdeglobals" else dest.split("/")[0]
            self.assertIn(
                root,
                roots_in_test,
                f"Root '{root}' from DEPLOY_SET destination '{dest}' not in test.nix expectedRoots",
            )

    def test_quick_setup_defaults_in_home_manager(self):
        txt = self.home_manager_nix.read_text()
        self.assertIn('default = "foot";', txt)
        self.assertIn('default = "helix";', txt)
        self.assertIn('default = "dolphin";', txt)
        self.assertIn('default = "brave";', txt)
        self.assertIn('default = "mpv";', txt)
        self.assertIn('default = "us,ir";', txt)
        self.assertIn('default = "grp:alt_shift_toggle";', txt)
        self.assertIn('default = ",winkeys";', txt)
        self.assertIn("naturalScroll", txt)
        self.assertIn("tapToClick", txt)
        self.assertTrue(re.search(r"lockTimeout.*?default\s*=\s*300;", txt, re.DOTALL))
        self.assertTrue(re.search(r"dpmsTimeout.*?default\s*=\s*360;", txt, re.DOTALL))
        self.assertIn("spicetify", txt)
        self.assertIn("telegram", txt)
        self.assertIn("yaziSuite", txt)
        self.assertIn("Darkly", txt)

        # yazi full suite tools
        for tool in ("yazi", "ripdrag", "trash-cli", "p7zip", "ffmpegthumbnailer", "poppler_utils", "resvg", "chafa", "lazygit", "simple-mtpfs"):
            self.assertIn(tool, txt)

    def test_quick_setup_defaults_in_nixos_module(self):
        txt = self.nixos_nix.read_text()
        self.assertIn('default = "foot";', txt)
        self.assertIn('default = "helix";', txt)
        self.assertIn('default = "dolphin";', txt)
        self.assertIn('default = "brave";', txt)
        self.assertIn('default = "mpv";', txt)
        self.assertIn('default = "none";', txt)  # direct TTY login (none)
        self.assertIn('default = "washi";', txt)  # SDDM with washi theme
        self.assertIn('default = "us,ir";', txt)
        self.assertIn('default = "grp:alt_shift_toggle";', txt)
        self.assertIn('default = ",winkeys";', txt)
        self.assertIn("naturalScrolling", txt)
        self.assertIn("tapping", txt)
        self.assertTrue(re.search(r"lockTimeout.*?default\s*=\s*300;", txt, re.DOTALL))
        self.assertTrue(re.search(r"dpmsTimeout.*?default\s*=\s*360;", txt, re.DOTALL))
        self.assertIn("programs.hyprland", txt)
        self.assertIn("withUWSM = true;", txt)
        self.assertIn("programs.fish", txt)
        self.assertIn("Darkly", txt)
        self.assertIn("spicetify-cli", txt)
        self.assertIn("telegram-desktop", txt)

    def test_quick_setup_defaults_in_configuration_template(self):
        txt = self.configuration_nix.read_text()
        self.assertIn('terminal = "foot";', txt)
        self.assertIn('editor = "helix";', txt)
        self.assertIn('fileManager = "dolphin";', txt)
        self.assertIn('browser = "brave";', txt)
        self.assertIn('mediaPlayer = "mpv";', txt)
        self.assertIn('greeter = "none";', txt)
        self.assertIn('sddmTheme = "washi";', txt)
        self.assertIn('layout = "us,ir";', txt)
        self.assertIn('options = "grp:alt_shift_toggle";', txt)
        self.assertIn('variant = ",winkeys";', txt)
        self.assertIn("naturalScroll = true;", txt)
        self.assertIn("tapToClick = true;", txt)
        self.assertIn("lockTimeout = 300;", txt)
        self.assertIn("dpmsTimeout = 360;", txt)
        self.assertIn("yaziSuite.enable = true;", txt)
        self.assertIn("spicetify.enable = true;", txt)
        self.assertIn("telegram.enable = true;", txt)
        self.assertIn("theming.enable = true;", txt)

    def test_flake_structure(self):
        txt = self.flake_nix.read_text()
        self.assertIn("nixosModules.xiu", txt)
        self.assertIn("homeManagerModules.xiu", txt)
        self.assertIn("nixosConfigurations.xiu", txt)
        self.assertIn("packages", txt)


if __name__ == "__main__":
    unittest.main()
