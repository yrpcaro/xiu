-- Xiu colorscheme: the palette arrives live from the shell's wallpaper
-- pipeline (~/.cache/xiu/colors.json and base16.json, rewritten on every
-- wallpaper or scheme change), with the washi fallback baked in for a cold start.
--
-- Enable with :colorscheme xiu (or colorscheme xiu in your init).

local function load_pill()
    local ok, decoded = pcall(function()
        local paths = {
            vim.fn.expand("~/.cache/xiu/colors.json"),
            vim.fn.expand("~/.cache/ricelin/colors.json"),
        }
        for _, path in ipairs(paths) do
            local f = io.open(path, "r")
            if f then
                local data = f:read("*a")
                f:close()
                local parsed = vim.json.decode(data)
                if parsed and parsed.primary and parsed.surface then
                    return parsed
                end
            end
        end
        return nil
    end)
    if ok and decoded then
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

local function load_base16(c)
    local ok, decoded = pcall(function()
        local paths = {
            vim.fn.expand("~/.cache/xiu/base16.json"),
            vim.fn.expand("~/.cache/ricelin/base16.json"),
        }
        for _, path in ipairs(paths) do
            local f = io.open(path, "r")
            if f then
                local data = f:read("*a")
                f:close()
                local parsed = vim.json.decode(data)
                if parsed and parsed.base08 then
                    return parsed
                end
            end
        end
        return nil
    end)
    if ok and decoded then
        return decoded
    end
    return {
        base00 = c.surface,
        base01 = c.surface_container_low,
        base02 = c.surface_container,
        base03 = c.surface_container_high,
        base04 = c.outline_variant,
        base05 = c.dim,
        base06 = c.subtle,
        base07 = c.cream,
        base08 = c.primary,
        base09 = c.on_primary_container or c.primary,
        base0a = c.tick_rest or c.primary,
        base0b = c.tick_rest or c.cream,
        base0c = c.subtle or c.cream,
        base0d = c.primary,
        base0e = c.primary_container or c.primary,
        base0f = c.outline or c.dim,
    }
end

local c = load_pill()
local b = load_base16(c)

vim.o.termguicolors = true

