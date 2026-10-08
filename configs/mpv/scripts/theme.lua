-- Xiu MPV theme reload script: monitors theme changes and applies them live
local opt = require 'mp.options'
local utils = require 'mp.utils'
local msg = require 'mp.msg'

local function get_config_dir()
    return mp.find_config_file("mpv.conf") and mp.command_native({"expand-path", "~~/"}) or nil
end

local theme_path = nil
local last_mtime = 0

local function reload_theme()
    if not theme_path then
        local dir = get_config_dir()
        if dir then
            theme_path = utils.join_path(dir, "theme.conf")
        else
            return
        end
    end

    local info = utils.file_info(theme_path)
    if not info or info.mtime == last_mtime then
        return
    end
    last_mtime = info.mtime

    local f = io.open(theme_path, "r")
    if not f then return end

    for line in f:lines() do
        line = line:gsub("^%s+", ""):gsub("%s+$", "")
        if not line:match("^#") and line:find("=") then
            local k, v = line:match("([^=]+)=(.*)")
            if k and v then
                k = k:gsub("^%s+", ""):gsub("%s+$", "")
                v = v:gsub("^[\"']", ""):gsub("[\"']$", "")
                pcall(function()
                    mp.set_property(k, v)
                end)
            end
        end
    end
    f:close()

    -- Also check uosc.conf
    local dir = get_config_dir()
    if dir then
        local uosc_conf = utils.join_path(dir, "script-opts/uosc.conf")
        local uf = io.open(uosc_conf, "r")
        if uf then
            for line in uf:lines() do
                if line:match("^color=") then
                    local color_val = line:sub(7)
                    pcall(function()
                        mp.commandv("change-list", "script-opts", "append", "uosc-color=" .. color_val)
                    end)
                    break
                end
            end
            uf:close()
        end
    end
end

-- Check on startup
mp.register_event("file-loaded", reload_theme)
reload_theme()

-- Periodically check every 2 seconds for theme updates while playing
mp.add_periodic_timer(2, reload_theme)
