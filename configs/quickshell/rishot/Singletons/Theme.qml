pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: theme

    FileView {
        id: colorFile
        path: (Quickshell.env("XDG_CACHE_HOME") || (Quickshell.env("HOME") + "/.cache")) + "/ricelin/colors.json"
        blockLoading: true
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
    }

    FileView {
        id: colorFileXiu
        path: (Quickshell.env("XDG_CACHE_HOME") || (Quickshell.env("HOME") + "/.cache")) + "/xiu/colors.json"
        blockLoading: true
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
    }

    function _readColors() {
        try {
            var tx = colorFileXiu.text();
            if (tx && tx.length > 0) {
                var ox = JSON.parse(tx);
                if (ox && ox.primary) return ox;
            }
        } catch (e) {}
        try {
            var t = colorFile.text();
            if (t && t.length > 0) {
                var o = JSON.parse(t);
                if (o && o.primary) return o;
            }
        } catch (e) {}
        return {};
    }

    readonly property var dyn: {
        void colorFile.text;
        void colorFileXiu.text;
        return theme._readColors();
    }

    readonly property color vermilion: dyn.primary || "#e0563b"
    readonly property color white:     dyn.bright || "#fff6f0"
    readonly property color idle:      dyn.cream || "#e6d6cb"
    readonly property color sep:       dyn.outline_variant || "#3a2a22"

    readonly property color dim:        Qt.alpha(dyn.surface || "#18120b", 0.62)
    readonly property color glassBg:    Qt.alpha(dyn.surface_container || "#251f17", 0.94)
    readonly property color glassBorder: dyn.outline_variant || "#3a2a22"
    readonly property color panelBg:    Qt.alpha(dyn.surface_container_high || "#302921", 0.98)
    readonly property color panelBorder: dyn.outline_variant || "#4f4539"

    readonly property color dimIcon: dyn.icon_dim || dyn.dim || Qt.rgba(0.77, 0.80, 0.85, 0.55)
    readonly property color winFill: Qt.alpha(vermilion, 0.16)
    readonly property color markerYellow: dyn.on_primary_container || "#f5d020"
    readonly property color stepText: white

    readonly property var swatches: [
        vermilion, white, dyn.surface || "#18120b", dyn.on_primary_container || "#f2c14e", dyn.primary_container || "#e23b3b", "#5bbf73", "#4f8fe0"
    ]

    readonly property string monoFamily: pick(
        ["JetBrainsMono Nerd Font", "JetBrains Mono", "DejaVu Sans Mono", "Liberation Mono"],
        "monospace")
    readonly property string sansFamily: pick(
        ["Inter", "Inter Display", "Noto Sans", "DejaVu Sans", "Liberation Sans"],
        "sans-serif")

    /**
     * Returns the first installed family from prefs, or the generic fallback
     * when none are present. Lets rishot ship without bundling fonts.
     */
    function pick(prefs, fallback) {
        var fams = Qt.fontFamilies();
        for (var i = 0; i < prefs.length; i++)
            if (fams.indexOf(prefs[i]) !== -1) return prefs[i];
        return fallback;
    }
}
