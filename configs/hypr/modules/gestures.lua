--[[
    Touchpad gestures, caelestia-style. The finger counts and every swipe
    tuning knob live in vars (so ~/.config/xiu/vars.lua can rewire them
    without touching this file), and the swipe distances follow the
    caelestia feel.

    The map, verified against Hyprland 0.56.2's gesture API
    (LuaBindingsConfigRules.cpp — actions are exactly: workspace, resize,
    move, special, close, float, fullscreen, cursor_zoom, scroll_move, or
    a Lua start/update/end callback receiving { delta = {x, y} }):

      3 fingers horizontal  workspaces (native CUnifiedWorkspaceSwipe —
                            the smoothest path; the tuning knobs above are
                            its knobs)
      3 fingers up / down    the stash special workspace (native)
      4 fingers horizontal   move the focused window to the previous/next
                            workspace (dispatcher gesture — native dispatch
                            per completed swipe)
      4 fingers vertical     volume, ±5% per threshold crossing with
                            continuous repeat while the swipe continues
                            (Lua callback: steps, because non-native
                            gestures dispatch rather than scrub)

    The finger count lives on hl.gesture only —
    gestures:workspace_swipe_fingers was removed in Hyprland 0.56, where
    an N-finger swipe is whatever hl.gesture registered for N fingers.
]]
local vars = require("modules.vars")

hl.config({
    gestures = {
        workspace_swipe_distance                 = 700,
        workspace_swipe_cancel_ratio             = 0.15,
        workspace_swipe_min_speed_to_force       = 5,
        workspace_swipe_direction_lock           = true,
        workspace_swipe_direction_lock_threshold = 10,
        workspace_swipe_create_new               = true,
    },
})

-- 3 fingers horizontal: swipe between workspaces.
hl.gesture({
    fingers   = vars.gestureWorkspaceFingers,
    direction = "horizontal",
    action    = "workspace",
})

-- 3 fingers vertical: the stash special workspace, up to open and down to
-- dismiss, the same pair caelestia uses for its scratchpad.
hl.gesture({
    fingers        = vars.gestureWorkspaceFingers,
    direction      = "up",
    action         = "special",
    workspace_name = "stash",
})

hl.gesture({
    fingers        = vars.gestureWorkspaceFingers,
    direction      = "down",
    action         = "special",
    workspace_name = "stash",
})

-- 4 fingers horizontal: carry the focused window to the neighbouring
-- workspace, left or right. One dispatcher gesture per direction, native
-- dispatch — the swipe completes and the move lands with it.
hl.gesture({
    fingers   = vars.gestureFingers,
    direction = "left",
    action    = "move",
})

hl.gesture({
    fingers   = vars.gestureFingers,
    direction = "right",
    action    = "move",
})

-- 4 fingers vertical: volume. A Lua-callback gesture accumulates the
-- swipe's y delta and steps the volume ±vars.volumeStep percent every
-- vars.gestureVolumeStep pixels of travel, repeating as the swipe
-- continues — so a long deliberate swipe rides the volume down in one
-- motion, and a short one taps a single step. The pole is chosen at the
-- first update so mid-swipe direction changes never fight; volume
-- commands match the keybinds (wpctl on the default sink, capped at
-- vars.volumeMax so a flung swipe cannot pin the speaker).
local vol_accum = 0
local vol_pole = 0

local function volume_step(sign)
    local flag = sign > 0 and "+" or "-"
    os.execute(string.format(
        "wpctl set-volume -l %.2f @DEFAULT_AUDIO_SINK@ %d%%%s >/dev/null 2>&1 &",
        vars.volumeMax / 100.0, vars.volumeStep, flag))
end

hl.gesture({
    fingers   = vars.gestureFingers,
    direction = "vertical",
    start     = function(_e)
        vol_accum = 0
        vol_pole = 0
    end,
    update    = function(e)
        if not e or not e.delta then
            return
        end
        local dy = e.delta.y or 0
        if vol_pole == 0 and dy ~= 0 then
            -- up is negative y on Wayland; the pole remembers which way
            -- this swipe is going so the accumulator only counts its own
            -- direction and a wobble never double-steps.
            vol_pole = dy > 0 and 1 or -1
        end
        -- only travel along the pole's axis accumulates
        local along = dy * vol_pole
        if along <= 0 then
            return
        end
        vol_accum = vol_accum + along
        local threshold = vars.gestureVolumeStep or 60
        while vol_accum >= threshold do
            -- swipe up raises volume, the natural deck-fader direction
            volume_step(vol_pole < 0 and 1 or -1)
            vol_accum = vol_accum - threshold
        end
    end,
    ["end"]   = function(_e)
        vol_accum = 0
        vol_pole = 0
    end,
})
