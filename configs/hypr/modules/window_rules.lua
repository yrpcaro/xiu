hl.window_rule({
    name           = "suppress-maximize",
    match          = { class = ".*" },
    suppress_event = "maximize",
})

hl.window_rule({
    name  = "fix-xwayland-drags",
    match = {
        class      = "^$",
        title      = "^$",
        xwayland   = true,
        float      = true,
        fullscreen = false,
        pin        = false,
    },
    no_focus = true,
})

hl.window_rule({
    name   = "float-system-dialogs",
    match  = { class = "(pavucontrol|nm-connection-editor|blueman-manager|org.kde.polkit-kde-authentication-agent-1|xdg-desktop-portal-.*|org\\.freedesktop\\.impl\\.portal\\.desktop\\..*|kdialog|org\\.kde\\.kdialog|[Zz]enity|[Yy]ad)" },
    float  = true,
    center = true,
})

hl.window_rule({
    name   = "float-portal-kde",
    match  = { class = "org.freedesktop.impl.portal.desktop.kde" },
    float  = true,
    size   = { 1200, 750 },
    center = true,
})

hl.window_rule({
    name   = "float-file-pickers",
    match  = { title = "(Open File|Save File|Save As|Choose Files|Choose File|Open Folder|Open Directory|Select File|Select Files|Select Folder|Select Directory|Select a File|Select a Folder|File Chooser|File Upload|Upload File|Choose Files to Upload|Select Files to Upload|Select File to Upload|Browse.*|Choose Directory|Import File|Export File|All Files)" },
    float  = true,
    center = true,
})

hl.window_rule({
    name  = "float-pip",
    match = { title = "^[Pp]icture(-| )in(-| )[Pp]icture$" },
    float = true,
})

-- Browser pop-up tabs/windows (Firefox, Chromium, Brave, Zen)
hl.window_rule({
    name   = "float-browser-modals",
    match  = {
        class = "([Ff]irefox.*|[Zz]en.*|[Cc]hromium.*|[Gg]oogle-chrome.*|[Bb]rave-browser|[Bb]rave)",
        modal = true,
    },
    float  = true,
    center = true,
})

hl.window_rule({
    name   = "float-browser-app-popups",
    match  = {
        class = "^(chrome-.*|brave-[a-zA-Z0-9]{10,}.*|brave-extension-.*)$",
    },
    float  = true,
    center = true,
})

hl.window_rule({
    name   = "float-browser-extension-popups",
    match  = {
        class = "([Ff]irefox.*|[Zz]en.*|[Cc]hromium.*|[Gg]oogle-chrome.*|[Bb]rave-browser|[Bb]rave)",
        title = "(^Extension: .*|.* - Extension$|.*Extension Popup.*|^Bitwarden.*|^1Password.*|^Library$|^Page Info — .*|^About (Mozilla Firefox|Zen Browser|Brave|Chromium)$|.*wants to (open|save)$)",
    },
    float  = true,
    center = true,
})

-- Dynamic geometry-based & title listener (adapted from Caelestia)
-- Floats browser popup windows whose dimensions are popup-sized (e.g. width <= 900 and height <= 850)
-- while ensuring normal browser sessions remain tiled by default.
local browser_classes = {
    ["firefox"] = true,
    ["zen"] = true,
    ["zen-browser"] = true,
    ["chromium"] = true,
    ["chromium-browser"] = true,
    ["google-chrome"] = true,
    ["brave-browser"] = true,
    ["brave"] = true,
}

local function is_browser_win(win)
    if not win or not win.class then return false end
    local c = win.class:lower()
    for b, _ in pairs(browser_classes) do
        if c:find(b, 1, true) then
            return true
        end
    end
    return false
end

local function count_browser_windows()
    local count = 0
    for _, w in ipairs(hl.get_windows()) do
        if is_browser_win(w) then
            count = count + 1
        end
    end
    return count
end

local function is_browser_popup(win, is_open_event)
    if not is_browser_win(win) then return false end
    if win.modal then return true end

    -- Standalone Chrome / Brave extension & app windows (exclude regular brave-browser)
    if win.class:match("^chrome%-") or (win.class:match("^brave%-") and not win.class:match("^brave%-browser")) then
        return true
    end

    -- Window titles for extension popups, Library, Page Info, and browser dialogs
    if win.title and win.title ~= "" then
        local t = win.title
        if t:match("^Extension: ") or t:match(" %- Extension$") or t:match("Extension Popup")
            or t:match("^Bitwarden") or t:match("^1Password")
            or t == "Library" or t:match("^Page Info")
            or t:match("^About Mozilla Firefox") or t:match("^About Zen")
            or t:match("^About Brave") or t:match("^About Chromium")
            or t:match("wants to open$") or t:match("wants to save$") then
            return true
        end
    end

    -- Dynamic geometry & popup detection on window creation:
    -- Only applies when an existing browser instance is already running (count > 1),
    -- ensuring primary browsing sessions always remain tiled.
    if is_open_event then
        local browser_count = count_browser_windows()
        if browser_count > 1 then
            -- Popups initialized with blank / about:blank / untitled
            if win.initial_title and (win.initial_title == "" or win.initial_title:match("^about:blank") or win.initial_title:match("^Untitled")) then
                return true
            end

            -- Geometry-based heuristic: requested dimensions are popup-sized (e.g. width < 900 and height < 800)
            if win.size and type(win.size.x) == "number" and type(win.size.y) == "number" then
                local w = win.size.x
                local h = win.size.y
                if w > 0 and h > 0 then
                    if w < 900 and h < 800 then
                        return true
                    end

                    if win.monitor and type(win.monitor.width) == "number" and type(win.monitor.height) == "number" then
                        local mw = win.monitor.width
                        local mh = win.monitor.height
                        if mw > 0 and mh > 0 then
                            if w <= math.floor(mw * 0.45) and h <= math.floor(mh * 0.75) then
                                return true
                            end
                        end
                    end
                end
            end
        end
    end

    return false
