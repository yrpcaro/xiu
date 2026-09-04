--[[
    Xiu keybinds, in the caelestia arrangement but on raw keycodes (code:NNN,
    the xkb keycode = evdev + 8) so every bind hits the same physical key on
    the us and ir(winkeys) layouts. Combos stay literal here — no vars
    indirection — so the pill's keybinds surface can parse and rebind them;
    the trailing -- comments are the names it shows. App commands live in
    modules/vars.lua and category workspaces in modules/toggles.lua, both
    overridable from ~/.config/xiu/. rishot owns the Print binds in
    rishot.lua.
]]
local mod = "SUPER"
local vars = require("modules.vars")
local toggles = require("modules.toggles")

local locked = { locked = true }
local mouse = { mouse = true }
local release = { release = true }
local repeating = { repeating = true }
local locked_repeating = { locked = true, repeating = true }

-- Digits pick the slot inside the current group of ten workspaces;
-- CTRL+SUPER plus a digit picks the group, keeping the current slot
-- (group 1 is workspaces 1-10, group 2 is 11-20, and so on).
local function goto_slot(i)
    return function()
        local active = hl.get_active_workspace()
        local base = active and (math.floor((active.id - 1) / 10) * 10) or 0
        hl.dispatch(hl.dsp.focus({ workspace = base + i }))
    end
end

local function move_slot(i)
    return function()
        local active = hl.get_active_workspace()
        local base = active and (math.floor((active.id - 1) / 10) * 10) or 0
        hl.dispatch(hl.dsp.window.move({ workspace = base + i, follow = false }))
    end
end

local function goto_group(i)
    return function()
        local active = hl.get_active_workspace()
        local slot = active and (((active.id - 1) % 10) + 1) or i
        hl.dispatch(hl.dsp.focus({ workspace = (i - 1) * 10 + slot }))
    end
end

local function move_group(i)
    return function()
        local active = hl.get_active_workspace()
        local slot = active and (((active.id - 1) % 10) + 1) or i
        hl.dispatch(hl.dsp.window.move({ workspace = (i - 1) * 10 + slot, follow = false }))
    end
end

local function resize_step(dx, dy)
    return function()
        hl.dispatch(hl.dsp.window.resize({ x = dx, y = dy, relative = true }))
    end
end

-- 55%x70% of the monitor, centered: the one-size "make this window normal" reset.
local function normalize_window()
    return function()
        local mon = hl.get_active_monitor()
        if mon and mon.width and mon.height then
            hl.dispatch(hl.dsp.window.resize({ x = math.floor(mon.width * 0.55), y = math.floor(mon.height * 0.70) }))
            hl.dispatch(hl.dsp.window.center())
        end
    end
end

