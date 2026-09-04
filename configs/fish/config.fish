# Xiu fish: the full toolkit, distro-agnostic — nothing here depends on a
# distribution's fish package. Every tool is guarded so a missing binary
# never breaks the shell.

if type -q starship
    set -gx STARSHIP_CONFIG $__fish_config_dir/starship.toml
    starship init fish | source
end
if type -q direnv
    direnv hook fish | source
end
if type -q zoxide
    zoxide init fish | source
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

# Prompt marks (OSC 133) so the terminal can jump between prompts in scrollback
function mark_prompt_start --on-event fish_prompt
    printf '\e]133;A\e\\'
end

# No greeting at all: nothing prints between opening the terminal and the
# first prompt.
set -g fish_greeting
