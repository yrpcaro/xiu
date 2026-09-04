pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

/**
 * Clipboard history bridge over clipvault: keeps a warm in-memory snapshot so
 * the clipboard surface opens instantly without shelling out on demand. A
 * wl-paste watcher fires on every clipboard change; after a short debounce the
 * thumbnail script regenerates missing image previews (and prunes stale ones),
 * then `clipvault list` is re-read into `entries`. Thumbnails are written before
 * the list lands so image delegates never bind to a not-yet-existing file. A
 * change arriving while the pipeline runs sets `pending` and replays once the
 * list lands, so no clipboard event is ever silently dropped; the watcher
 * respawns through a cooldown timer if wl-paste dies.
 *
 * clipvault addresses entries by their whole list line, so every entry carries
 * its line and copy/delete pipe it back in. Entries are plain objects:
 * { id, line, preview, isImage, meta, label, sizeLabel, thumb } where meta is
 * the raw binary descriptor ("245 KiB image/png 1920x1080"), label/sizeLabel
 * its display split ("png 1920×1080" / "245 KiB") and thumb the absolute path
 * of the cached preview png (empty for text).
 */
Singleton {
    id: root

    property var entries: []
    readonly property int count: entries.length
    property bool pending: false

    /**
     * False until one `clipvault list` succeeded, so the surface can tell a
     * genuinely empty history apart from a failed early-boot read.
     */
    property bool loaded: false

    /**
     * True when the clipvault binary is not on PATH, so the surface can say
     * that instead of a silent, forever-empty list. Probed once at startup:
     * a missing backend explains both a failed read and a watcher that
     * stores nothing.
     */
    property bool backendMissing: false

    readonly property string thumbDir: (Quickshell.env("XDG_CACHE_HOME") || (Quickshell.env("HOME") + "/.cache")) + "/clipvault-thumbs/"
    readonly property string thumbScript: Quickshell.env("HOME") + "/.config/hypr/scripts/cliphist-thumbs.sh"

    function refresh() {
        if (thumbProc.running || listProc.running || delProc.running || delQueue.length) {
            pending = true;
            return;
        }
        thumbProc.running = true;
    }

    function copy(entry) {
        if (!entry.line || entry.line.indexOf("\t") < 1)
            return;
        Quickshell.execDetached(["sh", "-c", "printf '%s' \"$1\" | clipvault get | wl-copy", "_", String(entry.line)]);
    }

    function wipe() {
        entries = [];
        wipeProc.running = true;
    }

    /**
     * Deletes are queued through a tracked process and any refresh is held
     * until the queue drains: a fire-and-forget delete racing an in-flight
     * list used to resurrect the removed entry from the stale snapshot. The
     * local prune stays optimistic so the row vanishes immediately. The
     * queue carries whole list lines — that is how clipvault addresses
     * entries — not bare ids.
     */
    property var delQueue: []

    function remove(entry) {
        if (!entry.line || entry.line.indexOf("\t") < 1)
            return;
        var id = String(entry.id);
        var kept = [];
        for (var i = 0; i < entries.length; i++)
            if (entries[i].id !== id)
                kept.push(entries[i]);
        entries = kept;
        delQueue.push(String(entry.line));
        pumpDeletes();
    }

    function pumpDeletes() {
        if (delProc.running || !delQueue.length)
            return;
        var line = delQueue.shift();
        delProc.command = ["sh", "-c", "printf '%s' \"$1\" | clipvault delete", "_", line];
        delProc.running = true;
    }

    Process {
        id: delProc
        onExited: {
            if (root.delQueue.length)
                root.pumpDeletes();
            else
                root.refresh();
        }
    }

    Process {
        id: watchProc
        command: ["wl-paste", "--watch", "echo", "x"]
        running: true
        stdout: SplitParser {
            onRead: debounce.restart()
        }
        onExited: respawn.restart()
    }

    Timer {
        id: respawn
        interval: 2000
        onTriggered: watchProc.running = true
    }

    Timer {
        id: debounce
        interval: 300
        onTriggered: root.refresh()
    }

    Process {
        id: wipeProc
        command: ["clipvault", "clear"]
        onExited: root.refresh()
    }

    Process {
        id: thumbProc
        command: ["sh", root.thumbScript]
        onExited: listProc.running = true
    }

    /**
     * A failed read (boot-time store lock, db hiccup) must not wipe the last
     * good snapshot; one quiet retry heals the race without looping.
     */
    Timer {
        id: listRetry
        interval: 2000
        onTriggered: root.refresh()
    }

    function applyList(text) {
        var lines = text.split("\n");
        var out = [];
        var metaRe = /^\[\[ binary data (.*) \]\]$/;
        var imgRe = /\b(png|jpg|jpeg|gif|bmp|webp)\b/;
        var splitRe = /^(\S+ \S+) (\S+) (\d+)x(\d+)$/;
        for (var i = 0; i < lines.length; i++) {
            var line = lines[i];
            var tab = line.indexOf("\t");
            if (tab < 1)
                continue;
            var id = line.substring(0, tab);
            if (!/^\d+$/.test(id))
                continue;
            var preview = line.substring(tab + 1);
            var m = metaRe.exec(preview);
            var isImage = m !== null && imgRe.test(m[1]);
            var label = "";
            var sizeLabel = "";
            if (isImage) {
                var p = splitRe.exec(m[1]);
                if (p) {
                    var type = p[2].indexOf("/") >= 0 ? p[2].substring(p[2].lastIndexOf("/") + 1) : p[2];
                    label = type + " " + p[3] + "×" + p[4];
                    sizeLabel = p[1];
                } else {
                    label = m[1];
                }
            }
            out.push({
                id: id,
                line: line,
                preview: preview,
                isImage: isImage,
                label: label,
                sizeLabel: sizeLabel,
                thumb: isImage ? root.thumbDir + id + ".png" : ""
            });
        }
        root.entries = out;
        root.loaded = true;
    }

    Process {
        id: listProc
        command: ["clipvault", "list"]
        stdout: StdioCollector { id: collected }
        onExited: {
            if (listProc.exitCode !== 0) {
                console.warn("clipvault list failed with exit code " + listProc.exitCode + ", retrying once");
                root.pending = false;
                listRetry.restart();
                return;
            }
            root.applyList(collected.text);
            if (root.pending) {
                root.pending = false;
                Qt.callLater(root.refresh);
            }
        }
    }

    Process {
        id: probeProc
        command: ["sh", "-c", "command -v clipvault >/dev/null 2>&1"]
        onExited: root.backendMissing = exitCode !== 0
    }

    Component.onCompleted: {
        probeProc.running = true;
        refresh();
    }
}
