hl.env("XCURSOR_THEME",   "Bibata-Modern-Ice")
hl.env("XCURSOR_SIZE",    "24")
hl.env("HYPRCURSOR_SIZE", "24")

-- The xiu CLI and the fallback tools install into ~/.local/bin, which login
-- managers never put on the session PATH — so keybinds calling `xiu` died on
-- boxes whose login shell never added it. fish re-adds it per shell in
-- config.fish; this prepend covers the compositor and everything it spawns.
hl.env("PATH", os.getenv("HOME") .. "/.local/bin:" .. (os.getenv("PATH") or ""))

hl.env("LIBVA_DRIVER_NAME",         "nvidia")
hl.env("NVD_BACKEND",               "direct")
hl.env("MOZ_DISABLE_RDD_SANDBOX",   "1")
hl.env("__GLX_VENDOR_LIBRARY_NAME", "nvidia")
hl.env("__GL_GSYNC_ALLOWED",        "0")
hl.env("__GL_VRR_ALLOWED",          "0")

hl.env("ELECTRON_OZONE_PLATFORM_HINT", "auto")

hl.env("QT_QPA_PLATFORMTHEME", "kde")

-- rishot's auto-save folder (it has no config file of its own; the env var
-- is the knob). Keep in sync with the dirs the pill creates at boot.
hl.env("RISHOT_SAVEDIR", os.getenv("HOME") .. "/Pictures/Screenshots")
