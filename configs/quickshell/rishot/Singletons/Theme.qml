pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: theme

    FileView {
        id: colorFile
        path: {
            var cache = Quickshell.env("XDG_CACHE_HOME") || (Quickshell.env("HOME") + "/.cache");
            return cache + "/xiu/colors.json";
        }
        blockLoading: true
        watchChanges: true
        printErrors: false
        onFileChanged: reload()

        JsonAdapter {
            id: dyn
            property string surface: "#18120b"
            property string surface_container: "#251f17"
            property string surface_container_low: "#211b13"
            property string surface_container_high: "#302921"
            property string surface_container_highest: "#3b342b"
            property string primary: "#e0563b"
            property string primary_container: "#a3371f"
            property string on_primary_container: "#ffddb3"
            property string outline: "#9c8f80"
            property string outline_variant: "#3a2a22"
            property string cream: "#e6d6cb"
            property string bright: "#fff6f0"
            property string subtle: "#b9a99e"
            property string dim: "#8a7d74"
            property string faint: "#6f635b"
            property string icon_dim: "#cdbfb4"
        }
    }

    readonly property color vermilion: dyn.primary || "#e0563b"
    readonly property color white:     dyn.bright || "#fff6f0"
    readonly property color idle:      dyn.cream || "#e6d6cb"
    readonly property color sep:       dyn.outline_variant || "#3a2a22"

    readonly property color dim:        Qt.rgba(8 / 255, 10 / 255, 16 / 255, 0.62)
    readonly property color glassBg:    Qt.alpha(dyn.surface_container || "#251f17", 0.94)
    readonly property color glassBorder: dyn.outline_variant || "#3a2a22"
    readonly property color panelBg:    Qt.alpha(dyn.surface_container_high || "#302921", 0.98)
    readonly property color panelBorder: dyn.outline_variant || "#4f4539"

    readonly property color dimIcon: dyn.icon_dim || Qt.rgba(0.77, 0.80, 0.85, 0.55)
    readonly property color winFill: Qt.alpha(vermilion, 0.16)
    readonly property color markerYellow: dyn.on_primary_container || "#f5d020"
    readonly property color stepText: white

    readonly property var swatches: [
        vermilion, white, dyn.surface || "#18120b", "#e23b3b", "#f2c14e", "#5bbf73", "#4f8fe0"
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