-- Launcher on Super+Space. The earlier tap-Super bind (release on the bare
-- modifier) misfired on every Super combo on some Hyprland builds — the
-- mod-only release shadowing is not dependable across versions — so the
-- launcher moved to an ordinary combo.
hl.bind(mod .. " + code:65", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/open-surface.sh launcher")) -- launcher

-- Keyboard layout: us <-> ir(winkeys), Alt+Shift like the classic DE toggle.
-- The pill's layout chip fires the same command and follows along on the
-- event; the release flag keeps it from firing the moment Shift goes down
-- inside another Alt+Shift combo.
hl.bind("ALT + code:50", hl.dsp.exec_cmd("hyprctl switchxkblayout current next"), release) -- switch keyboard layout

-- Session, notifications, lock
hl.bind("CTRL + ALT + code:119", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/open-surface.sh power")) -- session menu
hl.bind(mod .. " + code:57", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/open-surface.sh link")) -- notifications
hl.bind("CTRL + ALT + code:54", hl.dsp.exec_cmd("qs -c pill ipc call notifs clear")) -- clear notifications
hl.bind(mod .. " + code:45", hl.dsp.exec_cmd('qs -c pill ipc call pill peek ""')) -- peek the pill
hl.bind(mod .. " + code:46", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/lock.sh")) -- lock
hl.bind(mod .. " + ALT + code:46", hl.dsp.exec_cmd("sh -c 'qs -c pill kill; $HOME/.config/hypr/scripts/lock.sh'")) -- restart shell and lock
hl.bind(mod .. " + SHIFT + code:46", hl.dsp.exec_cmd(vars.sleepCmd)) -- sleep
hl.bind("CTRL + SUPER + SHIFT + code:27", hl.dsp.exec_cmd("pkill -f watchdog.sh; qs -c pill kill; qs -c lock kill"), release) -- stop shells (no auto-restart)
hl.bind("CTRL + SUPER + ALT + code:27", hl.dsp.exec_cmd("qs -c pill kill; qs -c lock kill"), release) -- restart shells (watchdogs respawn them)

-- Window actions
hl.bind(mod .. " + code:24", hl.dsp.window.close()) -- close window
hl.bind(mod .. " + code:41", hl.dsp.window.fullscreen()) -- fullscreen
hl.bind(mod .. " + ALT + code:41", hl.dsp.window.fullscreen({ mode = "maximized" })) -- maximize window
hl.bind(mod .. " + ALT + code:65", hl.dsp.window.float()) -- toggle floating
hl.bind(mod .. " + code:52", hl.dsp.window.drag()) -- move window (keyboard)
hl.bind(mod .. " + code:53", hl.dsp.window.resize()) -- resize window (keyboard)
hl.bind(mod .. " + mouse:272", hl.dsp.window.drag(), mouse) -- drag window
hl.bind(mod .. " + mouse:273", hl.dsp.window.resize(), mouse) -- resize window
hl.bind(mod .. " + code:33", hl.dsp.window.pin()) -- pin window
hl.bind("CTRL + SUPER + code:51", hl.dsp.window.center()) -- center window
hl.bind("CTRL + SUPER + ALT + code:51", normalize_window()) -- normalize window
hl.bind(mod .. " + ALT + code:51", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/xiu-resizer pip")) -- picture-in-picture

-- Focus and move by direction
hl.bind(mod .. " + code:113", hl.dsp.focus({ direction = "left" })) -- focus left
hl.bind(mod .. " + code:114", hl.dsp.focus({ direction = "right" })) -- focus right
hl.bind(mod .. " + code:111", hl.dsp.focus({ direction = "up" })) -- focus up
hl.bind(mod .. " + code:116", hl.dsp.focus({ direction = "down" })) -- focus down
hl.bind(mod .. " + SHIFT + code:113", hl.dsp.window.move({ direction = "left" })) -- move window left
hl.bind(mod .. " + SHIFT + code:114", hl.dsp.window.move({ direction = "right" })) -- move window right
hl.bind(mod .. " + SHIFT + code:111", hl.dsp.window.move({ direction = "up" })) -- move window up
hl.bind(mod .. " + SHIFT + code:116", hl.dsp.window.move({ direction = "down" })) -- move window down

-- Window resize by keys
hl.bind(mod .. " + code:20", resize_step(-10, 0), repeating) -- narrower
hl.bind(mod .. " + code:21", resize_step(10, 0), repeating) -- wider
hl.bind(mod .. " + SHIFT + code:20", resize_step(0, -10), repeating) -- shorter
hl.bind(mod .. " + SHIFT + code:21", resize_step(0, 10), repeating) -- taller

-- Alt-tab and window groups
hl.bind("ALT + code:23", hl.dsp.window.cycle_next(), repeating) -- cycle windows
hl.bind("SHIFT + ALT + code:23", hl.dsp.window.cycle_next({ next = false }), repeating) -- cycle windows back
hl.bind("CTRL + ALT + code:23", hl.dsp.group.next(), repeating) -- cycle window group
hl.bind("CTRL + SHIFT + ALT + code:23", hl.dsp.group.prev(), repeating) -- cycle window group back
hl.bind(mod .. " + code:59", hl.dsp.group.toggle()) -- toggle window group
hl.bind(mod .. " + SHIFT + code:59", hl.dsp.group.lock_active()) -- lock into window group
hl.bind(mod .. " + code:30", hl.dsp.window.move({ out_of_group = true })) -- ungroup window

-- Workspaces 1-10, the slot in the current group
hl.bind(mod .. " + code:10", goto_slot(1)) -- workspace 1
hl.bind(mod .. " + code:11", goto_slot(2)) -- workspace 2
hl.bind(mod .. " + code:12", goto_slot(3)) -- workspace 3
hl.bind(mod .. " + code:13", goto_slot(4)) -- workspace 4
hl.bind(mod .. " + code:14", goto_slot(5)) -- workspace 5
hl.bind(mod .. " + code:15", goto_slot(6)) -- workspace 6
hl.bind(mod .. " + code:16", goto_slot(7)) -- workspace 7
hl.bind(mod .. " + code:17", goto_slot(8)) -- workspace 8
hl.bind(mod .. " + code:18", goto_slot(9)) -- workspace 9
hl.bind(mod .. " + code:19", goto_slot(10)) -- workspace 10

-- Move the window to the slot in the current group
hl.bind("SUPER + ALT + code:10", move_slot(1)) -- move window to workspace 1
hl.bind("SUPER + ALT + code:11", move_slot(2)) -- move window to workspace 2
hl.bind("SUPER + ALT + code:12", move_slot(3)) -- move window to workspace 3
hl.bind("SUPER + ALT + code:13", move_slot(4)) -- move window to workspace 4
hl.bind("SUPER + ALT + code:14", move_slot(5)) -- move window to workspace 5
hl.bind("SUPER + ALT + code:15", move_slot(6)) -- move window to workspace 6
hl.bind("SUPER + ALT + code:16", move_slot(7)) -- move window to workspace 7
hl.bind("SUPER + ALT + code:17", move_slot(8)) -- move window to workspace 8
hl.bind("SUPER + ALT + code:18", move_slot(9)) -- move window to workspace 9
hl.bind("SUPER + ALT + code:19", move_slot(10)) -- move window to workspace 10

-- Workspace groups: the digit picks the group, keeping the current slot
hl.bind("CTRL + SUPER + code:10", goto_group(1)) -- workspace group 1
hl.bind("CTRL + SUPER + code:11", goto_group(2)) -- workspace group 2
hl.bind("CTRL + SUPER + code:12", goto_group(3)) -- workspace group 3
hl.bind("CTRL + SUPER + code:13", goto_group(4)) -- workspace group 4
hl.bind("CTRL + SUPER + code:14", goto_group(5)) -- workspace group 5
hl.bind("CTRL + SUPER + code:15", goto_group(6)) -- workspace group 6
hl.bind("CTRL + SUPER + code:16", goto_group(7)) -- workspace group 7
hl.bind("CTRL + SUPER + code:17", goto_group(8)) -- workspace group 8
hl.bind("CTRL + SUPER + code:18", goto_group(9)) -- workspace group 9
hl.bind("CTRL + SUPER + code:19", goto_group(10)) -- workspace group 10

-- Move the window to the group, keeping the current slot
hl.bind("CTRL + SUPER + ALT + code:10", move_group(1)) -- move window to group 1
hl.bind("CTRL + SUPER + ALT + code:11", move_group(2)) -- move window to group 2
hl.bind("CTRL + SUPER + ALT + code:12", move_group(3)) -- move window to group 3
hl.bind("CTRL + SUPER + ALT + code:13", move_group(4)) -- move window to group 4
hl.bind("CTRL + SUPER + ALT + code:14", move_group(5)) -- move window to group 5
hl.bind("CTRL + SUPER + ALT + code:15", move_group(6)) -- move window to group 6
hl.bind("CTRL + SUPER + ALT + code:16", move_group(7)) -- move window to group 7
hl.bind("CTRL + SUPER + ALT + code:17", move_group(8)) -- move window to group 8
hl.bind("CTRL + SUPER + ALT + code:18", move_group(9)) -- move window to group 9
hl.bind("CTRL + SUPER + ALT + code:19", move_group(10)) -- move window to group 10

-- Relative workspace movement
hl.bind(mod .. " + mouse_down", hl.dsp.focus({ workspace = "r+1" })) -- next workspace
hl.bind(mod .. " + mouse_up", hl.dsp.focus({ workspace = "r-1" })) -- previous workspace
hl.bind(mod .. " + code:117", hl.dsp.focus({ workspace = "r+1" }), repeating) -- next workspace
hl.bind(mod .. " + code:112", hl.dsp.focus({ workspace = "r-1" }), repeating) -- previous workspace
hl.bind("CTRL + SUPER + code:114", hl.dsp.focus({ workspace = "r+1" }), repeating) -- next workspace
hl.bind("CTRL + SUPER + code:113", hl.dsp.focus({ workspace = "r-1" }), repeating) -- previous workspace
hl.bind("CTRL + SUPER + mouse_down", hl.dsp.focus({ workspace = "r+10" })) -- next workspace group
hl.bind("CTRL + SUPER + mouse_up", hl.dsp.focus({ workspace = "r-10" })) -- previous workspace group
hl.bind("SUPER + ALT + mouse_down", hl.dsp.window.move({ workspace = "r+1" })) -- move window to next workspace
hl.bind("SUPER + ALT + mouse_up", hl.dsp.window.move({ workspace = "r-1" })) -- move window to previous workspace
hl.bind("SUPER + ALT + code:117", hl.dsp.window.move({ workspace = "r+1" }), repeating) -- move window to next workspace
hl.bind("SUPER + ALT + code:112", hl.dsp.window.move({ workspace = "r-1" }), repeating) -- move window to previous workspace
hl.bind("CTRL + SUPER + SHIFT + code:114", hl.dsp.window.move({ workspace = "r+1" }), repeating) -- move window to next workspace
hl.bind("CTRL + SUPER + SHIFT + code:113", hl.dsp.window.move({ workspace = "r-1" }), repeating) -- move window to previous workspace

-- Special workspaces
hl.bind(mod .. " + code:39", hl.dsp.workspace.toggle_special("stash")) -- stash workspace
hl.bind(mod .. " + SHIFT + code:39", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/special-toggle.sh stash")) -- send window to stash
hl.bind("CTRL + SUPER + SHIFT + code:111", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/special-toggle.sh stash")) -- send window to stash
hl.bind("CTRL + SUPER + SHIFT + code:116", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/special-toggle.sh stash")) -- take window back from stash
hl.bind(mod .. " + ALT + code:33", hl.dsp.workspace.toggle_special("private")) -- private workspace
hl.bind(mod .. " + SHIFT + code:33", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/special-toggle.sh private")) -- send window to private
hl.bind(mod .. " + ALT + code:58", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/minimize-toggle.sh")) -- minimize toggle
hl.bind("CTRL + SUPER + code:58", hl.dsp.workspace.toggle_special("minimized")) -- minimized stash

-- App-category workspaces (see modules/toggles.lua)
hl.bind("CTRL + SHIFT + code:9", toggles.toggle("sysmon")) -- system monitor workspace
hl.bind(mod .. " + code:58", toggles.toggle("music")) -- music workspace
hl.bind(mod .. " + code:40", toggles.toggle("communication")) -- communication workspace
hl.bind(mod .. " + code:27", toggles.toggle("todo")) -- todo workspace

-- Apps
hl.bind(mod .. " + code:36", hl.dsp.exec_cmd(vars.terminal)) -- terminal
hl.bind(mod .. " + code:25", hl.dsp.exec_cmd(vars.browser)) -- browser
hl.bind(mod .. " + code:54", hl.dsp.exec_cmd(vars.editor)) -- editor
hl.bind(mod .. " + code:26", hl.dsp.exec_cmd(vars.fileManager)) -- file manager
hl.bind("CTRL + ALT + code:55", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/open-surface.sh mixer")) -- mixer

-- Wallpaper, capture, record
hl.bind(mod .. " + code:56", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/wallpaper.sh")) -- random wallpaper
hl.bind(mod .. " + SHIFT + code:56", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/open-surface.sh wallpaper")) -- wallpaper picker
hl.bind(mod .. " + SHIFT + code:54", hl.dsp.exec_cmd("hyprpicker -a")) -- color picker
hl.bind(mod .. " + code:42", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/open-surface.sh gameMode")) -- game mode
hl.bind("CTRL + ALT + code:27", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/record.sh")) -- screen record

-- Clipboard
hl.bind(mod .. " + code:55", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/open-surface.sh clipboard")) -- clipboard history
hl.bind("CTRL + SHIFT + ALT + code:55", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/scripts/paste-latest.sh"), locked) -- paste latest clipboard
hl.bind(mod .. " + code:60", hl.dsp.exec_cmd("xiu emoji -p")) -- emoji picker

-- Media keys (Quickshell globals registered by the pill's Players singleton)
hl.bind("CTRL + SUPER + code:65", hl.dsp.global("quickshell:mediaToggle"), locked) -- play / pause
hl.bind("CTRL + SUPER + code:21", hl.dsp.global("quickshell:mediaNext"), locked) -- next track
hl.bind("CTRL + SUPER + code:20", hl.dsp.global("quickshell:mediaPrev"), locked) -- previous track

-- Hardware keys (layout-independent symbols)
hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%+"), locked_repeating) -- volume up
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"), locked_repeating) -- volume down
hl.bind("XF86AudioMute", hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"), locked) -- mute audio
hl.bind(mod .. " + SHIFT + code:58", hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"), locked) -- mute audio
hl.bind("XF86MonBrightnessUp", hl.dsp.exec_cmd("brightnessctl set 5%+"), locked_repeating) -- brightness up
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("brightnessctl set 5%-"), locked_repeating) -- brightness down
hl.bind("XF86AudioPlay", hl.dsp.global("quickshell:mediaToggle"), locked) -- play / pause
hl.bind("XF86AudioNext", hl.dsp.global("quickshell:mediaNext"), locked) -- next track
hl.bind("XF86AudioPrev", hl.dsp.global("quickshell:mediaPrev"), locked) -- previous track
