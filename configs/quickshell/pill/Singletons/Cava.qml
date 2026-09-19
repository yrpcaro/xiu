pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire
import Quickshell.Services.Mpris

/**
 * Live audio spectrum for the rest-pill visualizer. A headless cava captures the
 * default sink monitor, so the bars answer to any system sound (music, a
 * background video, a game) instead of one MPRIS player. cava runs the FFT and
 * smoothing; we only parse its raw ascii frames into normalized 0..1 levels.
 *
 * Silence arrives as an all-zero frame every tick, which `active` debounces into
 * a clean play/stop signal so the glyph morph does not flap between tracks.
 *
 * cava is an optional dependency: the in-app updater only merges config files and
 * never installs packages, so a machine that pulled an update without cava on it
 * must degrade cleanly. We probe for the binary once and only ever spawn it when
 * it is actually present, which keeps the plain clock on those machines.
 *
 * To avoid idle CPU drain from continuous 30fps FFT calculation, cavaProc is
 * gated: when no audio is playing across MPRIS or Pipewire sink link groups,
 * cava shuts down completely after a hold/silence timeout and restarts
 * reactively as soon as audio playback begins.
 */
Singleton {
    id: root

    readonly property int bars: 5
    property var levels: []
    property bool active: false

    property bool available: false
    readonly property bool wanted: Flags.musicViz && !Flags.gameMode && available

    PwNodeLinkTracker {
        id: linkTracker
        node: Pipewire.defaultAudioSink
    }

    readonly property bool mprisPlaying: {
        var players = Mpris.players.values;
        for (var i = 0; i < players.length; i++) {
            if (players[i] && players[i].isPlaying)
                return true;
        }
        return false;
    }

    readonly property bool pipewireActive: {
        var links = linkTracker.linkGroups;
        if (!links || links.length === 0)
            return false;
        var players = Mpris.players.values;
        var playerNames = [];
        for (var i = 0; i < players.length; i++) {
            if (players[i] && players[i].identity)
                playerNames.push(players[i].identity.toLowerCase());
            if (players[i] && players[i].desktopEntry)
                playerNames.push(players[i].desktopEntry.toLowerCase());
        }
        for (var j = 0; j < links.length; j++) {
            var lg = links[j];
            var sName = (lg && lg.source && lg.source.name) ? lg.source.name.toLowerCase() : "";
            var isMpris = false;
            for (var k = 0; k < playerNames.length; k++) {
                if (playerNames[k] && (sName.indexOf(playerNames[k]) >= 0 || playerNames[k].indexOf(sName) >= 0)) {
                    isMpris = true;
                    break;
                }
            }
            if (!isMpris)
                return true;
        }
        return false;
    }

    readonly property bool playbackActive: mprisPlaying || pipewireActive
    property bool silenceTimedOut: false

    readonly property bool shouldRun: wanted && (playbackActive || holdTimer.running) && !silenceTimedOut

    onPlaybackActiveChanged: {
        if (playbackActive) {
            silenceTimedOut = false;
            holdTimer.stop();
        } else {
            holdTimer.restart();
        }
    }

    Timer {
        id: holdTimer
        interval: 1500
        onTriggered: root.updateRunning()
    }

    Timer {
        id: silenceTimer
        interval: 3000
        onTriggered: {
            if (!root.mprisPlaying)
                root.silenceTimedOut = true;
        }
    }

    onShouldRunChanged: updateRunning()

    function updateRunning() {
        var r = root.shouldRun;
        if (cavaProc.running !== r)
            cavaProc.running = r;
        if (!r) {
            root.active = false;
            root.levels = [];
        }
    }

    /**
     * autosens is off so a silent browser holding the sink stays at zero bars
     * instead of autosens amplifying the noise floor up to full range and tripping
     * the visualizer on dead silence. The trade is a fixed gain, tuned so real
     * music fills the bars while silence stays under the activate threshold.
     */
    readonly property string config: "[general]\n"
        + "bars = " + bars + "\nframerate = 30\nautosens = 0\nsensitivity = 5500\n"
        + "[input]\nmethod = pipewire\nsource = auto\n"
        + "[output]\nmethod = raw\nraw_target = /dev/stdout\ndata_format = ascii\n"
        + "ascii_max_range = 1000\nbar_delimiter = 59\nframe_delimiter = 10\n"
        + "channels = mono\nmono_option = average\n"
        + "[smoothing]\nnoise_reduction = 0.77\n"

    Component.onCompleted: updateRunning()

    Process {
        running: true
        command: ["sh", "-c", "command -v cava >/dev/null 2>&1"]
        onExited: (code) => {
            root.available = (code === 0);
            root.updateRunning();
        }
    }

    Process {
        id: cavaProc
        command: ["sh", "-c", "printf '%s' \"$1\" | cava -p /dev/stdin", "_", root.config]
        stdout: SplitParser {
            onRead: (line) => {
                if (!line)
                    return;
                const parts = line.split(";");
                const out = [];
                let peak = 0;
                for (let i = 0; i < root.bars; i++) {
                    const v = (parseInt(parts[i]) || 0) / 1000;
                    out.push(v);
                    if (v > peak)
                        peak = v;
                }
                /**
                 * Silence frames stop mattering once the morph has settled back
                 * to the clock, so skip the 60Hz levels churn while both the
                 * frame and the stored levels are already flat.
                 */
                const flat = peak <= 0.001 && !root.active;
                if (!flat)
                    root.levels = out;
                if (peak > 0.02) {
                    root.active = true;
                    root.silenceTimedOut = false;
                    idle.restart();
                    silenceTimer.restart();
                }
            }
        }
        /** A crash while cava is still wanted earns one relaunch after a beat, never a tight respawn loop. */
        onExited: (code) => {
            if (root.shouldRun)
                relaunch.restart();
        }
    }

    Timer {
        id: relaunch
        interval: 1500
        onTriggered: if (root.shouldRun) cavaProc.running = true
    }

    /** Short debounce so inter-track gaps do not snap the morph back to the clock. */
    Timer {
        id: idle
        interval: 450
        onTriggered: root.active = false
    }
}
