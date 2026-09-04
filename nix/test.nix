# Store-free harness for the deploy-set function: `nix --store dummy:// eval
# --impure --file nix/test.nix` walks the real checkout and asserts the
# mapping, so the nix side can never silently drift from the installer's
# DEPLOY_SET (the dest roots are asserted entry for entry).
let
  repo = toString ./..;
  files = import ./deploy-set.nix { inherit repo; };
  dests = map (f: f.dest) files;
  has = d: builtins.elem d dests;

  # The same roots as installer/deploy.py DEPLOY_SET maps to.
  expectedRoots = [
    "hypr" "quickshell" "ghostty" "foot" "fish" "fastfetch"
    "btop" "cava" "micro" "htop" "nvtop" "nvim" "helix"
    "bottom" "yazi" "spicetify"
    "xdg-desktop-portal" "uwsm" "xiu"
    "systemd" "kdeglobals"
  ];

  underKnownRoot = d:
    d == "kdeglobals"
    || builtins.elem (builtins.head (builtins.split "/" d)) expectedRoots;

  checks = [
    { ok = builtins.length files > 150; what = "the set walks a real tree (got ${toString (builtins.length files)} files)"; }
    { ok = has "quickshell/pill/Pill.qml" && has "hypr/hyprland.lua"; what = "dir entries land under their dest"; }
    { ok = has "kdeglobals" && has "systemd/user/hyprland-session.target"; what = "single-file entries land at their dest"; }
    { ok = has "xiu/browser-integration/manifest.json"; what = "browser integration parks under xiu's own dir"; }
    { ok = builtins.length (builtins.filter (d: builtins.match "xdg-desktop-portal/.*" d != null) dests) > 0; what = "portal config targets xdg-desktop-portal's dir"; }
    { ok = !(has "hypr/modules/binds.lua") && !(has "hypr/hypridle.conf") && !(has "fish/config.fish"); what = "user-owned files are never linked"; }
    { ok = !builtins.any (d: builtins.match ".*__pycache__.*" d != null || builtins.match ".*/test_.*\\.py" d != null) dests; what = "dev cruft never links"; }
    { ok = builtins.all (f: builtins.pathExists f.src) files; what = "every src exists in the checkout"; }
    { ok = builtins.all underKnownRoot dests; what = "no file lands outside the known dest roots"; }
  ];

  failed = map (c: c.what) (builtins.filter (c: !c.ok) checks);
in
if failed == [ ]
then "all ${toString (builtins.length checks)} deploy-set checks pass (${toString (builtins.length files)} files)"
else builtins.throw ("deploy-set checks failed: " + builtins.concatStringsSep "; " failed)
