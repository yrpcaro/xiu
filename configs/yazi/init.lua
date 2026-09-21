-- Xiu yazi init: plugin entry points and modern styling
-- Uses th.icon.match(file) instead of deprecated file:icon()

-- 1. Full rounded borders for an ultra-modern aesthetic
pcall(function()
    require("full-border"):setup({
        type = ui.Border.ROUNDED,
    })
end)

-- 2. Modern statusline & deprecation guard (yatline)
local ok_yatline, yatline = pcall(require, "yatline")
if ok_yatline and yatline then
    -- Guard: replace deprecated hovered:icon() with th.icon:match(file) / th.icon.match(file)
    if yatline.string and yatline.string.get then
        yatline.string.get.hovered_file_extension = function(self, show_icon)
            local hovered = cx.active.current.hovered
            if not hovered then
                return ""
            end
            local name = hovered.cha.is_dir and "dir" or (hovered.url.name and hovered.url.name:match("%.([^%.]+)$") or "")
            if show_icon then
                local icon = nil
                if th and th.icon then
                    local ok, res = pcall(function() return th.icon:match(hovered) end)
                    if ok and res then
                        icon = res
                    else
                        local ok2, res2 = pcall(function() return th.icon.match(hovered) end)
                        if ok2 and res2 then icon = res2 end
                    end
                end
                local icon_str = (icon and icon.text) or (hovered.cha.is_dir and "" or "")
                return icon_str .. " " .. name
            else
                return name
            end
        end
    end

    yatline:setup({
        section_separator = { open = "", close = "" },
        part_separator = { open = "", close = "" },
        inverse_separator = { open = "", close = "" },
        style_a = {
            fg = "black",
            bg_mode = {
                normal = "white",
                select = "yellow",
                un_set = "red",
            },
        },
        selected = { icon = "󰻭", fg = "yellow" },
        copied   = { icon = "", fg = "green" },
        cut      = { icon = "", fg = "red" },
        files    = { icon = "", fg = "blue" },
        total    = { icon = "󰮍", fg = "yellow" },
        success  = { icon = "", fg = "green" },
        failed   = { icon = "", fg = "red" },
    })
end

-- 3. Modern custom linemodes
function Linemode:size_and_mtime()
    local time = math.floor(self._file.cha.mtime or 0)
    local timestr = time == 0 and "" or os.date("%Y-%m-%d %H:%M", time)
    local size = self._file:size()
    return ui.Line(string.format("%s  %s", size and ya.readable_size(size) or "-", timestr))
end

-- Global guard: intercept deprecated File:icon() across all plugins (e.g. root.yazi)
if File then
    File.icon = function(self, opts)
        if th and th.icon then
            local ok, icon = pcall(function() return th.icon:match(self, opts) end)
            if ok and icon then return icon end
            local ok2, icon2 = pcall(function() return th.icon.match(self, opts) end)
            if ok2 and icon2 then return icon2 end
        end
        return nil
    end
end

-- 4. Modern Header with breadcrumb folder icon
if Header and Header.cwd then
    Header.cwd = function(self)
        local max = (self._area and self._area.w or 80) - (self._right_width or 0)
        if max <= 0 then return "" end
        local cwd_str = tostring(self._current and self._current.cwd or (cx and cx.active and cx.active.current and cx.active.current.cwd) or "")
        local flags_str = self.flags and self:flags() or ""
        local s = ya.readable_path(cwd_str) .. flags_str
        return ui.Line {
            ui.Span("   "):style(th.mgr.cwd),
            ui.Span(ui.truncate(s, { max = math.max(1, max - 4), rtl = true })):style(th.mgr.cwd):bold(),
        }
    end
end
