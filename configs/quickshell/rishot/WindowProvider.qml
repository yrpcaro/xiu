import QtQuick
import Quickshell
import Quickshell.Io
import "lib/providers.js" as Providers

/**
 * Detects the running Wayland compositor and enumerates its windows in global
 * coordinates for region-mode window-snap.
 *
 * Supports Hyprland, Sway and Niri via their CLI query tools. On any other
 * compositor, or on a query failure, it emits an empty list so region and
 * monitor selection keep working unaffected. The result is delivered through
 * `windowsReady` after `refresh()` completes the underlying queries.
 */
Item {
    id: provider

    /**
     * Detected compositor: "hyprland", "sway", "niri" or "none".
     * Resolved from compositor-specific environment variables, with
     * XDG_CURRENT_DESKTOP as a secondary hint.
     */
    readonly property string kind: {
        if (Quickshell.env("HYPRLAND_INSTANCE_SIGNATURE")) return "hyprland";
        if (Quickshell.env("SWAYSOCK")) return "sway";
        if (Quickshell.env("NIRI_SOCKET")) return "niri";
        var desktop = (Quickshell.env("XDG_CURRENT_DESKTOP") || "").toLowerCase();
        if (desktop.indexOf("hyprland") !== -1) return "hyprland";
        if (desktop.indexOf("sway") !== -1) return "sway";
        if (desktop.indexOf("niri") !== -1) return "niri";
        return "none";
    }

    /**
     * Emitted once window enumeration finishes.
     * @param {Array<{x,y,w,h,z}>} rects Window rects in global coordinates.
     */
    signal windowsReady(var rects)

    /**
     * Query the active compositor and emit `windowsReady` with the result.
     * Compositors with no known query path emit an empty list immediately.
     */
    function refresh() {
        if (kind === "hyprland") hyprMonitorsProc.running = true;
        else if (kind === "sway") swayProc.running = true;
        else if (kind === "niri") niriWindowsProc.running = true;
        else provider.windowsReady([]);
    }

    Process {
        id: hyprMonitorsProc
        command: ["hyprctl", "monitors", "-j"]
        stdout: StdioCollector { id: hyprMonitorsOut }
        onExited: (code) => {
            if (code !== 0) { provider.windowsReady([]); return; }
            hyprClientsProc.running = true;
        }
    }

    Process {
        id: hyprClientsProc
        command: ["hyprctl", "clients", "-j"]
        stdout: StdioCollector { id: hyprClientsOut }
        onExited: (code) => {
            if (code !== 0) { provider.windowsReady([]); return; }
            provider.windowsReady(Providers.parseHyprland(hyprMonitorsOut.text, hyprClientsOut.text));
        }
    }

    Process {
        id: swayProc
        command: ["swaymsg", "-t", "get_tree"]
        stdout: StdioCollector { id: swayOut }
        onExited: (code) => {
            if (code !== 0) { provider.windowsReady([]); return; }
            provider.windowsReady(Providers.parseSway(swayOut.text));
        }
    }

    Process {
        id: niriWindowsProc
        command: ["niri", "msg", "--json", "windows"]
        stdout: StdioCollector { id: niriWindowsOut }
        onExited: (code) => {
            if (code !== 0) { provider.windowsReady([]); return; }
            niriWorkspacesProc.running = true;
        }
    }

    Process {
        id: niriWorkspacesProc
        command: ["niri", "msg", "--json", "workspaces"]
        stdout: StdioCollector { id: niriWorkspacesOut }
        onExited: (code) => {
            if (code !== 0) { provider.windowsReady([]); return; }
            niriOutputsProc.running = true;
        }
    }

    Process {
        id: niriOutputsProc
        command: ["niri", "msg", "--json", "outputs"]
        stdout: StdioCollector { id: niriOutputsOut }
        onExited: (code) => {
            if (code !== 0) { provider.windowsReady([]); return; }
            provider.windowsReady(Providers.parseNiri(
                niriWindowsOut.text, niriWorkspacesOut.text, niriOutputsOut.text));
        }
    }
}
