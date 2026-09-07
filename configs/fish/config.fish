# Xiu fish: the full toolkit, distro-agnostic — nothing here depends on a
# distribution's fish package. Every tool is guarded so a missing binary
# never breaks the shell.

# The installer drops the xiu CLI (and the fallback tools) into ~/.local/bin;
# make sure that is on PATH even when the profile does not add it.
fish_add_path -g ~/.local/bin ~/.cargo/bin

if type -q starship
    set -gx STARSHIP_CONFIG $__fish_config_dir/starship.toml
    starship init fish | source
end
if type -q direnv
    direnv hook fish | source
end
# zoxide owns cd itself, the caelestia way: plain `cd` gets the fuzzy jump.
if type -q zoxide
    zoxide init fish --cmd cd | source
end

abbr -a ff fastfetch

# eza is ls; the l-family rides on it
if type -q eza
    alias ls 'eza --icons --group-directories-first -1'
end
abbr -a l 'ls'
abbr -a ll 'eza --icons --group-directories-first -la'
abbr -a la 'eza --icons --group-directories-first -a'
abbr -a lla 'eza --icons --group-directories-first -la'

# git, one keystroke at a time; lg opens gitui
abbr -a lg gitui
abbr -a gd 'git diff'
abbr -a ga 'git add .'
abbr -a gc 'git commit -am'
abbr -a gl 'git log'
abbr -a gs 'git status'
abbr -a gst 'git stash'
abbr -a gsp 'git stash pop'
abbr -a gp 'git push'
abbr -a gpl 'git pull'
abbr -a gsw 'git switch'
abbr -a gsm 'git switch main'
abbr -a gb 'git branch'
abbr -a gbd 'git branch -d'
abbr -a gco 'git checkout'
abbr -a gsh 'git show'

# Rust coreutils: interactive defaults only — scripts never see these
if type -q bat
    alias cat 'bat --paging=never'
end
if type -q fd
    alias find fd
end
if type -q rg
    alias grep rg
end
if type -q dust
    alias du dust
end
# rip2's rip is rm with a safety net: deletions land in the graveyard
# (~/.local/share/rip) and `rip -u` brings them back; `command rm` is still
# the real thing when you mean it.
if type -q rip
    alias rm rip
end

# nvim owns the vi/vim names when it is around
if type -q nvim
    alias vim nvim
    alias vi nvim
end

# yazi closes into the directory you were in (the official shell wrapper)
if type -q yazi
    function y -w yazi -d 'file manager; quits back into the dir you were in'
        set tmp (mktemp -t "yazi-cwd.XXXXXX")
        command yazi $argv --cwd-file="$tmp"
        if read -z cwd < "$tmp"; and [ "$cwd" != "$PWD" ]; and test -d "$cwd"
            builtin cd -- "$cwd"
        end
        command rm -f -- "$tmp"
    end
end

# Terminals that never read a config file still open in the current palette:
# replay the wallcolors OSC sequences, guarded so piped shells stay clean.
if test -t 1; and test -f ~/.cache/ricelin/sequences.txt
    cat ~/.cache/ricelin/sequences.txt
end

# Prompt marks (OSC 133) so the terminal can jump between prompts in scrollback
function mark_prompt_start --on-event fish_prompt
    printf '\e]133;A\e\\'
end

# Syntax colors follow the wallpaper: wallcolors.py rewrites syntax.fish on
# every palette change. Sourced before the user's own file so a hand
# override still wins; a missing file leaves fish's defaults alone.
if test -f $__fish_config_dir/syntax.fish
    source $__fish_config_dir/syntax.fish
end

# fzf rides the terminal's own palette: ANSI slots for the chrome, the pill's
# accent for the pointer, read fresh at every shell start.
if type -q fzf
    set -gx FZF_DEFAULT_OPTS "--color=fg:7,bg:-1,hl:6,fg+:15,bg+:0,hl+:14,info:8,border:8,prompt:4,marker:13,spinner:8,header:8"
    set -l _xiu_accent (jq -r '.primary // empty' ~/.cache/ricelin/colors.json 2>/dev/null)
    if test -n "$_xiu_accent"
        set -gx FZF_DEFAULT_OPTS "$FZF_DEFAULT_OPTS,pointer:$_xiu_accent"
    end
    source /usr/share/fzf/key-bindings.fish 2>/dev/null
    source /usr/share/fzf/completions.fish 2>/dev/null
end

# No greeting at all: nothing prints between opening the terminal and the
# first prompt.
set -g fish_greeting

# Your own fish bits, kept outside the deployed tree like xiu-vars.lua on the
# hypr side, so updates never touch them.
if test -f ~/.config/xiu/user-config.fish
    source ~/.config/xiu/user-config.fish
end
