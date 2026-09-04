--[[
    Xiu app and tuning defaults. Any key can be overridden by
    ~/.config/xiu/vars.lua, a plain Lua file returning a table:

        return {
            terminal = "ghostty",
            fileManager = "yazi",
        }

    The override file lives outside the deployed config tree so updates never
    touch it; the installer seeds a commented template on first install.
    Binds stay literal in binds.lua (not here) so the pill's keybinds surface
    can keep parsing and editing them.
]]
local config_dir = os.getenv("XDG_CONFIG_HOME") or (os.getenv("HOME") .. "/.config")

local vars = {
    terminal    = "foot",
    browser     = "brave",
    editor      = "foot -e helix",
    fileManager = "dolphin",
    telegram    = "telegram-desktop",
    musicPlayer = "spotify",
    volumeStep  = 5,
    volumeMax   = 100,
    sleepCmd    = "systemctl suspend",

    -- Touchpad gestures (modules/gestures.lua): fingers for the workspace
    -- swipe and for the vertical special-workspace pair.
    gestureWorkspaceFingers = 3,
    gestureFingers          = 4,
}

local ok, user = pcall(dofile, config_dir .. "/xiu/vars.lua")
if ok and type(user) == "table" then
    for key, value in pairs(user) do
        vars[key] = value
    end
end

return vars
