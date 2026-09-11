-- Xiu yazi init: plugin entry points, loaded by yazi at startup next to
-- yazi.toml. Only the plugins that need a setup call are here — the rest
-- (smart-enter, bypass, compress, chmod, diff, git, mount, the whole
-- yazi-rs set) are self-configuring and run straight off the keymap.

-- The statusline: yatline never draws without this call. The minimal
-- defaults already track the palette through theme.toml's mode colors.
require("yatline"):setup()

-- Bookmarks: optional — the defaults (no config) give the documented keys.
-- require("bookmarks"):setup()

-- yamb: bookmark config lives here per its README when wanted.
-- local bookmarks = {}
-- require("yamb"):setup({ bookmarks = bookmarks })