end

local function handle_browser_open(win)
    if not is_browser_win(win) then return end
    if is_browser_popup(win, true) then
        if not win.floating then
            hl.dispatch(hl.dsp.window.float({ action = "on", window = win }))
            hl.dispatch(hl.dsp.window.center({ window = win }))
        end
    end
end

local function handle_browser_title(win)
    if not is_browser_win(win) then return end
    if is_browser_popup(win, false) then
        if not win.floating then
            hl.dispatch(hl.dsp.window.float({ action = "on", window = win }))
            hl.dispatch(hl.dsp.window.center({ window = win }))
        end
    end
end

hl.on("window.open", handle_browser_open)
hl.on("window.title", handle_browser_title)

hl.window_rule({
    name   = "float-termfilechooser",
    match  = { class = "termfilechooser" },
    float  = true,
    size   = { 1200, 750 },
    center = true,
})

hl.window_rule({
    name  = "float-ghosttype",
    match = { class = "Ghosttype-app" },
    float = true,
})

hl.window_rule({
    name  = "float-qalculate",
    match = { class = "io.github.Qalculate.qalculate-qt" },
    float = true,
})

-- GUI File Managers (Dolphin, Thunar, Nautilus, Nemo, PCManFM, Caja, Krusader)
hl.window_rule({
    name   = "float-file-managers",
    match  = { class = "(org.kde.dolphin|[Tt]hunar|org.gnome.Nautilus|[Nn]autilus|[Nn]emo|[Pp]cmanfm|[Pp]cmanfm-qt|[Cc]aja|krusader|org.kde.krusader)" },
    float  = true,
    size   = { 1500, 850 },
    center = true,
})

hl.window_rule({
    name   = "float-file-managers-properties",
    match  = { class = "(org.kde.dolphin|[Tt]hunar|org.gnome.Nautilus|[Nn]autilus|[Nn]emo|[Pp]cmanfm|[Pp]cmanfm-qt|[Cc]aja|krusader|org.kde.krusader)", title = "(^Properties.*|.* - Properties$)" },
    float  = true,
    size   = { 500, 640 },
    center = true,
})

hl.window_rule({
    name   = "float-file-managers-progress",
    match  = { class = "(org.kde.dolphin|[Tt]hunar|org.gnome.Nautilus|[Nn]autilus|[Nn]emo|[Pp]cmanfm|[Pp]cmanfm-qt|[Cc]aja|krusader|org.kde.krusader)", title = "^(Copying|Moving|Deleting|Delete|File Operations|File Operation Progress|Progress).*" },
    float  = true,
    size   = { 520, 260 },
    center = true,
})

hl.window_rule({
    name   = "float-dolphin-configure",
    match  = { class = "org.kde.dolphin", title = "^Configure.*" },
    float  = true,
    size   = { 896, 684 },
    center = true,
})

hl.window_rule({
    name   = "float-thunar-preferences",
    match  = { class = "[Tt]hunar", title = "^(Thunar Preferences|Preferences)$" },
    float  = true,
    size   = { 600, 720 },
    center = true,
})

hl.window_rule({
    name   = "float-thunar-bulk-rename",
    match  = { class = "[Tt]hunar", title = "^Bulk Rename.*" },
    float  = true,
    size   = { 960, 640 },
    center = true,
})

hl.window_rule({
    name   = "float-file-managers-dialogs",
    match  = { class = "(org.kde.dolphin|[Tt]hunar|org.gnome.Nautilus|[Nn]autilus|[Nn]emo|[Pp]cmanfm|[Pp]cmanfm-qt|[Cc]aja|krusader|org.kde.krusader)", title = "^(Create New|New Folder|Rename|Confirm|Overwrite|Attention|Information|Warning|Question|Error|Set Default Application|Open With|Choose Application|Add Bookmark).*" },
    float  = true,
    size   = { 600, 245 },
    center = true,
})


hl.window_rule({
    name   = "float-ripdrag",
    match  = { class = "(ripdrag|.*ripdrag.*)" },
    float  = true,
    pin    = true,
    center = true,
})

hl.window_rule({
    name   = "float-vibelark",
    match  = { class = "App", title = "VibeLark" },
    float  = true,
    size   = { 1480, 960 },
    center = true,
})

hl.window_rule({
    name         = "idle-inhibit-fullscreen",
    match        = { class = ".*" },
    idle_inhibit = "fullscreen",
})

hl.window_rule({
    name      = "music-spotify",
    match     = { class = "[Ss]potify" },
    workspace = "special:spotify",
})

hl.window_rule({
    name      = "space-telegram",
    match     = { class = "(org.telegram.desktop|TelegramDesktop)" },
    workspace = "special:telegram",
})

-- Keep the pill from fading when the session lock hides and restores its layer,
-- so it never pops back in out of sync with the shot the lock morph collapses onto.
hl.layer_rule({
    name    = "pill-no-fade",
    match   = { namespace = "pill" },
    no_anim = true,
})

local ok, stashApps = pcall(require, "modules.stash-apps")
if ok and type(stashApps) == "table" then
    for _, cls in ipairs(stashApps) do
        hl.window_rule({
            name      = "stash-" .. cls,
            match     = { class = cls },
            workspace = "special:stash",
        })
    end
end
