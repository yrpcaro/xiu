/**
 * Launcher action commands.
 * Provides Caelestia/Xiu command parity (>wallpaper, >random, >theme, >variant,
 * >light, >dark, >calc, >install, >clipboard, >settings, >gamemode, >record,
 * >screenshot, >update, >lock, >sleep, >reboot, >poweroff, >logout).
 */

var ALL_COMMANDS = [
    {
        name: "Calculator",
        prefix: ">calc",
        aliases: ["=calc"],
        desc: "Evaluate arithmetic expressions (e.g. >calc 24 * 7)",
        icon: "stopwatch",
        needsArg: true,
        argPlaceholder: "<expression>"
    },
    {
        name: "Install AppImage",
        prefix: ">install",
        aliases: [">appimage"],
        desc: "Install an AppImage or package from path",
        icon: "download",
        needsArg: true,
        argPlaceholder: "<path/to/app.AppImage>"
    },
    {
        name: "Wallpaper Picker",
        prefix: ">wallpaper",
        aliases: [">wp"],
        desc: "Open wallpaper selector",
        icon: "palette",
        command: ["xiu", "open", "wallpaper"]
    },
    {
        name: "Random Wallpaper",
        prefix: ">random",
        desc: "Switch to a random wallpaper",
        icon: "sparkles",
        command: ["xiu", "wallpaper", "random"]
    },
    {
        name: "Theme / Appearance",
        prefix: ">theme",
        aliases: [">scheme"],
        desc: "Open appearance and theme switcher",
        icon: "palette",
        command: ["xiu", "open", "appearance"]
    },
    {
        name: "Scheme Variant",
        prefix: ">variant",
        desc: "Change theme color scheme variant",
        icon: "sparkles",
        command: ["xiu", "open", "appearance"]
    },
    {
        name: "Light Mode",
        prefix: ">light",
        desc: "Switch theme to light mode",
        icon: "sun",
        command: ["xiu", "theme", "light"]
    },
    {
        name: "Dark Mode",
        prefix: ">dark",
        desc: "Switch theme to dark mode",
        icon: "moon",
        command: ["xiu", "theme", "dark"]
    },
    {
        name: "Clipboard History",
        prefix: ">clipboard",
        aliases: [">clip"],
        desc: "Open clipboard manager",
        icon: "inbox",
        command: ["xiu", "open", "clipboard"]
    },
    {
        name: "Settings",
        prefix: ">settings",
        desc: "Open shell configuration and settings",
        icon: "cog",
        command: ["xiu", "open", "settings"]
    },
    {
        name: "Gamemode",
        prefix: ">gamemode",
        desc: "Toggle gaming performance mode",
        icon: "gamepad",
        command: ["xiu", "gamemode", "toggle"]
    },
    {
        name: "Screen Recorder",
        prefix: ">record",
        desc: "Open screen recorder",
        icon: "record",
        command: ["xiu", "open", "record"]
    },
    {
        name: "Screenshot",
        prefix: ">screenshot",
        desc: "Take a screenshot",
        icon: "app-window",
        command: ["xiu", "screenshot"]
    },
    {
        name: "Update Xiu",
        prefix: ">update",
        desc: "Check and install updates",
        icon: "download",
        command: ["xiu", "update"]
    },
    {
        name: "Lock Session",
        prefix: ">lock",
        desc: "Lock the screen session",
        icon: "lock",
        command: ["loginctl", "lock-session"]
    },
    {
        name: "Sleep / Suspend",
        prefix: ">sleep",
        desc: "Suspend system to RAM",
        icon: "suspend",
        command: ["systemctl", "suspend"]
    },
    {
        name: "Reboot",
        prefix: ">reboot",
        desc: "Restart the computer",
        icon: "reboot",
        command: ["systemctl", "reboot"]
    },
    {
        name: "Shutdown",
        prefix: ">poweroff",
        aliases: [">shutdown"],
        desc: "Shut down and power off",
        icon: "shutdown",
        command: ["systemctl", "poweroff"]
    },
    {
        name: "Log Out",
        prefix: ">logout",
        desc: "Exit desktop session",
        icon: "logout",
        command: ["hyprctl", "dispatch", "exit"]
    }
];

function matchCommands(query) {
    if (!query || query.indexOf(">") !== 0)
        return [];
    var q = query.trim();
    var spaceIdx = q.indexOf(" ");
    var verb = spaceIdx >= 0 ? q.substring(0, spaceIdx) : q;
    var arg = spaceIdx >= 0 ? q.substring(spaceIdx + 1).trim() : "";
    var verbLower = verb.toLowerCase();

    var results = [];
    for (var i = 0; i < ALL_COMMANDS.length; i++) {
        var cmd = ALL_COMMANDS[i];
        if (verbLower === ">") {
            results.push(Object.assign({}, cmd, { arg: arg, isCommand: true }));
            continue;
        }
        var matches = false;
        if (cmd.prefix.toLowerCase().indexOf(verbLower) === 0) {
            matches = true;
        } else if (cmd.aliases) {
            for (var a = 0; a < cmd.aliases.length; a++) {
                if (cmd.aliases[a].toLowerCase().indexOf(verbLower) === 0) {
                    matches = true;
                    break;
                }
            }
        } else if (cmd.name.toLowerCase().indexOf(verbLower.substring(1)) >= 0) {
            matches = true;
        }
        if (matches) {
            results.push(Object.assign({}, cmd, { arg: arg, isCommand: true }));
        }
    }
    return results;
}

if (typeof module !== "undefined" && module.exports) {
    module.exports = { ALL_COMMANDS, matchCommands };
}
