-- User-defined special workspaces. The Settings page rewrites this file, so keep
-- each entry on this shape. id is the special-workspace name, key is a single
-- Super-prefixed letter, apps are window classes that auto-route in.
return {
	-- key "D": Super+D for discord and discord workspace
	{ id = "discord", name = "Discord", desc = "", key = "D", apps = { "discord", "vesktop" } },
	-- No telegram space: Super+T spawns telegram directly (binds.lua);
	-- strays herd back through the communication workspace toggle instead.
}
