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
    name   = "float-browser-untitled-popups",
    match  = {
        class         = "([Ff]irefox.*|[Zz]en.*|[Cc]hromium.*|[Gg]oogle-chrome.*|[Bb]rave-browser|[Bb]rave)",
        initial_title = "^($|Untitled( - .*)?)$",
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

-- Dynamic title & open listener (adapted from Caelestia)
-- Ensures browser popups and extension prompts that map with generic titles (e.g. Mozilla Firefox / Zen Browser)
-- are automatically floated and centered upon title update or creation.
local browser_classes = {
    ["firefox"] = true,
    ["zen"] = true,
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

local function is_browser_popup(win)
    if not is_browser_win(win) then return false end
    if win.floating then return false end
    if win.modal then return true end

    -- Standalone Chrome / Brave extension & app windows (exclude regular brave-browser)
    if win.class:match("^chrome%-") or (win.class:match("^brave%-") and not win.class:match("^brave%-browser")) then
        return true
    end

    -- Initial creation title: empty string or Untitled popup window in Chromium/Brave
    if win.initial_title and (win.initial_title == "" or win.initial_title:match("^Untitled")) then
        return true
    end

    -- Window titles for extension popups, Library, Page Info, and browser dialogs
    if win.title and win.title ~= "" then
        local t = win.title
        if t:match("^Extension: ") or t:match(" %- Extension$") or t:match("Extension Popup")
            or t:match("^Bitwarden") or t:match("^1Password")
            or t == "Library" or t:match("^Page Info")
            or t:match("^About Mozilla Firefox") or t:match("^About Zen")
            or t:match("wants to open$") or t:match("wants to save$") then
            return true
        end
    end

    return false
end

local function apply_browser_popup_float(win)
    if is_browser_popup(win) then
        hl.dispatch(hl.dsp.window.float({ action = "on", window = win }))
        hl.dispatch(hl.dsp.window.center({ window = win }))
    end
end

hl.on("window.open", apply_browser_popup_float)
hl.on("window.title", apply_browser_popup_float)

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
