--[[
    Touchpad gestures, caelestia-style. Three fingers swiped horizontally
    change the workspace; the finger counts and every swipe tuning knob live
    in vars (so ~/.config/xiu/vars.lua can rewire them without touching this
    file), and the swipe distances follow the caelestia feel.

    Gesture actions are Hyprland-native: "workspace" for the horizontal
    swipe, "special" with a workspace name for the vertical pair, and a Lua
    function for anything bespoke.
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
        workspace_swipe_fingers                  = vars.gestureWorkspaceFingers,
    },
})

-- The headline gesture: swipe between workspaces.
hl.gesture({
    fingers   = vars.gestureWorkspaceFingers,
    direction = "horizontal",
    action    = "workspace",
})

-- One more finger, vertically: the stash special workspace, up to open and
-- down to dismiss, the same pair caelestia uses for its scratchpad.
hl.gesture({
    fingers        = vars.gestureFingers,
    direction      = "up",
    action         = "special",
    workspace_name = "stash",
})

hl.gesture({
    fingers        = vars.gestureFingers,
    direction      = "down",
    action         = "special",
    workspace_name = "stash",
})
