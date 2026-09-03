-- Xiu colorscheme: the palette arrives live from the shell's wallpaper
-- pipeline (~/.cache/ricelin/colors.json, rewritten on every wallpaper or
-- scheme change), with the washi fallback baked in for a cold start.
--
-- Enable with :colorscheme xiu (or colorscheme xiu in your init).

local function load()
    local ok, decoded = pcall(function()
        local path = vim.fn.expand("~/.cache/ricelin/colors.json")
        local f = io.open(path, "r")
        if not f then
            return nil
        end
        local data = f:read("*a")
        f:close()
        return vim.json.decode(data)
    end)
    if ok and decoded and decoded.primary and decoded.surface then
        return decoded
    end
    return {
        surface = "#1c120c", surface_container_low = "#211711",
        surface_container = "#251a13", surface_container_high = "#2e231b",
        surface_container_highest = "#382b21", outline_variant = "#3a2a22",
        primary = "#e0563b", primary_container = "#a3371f",
        on_primary_container = "#ffb38a", outline = "#6f635b",
        cream = "#e6d6cb", bright = "#fff6f0", subtle = "#b9a99e",
        dim = "#8a7d74", faint = "#6f635b", icon_dim = "#cdbfb4",
        tick_rest = "#cbb6a3",
    }
end

local c = load()

vim.o.termguicolors = true

local groups = {
    Normal = { fg = c.cream, bg = c.surface },
    NormalFloat = { fg = c.cream, bg = c.surface_container },
    FloatBorder = { fg = c.outline_variant, bg = c.surface_container },
    CursorLine = { bg = c.surface_container_low },
    CursorLineNr = { fg = c.subtle },
    LineNr = { fg = c.faint },
    Visual = { bg = c.surface_container_high },
    Search = { fg = c.bright, bg = c.primary_container },
    IncSearch = { fg = c.bright, bg = c.primary },
    StatusLine = { fg = c.cream, bg = c.surface_container_high },
    StatusLineNC = { fg = c.dim, bg = c.surface_container_low },
    WinBar = { fg = c.subtle, bg = c.surface_container },
    WinBarNC = { fg = c.faint, bg = c.surface_container_low },
    Pmenu = { fg = c.cream, bg = c.surface_container },
    PmenuSel = { fg = c.bright, bg = c.surface_container_high },
    PmenuSbar = { bg = c.surface_container },
    PmenuThumb = { bg = c.surface_container_high },
    Comment = { fg = c.dim, italic = true },
    Constant = { fg = c.on_primary_container },
    String = { fg = c.tick_rest },
    Number = { fg = c.on_primary_container },
    Identifier = { fg = c.cream },
    Function = { fg = c.primary },
    Keyword = { fg = c.primary },
    Statement = { fg = c.primary },
    Type = { fg = c.subtle },
    Operator = { fg = c.subtle },
    Delimiter = { fg = c.faint },
    Error = { fg = c.bright, bg = c.primary_container },
    Todo = { fg = c.primary, bold = true },
    DiffAdd = { bg = c.surface_container_low },
    DiffChange = { bg = c.surface_container },
    DiffDelete = { fg = c.primary },
    Directory = { fg = c.primary },
    Title = { fg = c.bright, bold = true },
    NonText = { fg = c.outline_variant },
    SpecialKey = { fg = c.outline_variant },
    VertSplit = { fg = c.outline_variant },
    SignColumn = { bg = c.surface },
    Folded = { fg = c.dim, bg = c.surface_container_low },
}

for name, attrs in pairs(groups) do
    vim.api.nvim_set_hl(0, name, attrs)
end

vim.g.colors_name = "xiu"
