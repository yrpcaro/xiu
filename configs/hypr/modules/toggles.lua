--[[
    App-category special workspaces: sysmon, music, communication, todo. One
    key per category opens its special workspace, spawns every configured app
    that is not running (straight onto the workspace), herds strays back into
    place, and a second press dismisses it. Apps and match rules come from
    ~/.config/xiu/toggles.lua over the defaults below, same shape:

        return {
            music = {
                spotify = { match = { { class = "Spotify" } }, command = { "spotify" } },
            },
        }

    A match rule is a set of field -> substring checks (class, title,
    initialTitle, ...); a window matches when every field in one rule hits.
    `command` is spawned when no window matches; `move = false` leaves stray
    windows alone instead of herding them back.
]]
local config_dir = os.getenv("XDG_CONFIG_HOME") or (os.getenv("HOME") .. "/.config")

local defaults = {
    sysmon = {
        btop = { match = { { class = "btop", title = "btop" } },
                 command = { "foot", "-a", "btop", "-T", "btop", "fish", "-C", "exec", "btop" } },
    },
    music = {},
    communication = {},
    todo = {},
}

local function rules()
    local apps = {}
    for category, list in pairs(defaults) do apps[category] = list end
    local ok, user = pcall(dofile, config_dir .. "/xiu/toggles.lua")
    if ok and type(user) == "table" then
        for category, list in pairs(user) do
            apps[category] = apps[category] or {}
            for name, app in pairs(list) do apps[category][name] = app end
        end
    end
    return apps
end

local function field(window, key)
    local value = window[key]
    if value == nil and type(key) == "string" then
        value = window[key:gsub("(%u)", "_%1"):lower()]
    end
    return value
end

local function matches(window, match)
    for _, rule in ipairs(match) do
        local hit = true
        for key, want in pairs(rule) do
            local got = field(window, key)
            if got == nil or not tostring(got):find(tostring(want), 1, true) then
                hit = false
                break
            end
        end
        if hit then return true end
    end
    return false
end

local function quote(argv)
    local out = {}
    for i, arg in ipairs(argv) do
        out[i] = "'" .. tostring(arg):gsub("'", [['"'"']]) .. "'"
    end
    return table.concat(out, " ")
end

local function place(apps, category)
    local target = "special:" .. category
    for _, app in pairs(apps) do
        if app.match then
            local found = false
            local strays = {}
            for _, win in ipairs(hl.get_windows() or {}) do
                if matches(win, app.match) then
                    found = true
                    local ws = win.workspace and win.workspace.name
                    if ws ~= target then strays[#strays + 1] = win end
                end
            end
            if not found then
                if app.command then
                    hl.dispatch(hl.dsp.exec_cmd(quote(app.command), { workspace = target }))
                end
            elseif app.move ~= false then
                for _, win in ipairs(strays) do
                    hl.dispatch(hl.dsp.window.move({ window = win, workspace = target, follow = false }))
                end
            end
        end
    end
end

local function toggle(category)
    return function()
        local active = hl.get_active_special_workspace()
        local open = active and active.name == "special:" .. category
        if not open then
            hl.dispatch(hl.dsp.focus({ workspace = "special:" .. category }))
        end
        local apps = rules()[category]
        if apps then place(apps, category) end
        if open then
            hl.dispatch(hl.dsp.workspace.toggle_special(category))
        end
    end
end

return { toggle = toggle }
