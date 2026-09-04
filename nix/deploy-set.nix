# The deploy set as a plain function, so the home-manager module and the
# test harness share one definition. Mirrors DEPLOY_SET in
# installer/deploy.py — keep the two in step when the deploy set changes
# (the test below asserts the mapping itself, `xiu check` asserts the rest).
#
# repo: the xiu checkout (a path value).
# Result: a list of { dest, src } for every file worth linking, dest being
# the path under ~/.config and src the absolute file in the checkout.
{ repo }:

let
  # The same entries deploy.py ships: most land as whole dirs, the KDE
  # globals and the session target are single files that sit at a different
  # path than their source, the portal config targets xdg-desktop-portal's
  # dir, and the browser integration parks under xiu's own config dir.
  deploySet = [
    { name = "hypr"; src = "hypr"; dest = "hypr"; }
    { name = "quickshell"; src = "quickshell"; dest = "quickshell"; }
    { name = "ghostty"; src = "ghostty"; dest = "ghostty"; }
    { name = "foot"; src = "foot"; dest = "foot"; }
    { name = "fish"; src = "fish"; dest = "fish"; }
    { name = "fastfetch"; src = "fastfetch"; dest = "fastfetch"; }
    { name = "btop"; src = "btop"; dest = "btop"; }
    { name = "cava"; src = "cava"; dest = "cava"; }
    { name = "micro"; src = "micro"; dest = "micro"; }
    { name = "htop"; src = "htop"; dest = "htop"; }
    { name = "nvtop"; src = "nvtop"; dest = "nvtop"; }
    { name = "nvim"; src = "nvim"; dest = "nvim"; }
    { name = "helix"; src = "helix"; dest = "helix"; }
    { name = "bottom"; src = "bottom"; dest = "bottom"; }
    { name = "yazi"; src = "yazi"; dest = "yazi"; }
    { name = "spicetify"; src = "spicetify"; dest = "spicetify"; }
    { name = "portals"; src = "portals"; dest = "xdg-desktop-portal"; }
    { name = "uwsm"; src = "uwsm"; dest = "uwsm"; }
    { name = "browser"; src = "browser-integration"; dest = "xiu/browser-integration"; }
    { name = "kdeglobals"; src = "kde/kdeglobals"; dest = "kdeglobals"; file = true; }
    { name = "session"; src = "systemd/user/hyprland-session.target"; dest = "systemd/user/hyprland-session.target"; file = true; }
  ];

  # Files the rice treats as user-owned (deploy.py's PRESERVED plus the
  # Settings-written idle config): never linked, so HM activation can never
  # fight the shell's own Settings surface. Users who want them managed can
  # copy them once from the checkout.
  neverLink = [
    "hypr/modules/decoration.lua"
    "hypr/modules/binds.lua"
    "hypr/modules/monitors.lua"
    "hypr/modules/input.lua"
    "hypr/modules/env.lua"
    "hypr/modules/autostart.lua"
    "hypr/modules/animations.lua"
    "hypr/modules/stash-apps.lua"
    "hypr/modules/spaces.lua"
    "hypr/hypridle.conf"
    "fish/config.fish"
  ];

  # Dev cruft that never deploys (the same ignores as deploy.py's _copy).
  cruft = name:
    builtins.match "test_.*" name != null
    || name == "__pycache__"
    || builtins.match ".*\\.pyc" name != null;

  # Every file under dir, as dir-relative strings.
  walk = dir:
    let entries = builtins.readDir dir; in
    builtins.concatLists (builtins.attrValues (
      builtins.mapAttrs (name: type:
        if type == "directory"
        then (if cruft name then [ ] else map (f: "${name}/${f}") (walk (dir + "/${name}")))
        else (if cruft name then [ ] else [ name ])
      ) entries
    ));

  entryFiles = entry:
    if entry.file or false
    then [ { dest = entry.dest; src = "${repo}/configs/${entry.src}"; } ]
    else map (f: { dest = "${entry.dest}/${f}"; src = "${repo}/configs/${entry.src}/${f}"; })
      (walk "${repo}/configs/${entry.src}");

  allFiles = builtins.concatLists (map entryFiles deploySet);
in
builtins.filter (f: !builtins.elem f.dest neverLink) allFiles
