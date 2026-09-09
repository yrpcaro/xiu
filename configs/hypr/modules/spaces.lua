-- User-defined special workspaces. The Settings page rewrites this file, so keep
-- each entry on this shape. id is the special-workspace name, key is a single
-- Super-prefixed letter, apps are window classes that auto-route in.
return {
	-- key "A": no plain SUPER+A bind exists (telegram spawns on SUPER+G and
	-- Spotify on SUPER+O), so the space toggle owns the letter.
	{ id = "discord", name = "Discord", desc = "", key = "A", apps = { "discord", "vesktop" } },
	-- key "T": free of every bind in binds.lua; org.telegram.desktop is the
	-- tdesktop class, so windows auto-route into the space.
	{ id = "telegram", name = "Telegram", desc = "", key = "T", apps = { "org.telegram.desktop" } },
}
