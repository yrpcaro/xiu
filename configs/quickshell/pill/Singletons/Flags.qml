pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

/**
 * Shared session flags persisted to a small JSON file and watched for external
 * change, so every xiu daemon (pill, lock) reads and writes the same
 * Do-Not-Disturb and Keep-Awake state live without a second notification server
 * or idle inhibitor. Toggling in one surface updates the others on the next file
 * event, and the state survives a daemon restart.
 */
Singleton {
    id: root

    property alias dnd: adapter.dnd
    property alias keepAwake: adapter.keepAwake
    property alias time12h: adapter.time12h
    property alias clockSeconds: adapter.clockSeconds
    property alias showGlyphs: adapter.showGlyphs
    property alias paletteMode: adapter.paletteMode
    property alias wallpaperDir: adapter.wallpaperDir
    property alias randomScope: adapter.randomScope
    property alias uiScale: adapter.uiScale
    property alias reduceMotion: adapter.reduceMotion
    property alias manualHue: adapter.manualHue
    property alias manualDark: adapter.manualDark
    property alias manualSat: adapter.manualSat
    property alias uiFont: adapter.uiFont
    property alias pillOpacity: adapter.pillOpacity
    property alias pillBlur: adapter.pillBlur
    property alias topGap: adapter.topGap
    property alias appGap: adapter.appGap
    property alias autoHide: adapter.autoHide
    property alias autoHideDelay: adapter.autoHideDelay
    property alias recordCountdown: adapter.recordCountdown
    property alias recordDir: adapter.recordDir
    property alias recordFps: adapter.recordFps
    property alias recordQuality: adapter.recordQuality
    property alias recordCursor: adapter.recordCursor
    property alias recordMic: adapter.recordMic
    property alias recordDesktop: adapter.recordDesktop
    property alias recordClearedBefore: adapter.recordClearedBefore
    property alias idleLockMin: adapter.idleLockMin
    property alias idleScreenOffMin: adapter.idleScreenOffMin
    property alias idleSuspendMin: adapter.idleSuspendMin
    property alias weatherCity: adapter.weatherCity
    property alias weatherBackend: adapter.weatherBackend
    property alias weatherKey: adapter.weatherKey
    property alias musicViz: adapter.musicViz
    property alias gameMode: adapter.gameMode
    property alias gamePrevDnd: adapter.gamePrevDnd
    property alias gamePrevViz: adapter.gamePrevViz
    property alias gamePrevAwake: adapter.gamePrevAwake
    property alias gamePrevProfile: adapter.gamePrevProfile
    property alias nightLightMode: adapter.nightLightMode
    property alias nightLightTemp: adapter.nightLightTemp
    property alias nightLightOnMin: adapter.nightLightOnMin
    property alias nightLightOffMin: adapter.nightLightOffMin

    /**
     * True once flags.json has been read (or written from defaults): windows
     * that size off flag values wait for it so a commit made from the
     * adapter defaults is never followed by a visible resize.
     */
    readonly property bool loaded: root._loaded
    property bool _loaded: false

    /**
     * Auto-hide timing derived from autoHideDelay: how long the pointer must
     * dwell on the edge strip before the pill reveals, and how long it
     * lingers after the pointer leaves before retracting. "off" restores
     * instant behaviour in both directions.
     */
    readonly property var _delaySteps: ({ off: [0, 0], short: [120, 350], medium: [200, 600], long: [350, 1100] })
    readonly property int revealDwellMs: (_delaySteps[autoHideDelay] || _delaySteps.medium)[0]
    readonly property int hideLingerMs: (_delaySteps[autoHideDelay] || _delaySteps.medium)[1]

    FileView {
        id: file
        path: (Quickshell.env("XDG_STATE_HOME") || (Quickshell.env("HOME") + "/.local/state")) + "/ricelin/flags.json"
        blockLoading: true
        watchChanges: true
        printErrors: false

        onFileChanged: reload()
        onAdapterUpdated: writeAdapter()
        onLoaded: root._loaded = true
        onLoadFailed: function(error) {
            if (error === FileViewError.FileNotFound) {
                writeAdapter();
                root._loaded = true;
            }
        }

        JsonAdapter {
            id: adapter
            property bool dnd: false
            property bool keepAwake: false
            property bool time12h: false
            property bool clockSeconds: false
            property bool showGlyphs: true
            property string paletteMode: "static"
            /** Explicit wallpaper folder override. Empty means autodetect: the dir wallpaper.sh last resolved (ricelin-wallpaper-dir state file), then ~/Pictures/xiu/wallpapers. Lives in user state so an in-app update never clobbers a custom folder. */
            property string wallpaperDir: ""
            /** Super+B random target: "all" repaints every monitor, "cursor" only the one under the pointer. */
            property string randomScope: "all"
            property real uiScale: 1.0
            property bool reduceMotion: false
            property int manualHue: 30
            property bool manualDark: true
            property real manualSat: 0.5
            property string uiFont: ""
            property real pillOpacity: 1.0
            property bool pillBlur: false
            /** Top margin as a fraction of the shipped 8px. 0 sits the pill flush to the screen edge. */
            property real topGap: 1.0
            /** Pill-to-window band as a fraction of the shipped 12px. 0 tucks the windows flush under the pill. */
            property real appGap: 1.0
            /** CapsuleOS-style auto-hide: the pill retracts off the top edge when nothing needs it; dwell the screen edge to reveal. Off by default — the hover-pill behavior is the default contract. */
            property bool autoHide: false
            /** Reveal dwell / retract linger preset: off, short, medium, long. */
            property string autoHideDelay: "medium"
            property int recordCountdown: 5
            property string recordDir: ""
            property int recordFps: 60
            property string recordQuality: "high"
            property bool recordCursor: true
            property bool recordMic: true
            property bool recordDesktop: true
            property real recordClearedBefore: 0
            property int idleLockMin: 5
            property int idleScreenOffMin: 6
            property int idleSuspendMin: 0
            property string weatherCity: ""
            /** Weather source: "open-meteo" (keyless default) or "accuweather". */
            property string weatherBackend: "open-meteo"
            /** AccuWeather API key; empty leaves the backend on Open-Meteo. */
            property string weatherKey: ""
            property bool musicViz: true
            property bool gameMode: false
            property bool gamePrevDnd: false
            property bool gamePrevViz: true
            property bool gamePrevAwake: false
            property string gamePrevProfile: ""
            property string nightLightMode: "off"
            property int nightLightTemp: 4000
            property int nightLightOnMin: 1260
            property int nightLightOffMin: 450
        }
    }
}
