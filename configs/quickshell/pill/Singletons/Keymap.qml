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
    readonly property string code: (keymap.indexOf("Persian") >= 0 || keymap.indexOf("Farsi") >= 0) ? "FA" : (keymap.length > 0 ? "US" : "")

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

    Component.onCompleted: devicesProc.running = true

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
            if (parts.length >= 2)
                root.keymap = parts[parts.length - 1];
        }
    }
}