local groups = {
    -- Editor UI: transparent backgrounds, flat separators, calm inactive statusline
    Normal = { fg = c.cream, bg = "none" },
    NormalNC = { fg = c.cream, bg = "none" },
    NormalFloat = { fg = c.cream, bg = c.surface_container },
    FloatBorder = { fg = c.outline_variant, bg = c.surface_container },
    FloatTitle = { fg = c.bright, bg = c.surface_container, bold = true },
    SignColumn = { bg = "none" },
    FoldColumn = { fg = c.faint, bg = "none" },
    CursorLine = { bg = c.surface_container_low },
    CursorLineNr = { fg = c.subtle, bold = true },
    LineNr = { fg = c.faint },
    Visual = { bg = c.surface_container_high },
    Search = { fg = c.bright, bg = c.primary_container },
    IncSearch = { fg = c.bright, bg = c.primary },
    CurSearch = { fg = c.bright, bg = c.primary },
    StatusLine = { fg = c.cream, bg = c.surface_container },
    StatusLineNC = { fg = c.dim, bg = "none" },
    WinSeparator = { fg = c.surface_container_high, bg = "none" },
    VertSplit = { fg = c.surface_container_high, bg = "none" },
    TabLine = { fg = c.dim, bg = "none" },
    TabLineFill = { bg = "none" },
    TabLineSel = { fg = c.cream, bg = c.surface_container, bold = true },
    WinBar = { fg = c.subtle, bg = "none" },
    WinBarNC = { fg = c.faint, bg = "none" },
    Pmenu = { fg = c.cream, bg = c.surface_container },
    PmenuSel = { fg = c.bright, bg = c.surface_container_high, bold = true },
    PmenuSbar = { bg = c.surface_container },
    PmenuThumb = { bg = c.surface_container_high },
    ColorColumn = { bg = c.surface_container_low },
    Directory = { fg = c.primary },
    Title = { fg = c.bright, bold = true },
    NonText = { fg = c.outline_variant },
    SpecialKey = { fg = c.outline_variant },
    Folded = { fg = c.dim, bg = c.surface_container_low },

    -- Standard Syntax
    Comment = { fg = c.dim, italic = true },
    Constant = { fg = b.base0a },
    String = { fg = b.base0b },
    Character = { fg = b.base0e },
    Number = { fg = b.base09 },
    Boolean = { fg = b.base09 },
    Float = { fg = b.base09 },
    Identifier = { fg = c.cream },
    Function = { fg = b.base0d },
    Statement = { fg = c.primary },
    Conditional = { fg = c.primary },
    Repeat = { fg = c.primary },
    Label = { fg = c.primary },
    Operator = { fg = c.subtle },
    Keyword = { fg = c.primary },
    Exception = { fg = c.primary },
    PreProc = { fg = b.base0e },
    Include = { fg = c.primary },
    Define = { fg = b.base0e },
    Macro = { fg = b.base0e },
    PreCondit = { fg = b.base0e },
    Type = { fg = b.base0a },
    StorageClass = { fg = c.primary },
    Structure = { fg = b.base0a },
    Typedef = { fg = b.base0a },
    Special = { fg = b.base0e },
    SpecialChar = { fg = b.base0e },
    Tag = { fg = c.primary },
    Delimiter = { fg = c.dim },
    SpecialComment = { fg = c.subtle, italic = true },
    Debug = { fg = b.base08 },
    Underlined = { fg = c.primary, underline = true },
    Error = { fg = c.bright, bg = c.primary_container },
    Todo = { fg = c.primary, bold = true },

    -- Tree-sitter Tokens
    ["@comment"] = { fg = c.dim, italic = true },
    ["@comment.documentation"] = { fg = c.subtle, italic = true },
    ["@constant"] = { fg = b.base0a },
    ["@constant.builtin"] = { fg = b.base0a },
    ["@constant.macro"] = { fg = b.base0e },
    ["@string"] = { fg = b.base0b },
    ["@string.regex"] = { fg = b.base0c },
    ["@string.escape"] = { fg = b.base0e },
    ["@string.special"] = { fg = b.base0b },
    ["@character"] = { fg = b.base0e },
    ["@character.special"] = { fg = b.base0e },
    ["@number"] = { fg = b.base09 },
    ["@number.float"] = { fg = b.base09 },
    ["@boolean"] = { fg = b.base09 },
    ["@variable"] = { fg = c.cream },
    ["@variable.builtin"] = { fg = c.primary },
    ["@variable.parameter"] = { fg = c.subtle },
    ["@variable.member"] = { fg = c.bright },
    ["@function"] = { fg = b.base0d },
    ["@function.builtin"] = { fg = b.base0c },
    ["@function.macro"] = { fg = b.base0e },
    ["@function.method"] = { fg = b.base0d },
    ["@constructor"] = { fg = b.base0c },
    ["@keyword"] = { fg = c.primary },
    ["@keyword.function"] = { fg = c.primary },
    ["@keyword.operator"] = { fg = c.primary },
    ["@keyword.return"] = { fg = c.primary },
    ["@keyword.conditional"] = { fg = c.primary },
    ["@keyword.repeat"] = { fg = c.primary },
    ["@keyword.import"] = { fg = c.primary },
    ["@keyword.exception"] = { fg = c.primary },
    ["@keyword.directive"] = { fg = b.base0e },
    ["@type"] = { fg = b.base0a },
    ["@type.builtin"] = { fg = b.base0c },
    ["@type.definition"] = { fg = b.base0a },
    ["@type.qualifier"] = { fg = c.primary },
    ["@property"] = { fg = c.subtle },
    ["@field"] = { fg = c.subtle },
    ["@tag"] = { fg = c.primary },
    ["@tag.attribute"] = { fg = b.base0d },
    ["@tag.delimiter"] = { fg = c.faint },
    ["@operator"] = { fg = c.subtle },
    ["@punctuation.delimiter"] = { fg = c.dim },
    ["@punctuation.bracket"] = { fg = c.faint },
    ["@punctuation.special"] = { fg = c.primary },

    -- Markup (Markdown, Help files)
    ["@markup.heading"] = { fg = c.bright, bold = true },
    ["@markup.strong"] = { bold = true },
    ["@markup.italic"] = { italic = true },
    ["@markup.strikethrough"] = { strikethrough = true },
    ["@markup.link.url"] = { fg = c.dim, underline = true },
    ["@markup.link.label"] = { fg = c.primary },
    ["@markup.raw"] = { fg = b.base0b },
    ["@markup.list"] = { fg = c.primary },

    -- LSP Diagnostics
    DiagnosticError = { fg = b.base08 },
    DiagnosticWarn = { fg = b.base0a },
    DiagnosticInfo = { fg = b.base0c },
    DiagnosticHint = { fg = c.subtle },
    DiagnosticUnderlineError = { sp = b.base08, undercurl = true },
    DiagnosticUnderlineWarn = { sp = b.base0a, undercurl = true },
    DiagnosticUnderlineInfo = { sp = b.base0c, undercurl = true },
    DiagnosticUnderlineHint = { sp = c.subtle, undercurl = true },

    -- Diff & GitSigns
    GitSignsAdd = { fg = b.base0b },
    GitSignsChange = { fg = b.base0a },
    GitSignsDelete = { fg = b.base08 },
    DiffAdd = { fg = b.base0b, bg = "none" },
    DiffChange = { fg = b.base0a, bg = "none" },
    DiffDelete = { fg = b.base08, bg = "none" },
    DiffText = { fg = c.bright, bg = c.surface_container },
}

for name, attrs in pairs(groups) do
    vim.api.nvim_set_hl(0, name, attrs)
end

vim.g.colors_name = "xiu"
