<div align="center">

# xiu

**A Hyprland rice with a pill for a shell — Ricelin's body, caelestia's depth, reworked by hand.**

![xiu desktop](assets/hero.png)

</div>

xiu is my personal fork of [Ricelin](https://github.com/Gakuseei/Ricelin): the
same hand-written Quickshell pill, rebuilt around the things I wanted from it —
a deeper theming pipeline, caelestia-style keybinds that survive two keyboard
layouts, a lock screen worth looking at, and an installer that asks instead of
assuming. Everything below is what changed; the pill itself is still the star.

## The shell

Everything you see is hand-written Quickshell. One pill bar that morphs into
whatever surface I need.

![The pill surfaces](assets/shell.png)

The pill becomes media and now playing, a calendar, the wallpaper picker, the
clipboard history (clipvault behind it), an audio and brightness mixer, network
and bluetooth controls, a system monitor, a launcher, and the settings surface
where keybinds, idle behavior and the palette mode live without touching a
config file. There is also a lock screen and
[rishot](https://github.com/Gakuseei/rishot), the screenshot and annotation
tool, which lives in its own repo.

Hovering the pill lights it (a soft lift and an ember border tint) but never
resizes or remorphs it — it opens on a click, a keybind or an event, never on a
pointer crossing.

## What's different from Ricelin

**Compositor & keybinds**

- The whole bind map rewritten in the caelestia arrangement but on raw keycodes
  (`code:NNN`), so every bind hits the same physical key on both configured
  layouts — `us` and `ir(winkeys)`, toggled with `Super+Space`.
- Workspaces in groups of ten: digits pick the slot in the current group,
  `Ctrl+Super` + digit picks the group. Special workspaces for the stash
  (`Super+S`), the private space (`Super+Alt+P`) and the minimized set
  (`Ctrl+Super+M`), plus category toggles (music, communication, todo, sysmon).
- User overrides instead of edits: `xiu-vars.lua` and `xiu-user.lua` under
  `~/.config/xiu/` are loaded last and seeded on first run, so my app choices
  never overwrite yours on an update.

**The palette pipeline**

- matugen drives everything from the wallpaper: the pill, both terminals
  (foot and the optional ghostty), fastfetch, window borders, btop, htop,
  nvtop, cava, micro, helix, bottom, yazi, nvim (live — its colorscheme reads
  the palette file), spicetify, vesktop/vencord/equicord, Telegram (a
  generated `.attheme`), VSCode/VSCodium, Zed, GTK (adw-gtk3), Qt
  (qtengine + Darkly) and the window decorations.
- A scheme engine on top: seven presets, matugen's scheme variants, and
  smartScheme — a colourfulness heuristic that picks the variant per wallpaper.
  `xiu scheme` drives it from the terminal, the Look surface from the pill.
- Firefox and Zen get an original userChrome plus a live-theme WebExtension
  fed by a native messaging host, so the browser follows the wallpaper in real
  time; Brave/Chromium read their toolbar color from a managed policy
  (`xiu browser`).

**Tools**

- Terminal: **foot** (ghostty stays available as the optional fallback).
- Clipboard: **clipvault** behind the pill surface (cliphist as the fallback).
- The Rust set: eza, bat, fd, ripgrep, dust, zoxide, gitui, starship, direnv,
  bottom, yazi — with the fish aliases to match (`y` even closes into the
  directory you were in).
- **xiu**, the shell's own Rust CLI: `xiu shell/wallpaper/scheme/screenshot/
  record/clipboard/emoji/toggle/browser` plus `xiu check`, a drift and health
  report for the install.

**The lock screen**

Rearranged macOS-style over the original motion system: clock and date centered
in the upper third, a circular avatar (your `.face` or AccountsService icon,
with a glyph fallback) with the username beneath, and the password capsule
hanging under it — real frosted glass (a crop of the blurred desktop, blurred
once more), a border that tints on focus, ember-bead masking with a reveal eye,
and the failure shake intact. A status corner shows the keyboard layout,
battery, network and hold-to-confirm power actions.

**Elsewhere**

- Per-service IPC targets on the shell singletons (`qs -c pill ipc call …`).
- A window resizer / PiP daemon, ported from caelestia-cli as a standalone
  **GPL-3.0** script (see Credits).
- XDG portal tuning, uwsm-staged session env, optional trash-cli cleanup.
- An installer with a selftest and a real dry run, and a NixOS home-manager
  module (below).

## Stack

- WM: Hyprland, configured in Lua
- Shell UI: custom Quickshell
- Terminal: foot
- Shell: fish
- Font: JetBrains Mono Nerd
- Colors: matugen, palette pulled from the wallpaper

## Install

> [!WARNING]
> The installer is young. It hasn't had a clean-machine run beyond mine yet, so
> expect rough edges. Read `install.sh` first and keep backups. If something
> breaks, file a bug and say it's the installer.

One line, straight through the pipe:

```sh
curl -fsSL https://raw.githubusercontent.com/yrpcaro/xiu/main/install.sh | bash
```

`install.sh` is a thin bootstrap: it detects your distro (Arch, Debian, Fedora
or openSUSE), makes sure git and python3 are there, clones the rice, then hands
off to the guided Python installer. The wizard asks the questions that shape
the install:

- **profile** — Quick (core), Full (plus the daily apps) or Custom
- **file manager** — dolphin, yazi, thunar, or keep yours
- **login screen** — TTY (default), the torii SDDM theme, or greetd + tuigreet
- **browser live theme** — register the native host and drop the userChrome
  into your Firefox/Zen profiles
- **legacy tools** — keep ghostty + cliphist as fallbacks, or remove them once
  foot + clipvault are in
- **your configs** — carry them across (Settings, keybinds, fish; three-way
  merged on updates) or start from the repo defaults

Skip the wizard with flags, passed straight through the pipe:

```sh
curl -fsSL https://raw.githubusercontent.com/yrpcaro/xiu/main/install.sh | bash -s -- --quickstart
```

```
--quickstart  core defaults, no questions
--full        also install the daily apps
--sddm        also install the torii SDDM login theme
--no-deps     skip the package step, just deploy the configs
--dry-run     walk the whole flow and change nothing
--uninstall   remove the deployed configs and restore the backups
```

### NixOS

For home-manager, no build step — links instead of copies:

```nix
imports = [ /path/to/xiu/nix/home-manager.nix ];
programs.xiu = {
  enable = true;
  repo = /path/to/xiu;
};
```

The module links every deploy-set file into `~/.config` except the ones the
rice treats as user-owned (the Settings-written hypr modules, hypridle, fish),
so activation never fights the shell's own Settings surface; `exclude` covers
anything else you want to own in place. `nix/test.nix` is a store-free harness
for the mapping: `nix --store dummy:// eval --impure --file nix/test.nix`.

xiu is a Hyprland shell. On Niri, Sway or anything else only rishot (the
screenshot tool) makes sense; grab it from
[rishot](https://github.com/Gakuseei/rishot)'s own installer.

## Keybinds

Every bind is on a raw keycode, so the table is layout-proof — the key is named
for the `us` layout and lands on the same physical key in `ir(winkeys)`.
`Super` is the Windows key; tap it alone for the launcher.

| Key | Action |
|---|---|
| `Super` (tap) | app launcher |
| `Super` + `Space` | switch keyboard layout (us ⇄ ir) |
| **Session** | |
| `Ctrl` + `Alt` + `Del` | session menu (power) |
| `Super` + `N` | notifications |
| `Ctrl` + `Alt` + `C` | clear notifications |
| `Super` + `K` | peek the pill |
| `Super` + `L` | lock |
| `Super` + `Alt` + `L` | restart the shell and lock |
| `Super` + `Shift` + `L` | sleep |
| `Ctrl` + `Super` + `Shift` + `R` | stop the shells (no auto-restart) |
| `Ctrl` + `Super` + `Alt` + `R` | restart the shells |
| **Windows** | |
| `Super` + `Q` | close window |
| `Super` + `F` | fullscreen |
| `Super` + `Alt` + `F` | maximize |
| `Super` + `Alt` + `Space` | toggle floating |
| `Super` + `Z` / `Super` + `X` | move / resize window by keyboard |
| `Super` + drag | drag window |
| `Super` + right-drag | resize window |
| `Super` + `P` | pin window |
| `Ctrl` + `Super` + `\` | center window |
| `Ctrl` + `Super` + `Alt` + `\` | normalize window (55% × 70%, centered) |
| `Super` + `Alt` + `\` | picture-in-picture |
| `Super` + arrows | focus by direction |
| `Super` + `Shift` + arrows | move window by direction |
| `Super` + `-` / `Super` + `=` | narrower / wider (repeats) |
| `Super` + `Shift` + `-` / `=` | shorter / taller (repeats) |
| `Alt` + `Tab` / `Shift` + `Alt` + `Tab` | cycle windows |
| `Ctrl` + `Alt` + `Tab` | cycle window group |
| `Super` + `,` | toggle window group |
| `Super` + `Shift` + `,` | lock into window group |
| `Super` + `U` | ungroup window |
| **Workspaces** | |
| `Super` + `1`–`0` | workspace 1–10 (slot in the current group) |
| `Super` + `Alt` + `1`–`0` | move window to slot |
| `Ctrl` + `Super` + `1`–`0` | workspace group 1–10 (keeps the slot) |
| `Ctrl` + `Super` + `Alt` + `1`–`0` | move window to group |
| `Super` + wheel / `PgUp` / `PgDn` | previous / next workspace |
| `Ctrl` + `Super` + wheel | previous / next workspace group |
| `Super` + `Alt` + wheel / `PgUp` / `PgDn` | move window across workspaces |
| `Super` + `S` | stash workspace |
| `Super` + `Shift` + `S` | send window to the stash |
| `Ctrl` + `Super` + `Shift` + `Up` / `Down` | send window to the stash / take it back |
| `Super` + `Alt` + `P` | private workspace |
| `Super` + `Shift` + `P` | send window to the private space |
| `Super` + `Alt` + `M` | minimize toggle |
| `Ctrl` + `Super` + `M` | minimized stash |
| `Ctrl` + `Shift` + `Esc` | system monitor workspace |
| `Super` + `M` / `D` / `R` | music / communication / todo workspace |
| **Apps & tools** | |
| `Super` + `T` | terminal (foot) |
| `Super` + `W` | browser |
| `Super` + `C` | editor |
| `Super` + `E` | file manager |
| `Ctrl` + `Alt` + `V` | mixer |
| `Super` + `V` | clipboard history |
| `Ctrl` + `Shift` + `Alt` + `V` | paste latest clipboard (works while locked) |
| `Super` + `.` | emoji picker |
| `Super` + `B` | shuffle wallpaper and retheme |
| `Super` + `Shift` + `B` | wallpaper picker |
| `Super` + `Shift` + `C` | color picker |
| `Super` + `G` | game mode |
| `Ctrl` + `Alt` + `R` | screen record |
| `Print` / `Shift` + `Print` | rishot (see its repo) |
| **Media (work while locked)** | |
| `Ctrl` + `Super` + `Space` | play / pause |
| `Ctrl` + `Super` + `=` / `-` | next / previous track |
| `Super` + `Shift` + `M` | mute audio |
| hardware keys | volume, brightness, media |

## Syncing with upstream

xiu tracks [Gakuseei/Ricelin](https://github.com/Gakuseei/Ricelin) through the
`upstream` remote. After upstream moves:

```sh
git checkout xiu
git fetch upstream
git merge upstream/main
# resolve whatever conflicts come up — most of the pill's QML is untouched
# upstream, so the usual conflict points are the installer and the scripts
git push origin xiu
git checkout main
git merge xiu --no-edit
git push origin main
```

`xiu check` (the CLI's drift command) reports how far your checkout is from
both remotes, and the pill's own Settings > Updates surface does the same
in-app.

## Credits

- [Ricelin](https://github.com/Gakuseei/Ricelin) by
  [Gakuseei](https://github.com/Gakuseei) — the base this fork grew from:
  the pill, the surfaces, the Lua hypr setup and the installer skeleton.
  MIT licensed.
- [caelestia-dots](https://github.com/caelestia-dots) — the inspiration tree:
  the keybind arrangement is re-expressed from its (unlicensed) dots repo as
  functional data in original Lua; the window resizer / PiP daemon is ported
  from its GPL-3.0 CLI and ships as the standalone GPL-3.0
  `configs/hypr/scripts/xiu-resizer`; theme template *formats* (config
  syntaxes) were referenced while every generator in `wallcolors.py` is
  original code.
- [rishot](https://github.com/Gakuseei/rishot) — the screenshot tool, its own
  project.
- The lock screen, the SDDM background and the wallpapers are not mine. See
  [CREDITS](configs/sddm/themes/torii/CREDITS.md).
