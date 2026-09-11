pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

/**
 * Active keyboard layout, for the pill's layout chip. Quickshell's Hyprland
 * interface exposes no layout property, so the initial state comes from
 * `hyprctl devices -j` (the main keyboard's active keymap) and live changes
 * ride the activelayout event. `code` folds the verbose keymap name into
 * the two-letter code the chip shows.
 */
Singleton {
    id: root

    property string keymap: ""
    /**
     * Fold a verbose keymap name into the short code the chips show, for any
     * configured layout: the parenthetical country code when the name carries
     * one ("English (US)" -> "US", "German (DE)" -> "DE"), a small name map
     * for the languages whose xkb names have no parentheses (Persian -> FA),
     * else the first two letters of the name — never a hardcoded layout set.
     */
    function codeFor(name) {
        if (!name || name.length === 0)
            return "";
        var m = /\(([^)]+)\)\s*$/.exec(name);
        if (m)
            return m[1].toUpperCase();
        var named = {
            "persian": "FA", "arabic": "AR", "russian": "RU", "polish": "PL",
            "ukrainian": "UA", "greek": "GR", "turkish": "TR", "hebrew": "HE",
            "thai": "TH", "japanese": "JA", "korean": "KO", "chinese": "ZH"
        };
        var n = name.toLowerCase();
        if (named[n] !== undefined)
            return named[n];
        return name.substring(0, 2).toUpperCase();
    }

    readonly property string code: codeFor(keymap)

    /**
     * A monotonically increasing tick each time the layout actually changes
     * past the initial read — the pill's rest-state flash binds to this so
     * it can re-arm its timer on every switch without watching keymap
     * strings itself.
     */
    property int changed: 0

    /**
     * How many layouts are configured, from `hyprctl getoption input:kb_layout`.
     * The pill's layout chip (and the layout toggle's whole point) only exist
     * when there is more than one to switch between.
     */
    property int layoutCount: 0

    function applyLayoutCount(text) {
        try {
            var o = JSON.parse(text);
            var v = o && o.str ? String(o.str) : "";
            root.layoutCount = v.length ? v.split(",").length : 0;
        } catch (e) {
        }
    }

    function applyDevices(text) {
        try {
            var devs = JSON.parse(text);
            var kbs = devs && devs.keyboards ? devs.keyboards : [];
            for (var i = 0; i < kbs.length; i++) {
                if (kbs[i] && kbs[i].main) {
                    root.keymap = kbs[i].active_keymap || "";
                    return;
                }
            }
            if (kbs.length > 0 && kbs[0].active_keymap)
                root.keymap = kbs[0].active_keymap;
        } catch (e) {
        }
    }

    Component.onCompleted: {
        devicesProc.running = true;
        layoutsProc.running = true;
    }

    Process {
        id: layoutsProc
        command: ["hyprctl", "getoption", "input:kb_layout", "-j"]
        stdout: StdioCollector {
            onStreamFinished: root.applyLayoutCount(text)
        }
    }

    Process {
        id: devicesProc
        command: ["hyprctl", "devices", "-j"]
        stdout: StdioCollector {
            onStreamFinished: root.applyDevices(text)
        }
    }

    Connections {
        target: Hyprland
        function onRawEvent(event) {
            if (event.name !== "activelayout")
                return;
            var parts = String(event.data || "").split(",");
            if (parts.length >= 2) {
                var next = parts[parts.length - 1];
                if (next !== root.keymap)
                    root.changed += 1;
                root.keymap = next;
            }
        }
    }
}
