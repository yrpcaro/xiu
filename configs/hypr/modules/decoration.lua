local home = os.getenv("HOME")
local ok, wc = pcall(dofile, home .. "/.cache/xiu/hypr-colors.lua")
if not ok then
    ok, wc = pcall(dofile, home .. "/.cache/ricelin/hypr-colors.lua")
end
if not ok then wc = nil end

local function border(hex, fallback)
    if type(hex) ~= "string" then hex = fallback end
    return "rgb(" .. hex:gsub("#", "") .. ")"
end

-- Active windows carry a clean 1px hairline accent border (border_size = 1).
-- Inactive windows have a completely transparent border (rgba(0,0,0,0))
-- so focus reads immediately and inactive windows sit quietly against the wallpaper.
local active   = border(wc and wc.active, "#e0563b")
local inactive = "rgba(0,0,0,0)"
local locked_active   = border(wc and wc.locked_active, "#f0b85e")
local locked_inactive = "rgba(0,0,0,0)"

local group_active          = border(wc and wc.group_active, wc and wc.active or "#e0563b")
local group_inactive        = border(wc and wc.group_inactive, "#2e231b")
local group_locked_active   = border(wc and wc.group_locked_active, "#f0b85e")
local group_locked_inactive = border(wc and wc.group_locked_inactive, "#2e231b")
local group_text            = border(wc and wc.text_color, "#fff6f0")

--[[
    Splash rendering SEGVs Hyprland (pango free in renderSplash) when a monitor
    gets reconfigured while the splash would draw, e.g. a display apply from the
    pill. Logo and splash off closes that crash surface.
]]
hl.config({
    misc = {
        disable_hyprland_logo    = true,
        disable_splash_rendering = true,
        vfr                      = true,
    },
    general = {
        gaps_in     = 6,
        gaps_out    = 12,
        border_size = 1,
        layout      = "dwindle",
        resize_on_border = true,
        ["col.active_border"]   = active,
        ["col.inactive_border"] = inactive,
    },
    group = {
        ["col.border_active"]          = active,
        ["col.border_inactive"]        = inactive,
        ["col.border_locked_active"]   = locked_active,
        ["col.border_locked_inactive"] = locked_inactive,
        groupbar = {
            enabled               = true,
            font_family           = "Inter",
            font_size             = 10,
            gradients             = false,
            height                = 14,
            priority              = 3,
            render_titles         = true,
            scrolling             = true,
            text_color            = group_text,
            ["col.active"]        = group_active,
            ["col.inactive"]      = group_inactive,
            ["col.locked_active"] = group_locked_active,
            ["col.locked_inactive"] = group_locked_inactive,
        },
    },
    decoration = {
        rounding         = 12,
        rounding_power   = 4,
        active_opacity   = 1.00,
        inactive_opacity = 1.00,
        shadow = {
            enabled      = true,
            range        = 10,
            render_power = 2,
            color        = 0xaa14110f,
        },
        blur = {
            enabled           = true,
            size              = 6,
            passes            = 2,
            vibrancy          = 0.17,
            noise             = 0.01,
            new_optimizations = true,
        },
    },
})
