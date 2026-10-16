hl.env("XCURSOR_THEME",    "Bibata-Modern-Ice")
hl.env("XCURSOR_SIZE",     "24")
hl.env("HYPRCURSOR_THEME", "Bibata-Modern-Ice")
hl.env("HYPRCURSOR_SIZE",  "24")

-- The xiu CLI and the fallback tools install into ~/.local/bin, which login
-- managers never put on the session PATH — so keybinds calling `xiu` died on
-- boxes whose login shell never added it. fish re-adds it per shell in
-- config.fish; this prepend covers the compositor and everything it spawns.
hl.env("PATH", os.getenv("HOME") .. "/.local/bin:" .. (os.getenv("PATH") or ""))

-- Prioritize user-local data directory (~/.local/share) so Quickshell and Qt/GTK
-- apps always resolve user-installed icon themes, fonts, and desktop entries.
hl.env("XDG_DATA_DIRS", os.getenv("HOME") .. "/.local/share:" .. (os.getenv("XDG_DATA_DIRS") or "/usr/local/share:/usr/share"))

-- Default applications across compositor and spawned apps
local vars = require("modules.vars")
local ed = vars.editor or "helix"
local ed_clean = ed:match("%-e%s+([^%s]+)") or ed:match("([^%s]+)$") or ed
hl.env("EDITOR", ed_clean)
hl.env("VISUAL", ed_clean)
if vars.terminal then
    hl.env("TERMINAL", vars.terminal)
end
if vars.browser then
    hl.env("BROWSER", vars.browser)
end
if vars.imageViewer then
    hl.env("IMAGEVIEWER", vars.imageViewer)
end
if vars.videoPlayer then
    hl.env("VIDEOPLAYER", vars.videoPlayer)
end
if vars.documentViewer then
    hl.env("PDFVIEWER", vars.documentViewer)
end
if vars.musicPlayer then
    hl.env("MUSICPLAYER", vars.musicPlayer)
end

-- Hardware video acceleration and graphics environment.
-- Only apply Nvidia-specific variables when the proprietary nvidia driver is loaded.
local has_nvidia = (os.execute("test -c /dev/nvidia0") == 0) or (os.execute("test -d /proc/driver/nvidia") == 0)
if has_nvidia then
    hl.env("LIBVA_DRIVER_NAME",         "nvidia")
    hl.env("NVD_BACKEND",               "direct")
    hl.env("MOZ_DISABLE_RDD_SANDBOX",   "1")
    hl.env("__GLX_VENDOR_LIBRARY_NAME", "nvidia")
    hl.env("__GL_GSYNC_ALLOWED",        "0")
    hl.env("__GL_VRR_ALLOWED",          "0")
end

hl.env("ELECTRON_OZONE_PLATFORM_HINT", "auto")

hl.env("QT_QPA_PLATFORMTHEME", "kde")
hl.env("QT_STYLE_OVERRIDE", "Darkly")
hl.env("QT_USE_PORTAL", "1")
hl.env("GTK_USE_PORTAL", "1")
hl.env("QS_ICON_THEME", "yet-another-monochrome-icon-set")

-- rishot's auto-save folder and themed config dir.
hl.env("RISHOT_SAVEDIR", os.getenv("HOME") .. "/Pictures/Screenshots")
hl.env("RISHOT_CONFIG_DIR", os.getenv("HOME") .. "/.config/quickshell/rishot")
