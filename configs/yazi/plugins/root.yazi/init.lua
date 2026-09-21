-- root.yazi: Modern Root component layout for Yazi
-- Fixes deprecated file:icon() by using th.icon.match(file) / th.icon:match(file)

local function get_icon(file)
	if not file then return nil end
	if th and th.icon then
		local ok, icon = pcall(function() return th.icon:match(file) end)
		if ok and icon then return icon end
		local ok2, icon2 = pcall(function() return th.icon.match(th.icon, file) end)
		if ok2 and icon2 then return icon2 end
		local ok3, icon3 = pcall(function() return th.icon.match(file) end)
		if ok3 and icon3 then return icon3 end
	end
	return nil
end

local function setup()
	-- Ensure full-border is applied with rounded borders
	pcall(function()
		require("full-border"):setup({ type = ui.Border.ROUNDED })
	end)
end

return {
	setup = setup,
	get_icon = get_icon,
}
