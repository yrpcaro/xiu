pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris

/**
 * Audio levels for the lock screen, CapsuleOS-style: cava runs with the
 * lock's own assets/cava.conf (raw ascii, 12 bars, 0-100, 60fps) and each
 * line lands in `values` as 12 floats. The process only runs while the lock
 * is up AND something is actually playing — Mpris gates it, with a 1.5s hold
 * after a pause so the bars don't blink out mid-song — and `quiet` goes true
 * after 3s of silence so the renderer can fade instead of showing a flat
 * line. shell.qml flips `enabled` with the lock itself.
 */
Singleton {
    id: root

    readonly property int bars: 12
    property bool enabled: false
    property var values: []
    property bool quiet: true

    readonly property bool rawActive: {
        var l = Mpris.players.values;
        for (var i = 0; i < l.length; i++)
            if (l[i] && l[i].isPlaying)
                return true;
        return false;
    }

    readonly property bool active: rawActive || holdTimer.running

    onRawActiveChanged: {
        if (rawActive)
            holdTimer.stop();
        else
            holdTimer.restart();
    }

    onActiveChanged: {
        if (!active) {
            values = [];
            quiet = true;
            quietTimer.stop();
        }
    }

    Timer {
        id: holdTimer
        interval: 1500
    }

    Timer {
        id: quietTimer
        interval: 3000
        onTriggered: root.quiet = true
    }

    Process {
        running: root.active && root.enabled
        command: ["cava", "-p", Quickshell.shellPath("assets/cava.conf")]
        stdout: SplitParser {
            splitMarker: "\n"
            onRead: line => {
                if (!line || line.length === 0)
                    return;
                var parts = line.split(";");
                var out = [];
                var peak = 0;
                for (var i = 0; i < root.bars; i++) {
                    var v = parseInt(parts[i]);
                    var f = isNaN(v) ? 0 : Math.max(0, Math.min(1, v / 100));
                    if (f > peak)
                        peak = f;
                    out.push(f);
                }
                root.values = out;
                if (peak > 0.01) {
                    root.quiet = false;
                    quietTimer.restart();
                }
            }
        }
    }
}
