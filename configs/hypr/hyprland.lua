require("modules.env")
require("modules.monitors")
require("modules.input")
require("modules.decoration")
require("modules.animations")
require("modules.gestures")
require("modules.binds")
require("rishot")
require("modules.window_rules")
require("modules.spaces-apply")
require("modules.autostart")

pcall(require, "modules.private")

-- Personal machine-only hooks (gitignored local.lua: discord, crosshair, ...)
pcall(require, "local")

-- GhostType hotkey (managed by the app)
pcall(require, "ghosttype")

-- Xiu user config, loaded dead last so it can build on every module above.
-- Lives outside the deployed tree (~/.config/xiu/user.lua) so updates never
-- touch it; modules/vars.lua merges its sibling vars.lua the same way.
pcall(dofile, (os.getenv("XDG_CONFIG_HOME") or (os.getenv("HOME") .. "/.config")) .. "/xiu/user.lua")
