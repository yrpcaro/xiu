local home = os.getenv("HOME")
local ok, wc = pcall(dofile, home .. "/.cache/ricelin/hypr-colors.lua")
if not ok then wc = nil end

local function border(hex, fallback)
    if type(hex) ~= "string" then hex = fallback end
    return "rgb(" .. hex:gsub("#", "") .. ")"
end

-- Only the active window carries a border: the inactive color is fully
-- transparent (Hyprland's parser takes rgba(0,0,0,0) — a bare rgb() has no
-- alpha), so focus reads at a glance from the single hairline accent and
-- inactive windows sit quietly against the wallpaper.
local active   = border(wc and wc.active, "#e0563b")
local inactive = "rgba(0,0,0,0)"

--[[
    Splash rendering SEGVs Hyprland (pango free in renderSplash) when a monitor
    gets reconfigured while the splash would draw, e.g. a display apply from the
    pill. Logo and splash off closes that crash surface.
]]
hl.config({
    misc = {
        disable_hyprland_logo    = true,
        disable_splash_rendering = true,
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
    decoration = {
        rounding         = 12,
        rounding_power   = 4,
        active_opacity   = 1.00,
        inactive_opacity = 1.00,
        shadow = {
            enabled      = true,
            range        = 12,
            render_power = 3,
            color        = 0xaa14110f,
        },
        blur = {
            enabled           = true,
            size              = 8,
            passes            = 3,
            vibrancy          = 0.17,
            noise             = 0.01,
            new_optimizations = true,
        },
    },
})
