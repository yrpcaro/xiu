pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

/**
 * Active keyboard layout for the lock's status bar, same recipe as the pill's
 * Keymap: the initial state comes from `hyprctl devices -j` and live changes
 * ride the activelayout event. `code` folds the verbose keymap name into the
 * two-letter code the chip shows.
 */
Singleton {
    id: root

    property string keymap: ""
    property var configuredLayouts: []

    /**
     * Fold a verbose keymap name into a clean 2-3 uppercase letter code:
     * 1. Strict 2-3 letter parenthetical country code filter ("English (US)" -> "US", "German (DE)" -> "DE").
     *    4+ letter parenthetical descriptors ("Persian (Windows)", "Russian (phonetic)") are XKB layout variants,
     *    NOT country codes, and are discarded.
     * 2. Direct Hyprland Config Integration: matches configured kb_layout codes when present.
     * 3. Base name resolution: strip parenthetical variants ("Persian (Windows)" -> "Persian") and match standard dictionary.
     * 4. Strict Abbreviation Invariant: clamp/truncate the output to a maximum of 2 or 3 uppercase letters (never "WINDOWS").
     */
    function codeFor(name) {
        if (!name || name.length === 0)
            return "";

        // 1. Strict 2-3 letter parenthetical filter
        var m = /\(([A-Za-z]{2,3})\)\s*$/.exec(name);
        if (m)
            return m[1].toUpperCase();

        // 2. Base name resolution: strip parenthetical variants
        var baseName = name.replace(/\([^)]+\)/g, "").trim();
        var n = baseName.toLowerCase();

        // 3. Direct Hyprland Config Integration: check configuredLayouts
        if (root.configuredLayouts && root.configuredLayouts.length > 0) {
            for (var i = 0; i < root.configuredLayouts.length; i++) {
                var c = root.configuredLayouts[i].trim().toLowerCase();
                if (c.length >= 2 && c.length <= 3) {
                    if (n === c) return c.toUpperCase();
                    if ((c === "ir" || c === "fa") && (n === "persian" || n === "farsi")) return c.toUpperCase();
                    if (c === "us" && n === "english") return "US";
                    if (c === "ru" && n === "russian") return "RU";
                    if (c === "de" && n === "german") return "DE";
                    if (c === "fr" && n === "french") return "FR";
                    if (c === "es" && n === "spanish") return "ES";
                    if (c === "it" && n === "italian") return "IT";
                    if (c === "ar" && (n === "arabic" || n === "arab")) return "AR";
                    if (c === "tr" && n === "turkish") return "TR";
                    if (c === "gr" && n === "greek") return "GR";
                    if (c === "pl" && n === "polish") return "PL";
                    if (c === "ua" && n === "ukrainian") return "UA";
                    if (c === "he" && n === "hebrew") return "HE";
                    if (c === "ja" && n === "japanese") return "JA";
                    if (c === "ko" && n === "korean") return "KO";
                    if (c === "zh" && n === "chinese") return "ZH";
                    if (c === "pt" && n === "portuguese") return "PT";
                    if (c === "nl" && n === "dutch") return "NL";
                    if (c === "se" && n === "swedish") return "SE";
                    if (c === "no" && n === "norwegian") return "NO";
                    if (c === "dk" && n === "danish") return "DK";
                    if (c === "fi" && n === "finnish") return "FI";
                    if (c === "cz" && n === "czech") return "CZ";
                    if (c === "hu" && n === "hungarian") return "HU";
                    if (c === "ro" && n === "romanian") return "RO";
                }
            }
        }

        var named = {
            "persian": "FA", "farsi": "FA", "arabic": "AR", "russian": "RU", "polish": "PL",
            "ukrainian": "UA", "greek": "GR", "turkish": "TR", "hebrew": "HE",
            "thai": "TH", "japanese": "JA", "korean": "KO", "chinese": "ZH",
            "french": "FR", "german": "DE", "spanish": "ES", "italian": "IT",
            "swedish": "SV", "norwegian": "NO", "danish": "DA", "finnish": "FI",
            "portuguese": "PT", "czech": "CS", "hungarian": "HU", "romanian": "RO",
            "english": "US", "dutch": "NL"
        };
        if (named[n] !== undefined)
            return named[n];

        // 4. Fallback: take first 2 or 3 letters of baseName, clamped strictly to 3 chars
        var abbrev = baseName.substring(0, Math.min(3, Math.max(2, baseName.length))).toUpperCase();
        return abbrev.substring(0, 3);
    }

    readonly property string code: codeFor(keymap)

    /**
     * How many layouts are configured, from `hyprctl getoption input:kb_layout`.
     * The status corner's layout chip only exists when there is more than one.
     */
    property int layoutCount: 0

    function applyLayoutCount(text) {
        try {
            var o = JSON.parse(text);
            var v = o && o.str ? String(o.str) : "";
            root.configuredLayouts = v.length ? v.split(",").map(function(s) { return s.trim(); }) : [];
            root.layoutCount = root.configuredLayouts.length;
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
        id: devicesProc
        command: ["hyprctl", "devices", "-j"]
        stdout: StdioCollector {
            onStreamFinished: root.applyDevices(text)
        }
    }

    Process {
        id: layoutsProc
        command: ["hyprctl", "getoption", "input:kb_layout", "-j"]
        stdout: StdioCollector {
            onStreamFinished: root.applyLayoutCount(text)
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
