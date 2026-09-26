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
 *
 * clipvault only records what `wl-paste --watch clipvault store` feeds it, and
 * nothing else in the session runs that pipe. The autostart script starts it
 * once at boot, but a watcher that dies early (boot race, compositor restart)
 * used to stop history silently forever. So this singleton also heartbeats the
 * guarded script every 20s: it no-ops while the watcher lives, restarts it
 * when it doesn't, and the pgrep guard inside keeps it to one watcher however
 * often it runs. The heartbeat stays down while clipvault is missing.
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
    readonly property string watchScript: Quickshell.env("HOME") + "/.config/hypr/scripts/cliphist-watch.sh"

    readonly property string stateDir: (Quickshell.env("XDG_STATE_HOME") || (Quickshell.env("HOME") + "/.local/state")) + "/xiu"
    property var pinnedLines: []
    property var manualOrder: []

    FileView {
        id: pinnedStore
        path: root.stateDir + "/clipboard-pinned.json"
        blockLoading: true
        atomicWrites: true
        printErrors: false
        onLoaded: root.loadPinned()
        onLoadFailed: function(err) {
            Quickshell.execDetached(["mkdir", "-p", root.stateDir]);
            if (err === FileViewError.FileNotFound) {
                pinnedStore.setText("[]");
            }
        }
    }

    FileView {
        id: orderStore
        path: root.stateDir + "/clipboard-order.json"
        blockLoading: true
        atomicWrites: true
        printErrors: false
        onLoaded: root.loadOrder()
        onLoadFailed: function(err) {
            Quickshell.execDetached(["mkdir", "-p", root.stateDir]);
            if (err === FileViewError.FileNotFound) {
                orderStore.setText("[]");
            }
        }
    }

    function _entryKey(entry) {
        if (!entry) return "";
        var p = String(entry.preview || "");
        if (p.length > 0) return p;
        var l = String(entry.line || "");
        if (l.length > 0) return l;
        return String(entry.id || "");
    }

    function isPinned(entry) {
        if (!entry) return false;
        var p = String(entry.preview || "");
        var l = String(entry.line || "");
        var id = String(entry.id || "");
        for (var i = 0; i < pinnedLines.length; i++) {
            var item = String(pinnedLines[i]);
            if (item === p || item === l || (id.length && item === id))
                return true;
        }
        return false;
    }

    function pinnedIndex(entry) {
        if (!entry) return -1;
        var p = String(entry.preview || "");
        var l = String(entry.line || "");
        var id = String(entry.id || "");
        for (var i = 0; i < pinnedLines.length; i++) {
            var item = String(pinnedLines[i]);
            if (item === p || item === l || (id.length && item === id))
                return i;
        }
        return -1;
    }

    function pin(entry) {
        if (!entry || root.isPinned(entry)) return;
        var key = root._entryKey(entry);
        if (!key.length) return;
        var next = pinnedLines.slice();
        next.push(key);
        pinnedLines = next;
        pinnedStore.setText(JSON.stringify(pinnedLines));
        pinnedStore.waitForJob();
        var nextEntries = [];
        for (var j = 0; j < entries.length; j++) {
            var item = Object.assign({}, entries[j]);
            item.pinned = root.isPinned(item);
            nextEntries.push(item);
        }
        entries = nextEntries;
    }

    function unpin(entry) {
        if (!entry) return;
        var p = String(entry.preview || "");
        var l = String(entry.line || "");
        var id = String(entry.id || "");
        var next = [];
        for (var i = 0; i < pinnedLines.length; i++) {
            var item = String(pinnedLines[i]);
            if (item === p || item === l || (id.length && item === id)) {
                continue;
            }
            next.push(item);
        }
        pinnedLines = next;
        pinnedStore.setText(JSON.stringify(pinnedLines));
        pinnedStore.waitForJob();
        var nextUnpinned = [];
        for (var k = 0; k < entries.length; k++) {
            var unpItem = Object.assign({}, entries[k]);
            unpItem.pinned = root.isPinned(unpItem);
            nextUnpinned.push(unpItem);
        }
        entries = nextUnpinned;
    }

    function togglePin(entry) {
        if (!entry) return;
        if (root.isPinned(entry)) {
            root.unpin(entry);
        } else {
            root.pin(entry);
        }
    }

    function loadPinned() {
        try {
            var raw = pinnedStore.text();
            if (raw && raw.trim().length > 0) {
                var arr = JSON.parse(raw);
                if (Array.isArray(arr))
                    root.pinnedLines = arr;
            }
        } catch (e) {
            root.pinnedLines = [];
        }
    }

    function loadOrder() {
        try {
            var raw = orderStore.text();
            if (raw && raw.trim().length > 0) {
                var arr = JSON.parse(raw);
                if (Array.isArray(arr))
                    root.manualOrder = arr;
            }
        } catch (e) {
            root.manualOrder = [];
        }
    }

    function manualIndex(entry) {
        if (!entry) return 999999;
        var key = root._entryKey(entry);
        for (var i = 0; i < manualOrder.length; i++) {
            if (manualOrder[i] === key || manualOrder[i] === entry.line || manualOrder[i] === entry.preview)
                return i;
        }
        for (var j = 0; j < entries.length; j++) {
            if (entries[j] === entry || entries[j].id === entry.id)
                return 100000 + j;
        }
        return 999999;
    }

    function moveEntry(a, b, currentResults) {
        if (!a || !b || a.id === b.id) return;
        var isAPinned = root.isPinned(a);
        var isBPinned = root.isPinned(b);

        if (isAPinned && !isBPinned) {
            root.unpin(a);
        } else if (!isAPinned && isBPinned) {
            root.pin(a);
        }

        if (root.isPinned(a) && root.isPinned(b)) {
            var idxA = root.pinnedIndex(a);
            var idxB = root.pinnedIndex(b);
            if (idxA !== -1 && idxB !== -1 && idxA !== idxB) {
                var nextPins = pinnedLines.slice();
                var pItem = nextPins.splice(idxA, 1)[0];
                nextPins.splice(idxB, 0, pItem);
                pinnedLines = nextPins;
                pinnedStore.setText(JSON.stringify(pinnedLines));
                pinnedStore.waitForJob();
            }
        }

        var keyA = root._entryKey(a);
        var keyB = root._entryKey(b);
        var list = currentResults || entries;
        var newOrder = [];
        var seen = {};
        if (manualOrder && manualOrder.length > 0) {
            for (var m = 0; m < manualOrder.length; m++) {
                newOrder.push(manualOrder[m]);
                seen[manualOrder[m]] = true;
            }
        }
        for (var k = 0; k < list.length; k++) {
            var kKey = root._entryKey(list[k]);
            if (!seen[kKey]) {
                newOrder.push(kKey);
                seen[kKey] = true;
            }
        }
        var mIdxA = newOrder.indexOf(keyA);
        var mIdxB = newOrder.indexOf(keyB);
        if (mIdxA !== -1 && mIdxB !== -1 && mIdxA !== mIdxB) {
            var mItem = newOrder.splice(mIdxA, 1)[0];
            newOrder.splice(mIdxB, 0, mItem);
            manualOrder = newOrder;
            orderStore.setText(JSON.stringify(manualOrder));
            orderStore.waitForJob();
        }

        var eIdxA = -1, eIdxB = -1;
        for (var i = 0; i < entries.length; i++) {
            if (entries[i].id === a.id) eIdxA = i;
            if (entries[i].id === b.id) eIdxB = i;
        }
        if (eIdxA !== -1 && eIdxB !== -1 && eIdxA !== eIdxB) {
            var nextEntries = entries.slice();
            var eItem = nextEntries.splice(eIdxA, 1)[0];
            nextEntries.splice(eIdxB, 0, eItem);
            entries = nextEntries;
        } else {
            entries = entries.slice();
        }
    }

    function swapEntries(a, b, currentResults) {
        root.moveEntry(a, b, currentResults);
    }

    function kickStore() {
        Quickshell.execDetached(["sh", root.watchScript]);
    }

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
        var pinnedIds = [];
        var kept = [];
        for (var i = 0; i < entries.length; i++) {
            if (root.isPinned(entries[i])) {
                var nid = Number(entries[i].id);
                if (isFinite(nid) && nid > 0)
                    pinnedIds.push(nid);
                kept.push(entries[i]);
            }
        }
        entries = kept;
        if (pinnedIds.length === 0) {
            wipeProc.running = true;
        } else {
            var dbPath = Quickshell.env("CLIPVAULT_DB") || ((Quickshell.env("XDG_DATA_HOME") || (Quickshell.env("HOME") + "/.local/share")) + "/clipvault.db");
            var sql = "DELETE FROM clipboard WHERE id NOT IN (" + pinnedIds.join(",") + "); VACUUM;";
            selectiveWipeProc.command = ["sqlite3", dbPath, sql];
            selectiveWipeProc.running = true;
        }
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
        if (root.isPinned(entry)) {
            root.togglePin(entry);
        }
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
        onTriggered: {
            watchProc.running = true;
            if (!root.backendMissing)
                root.kickStore();
        }
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
        id: selectiveWipeProc
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
            var entryObj = {
                id: id,
                line: line,
                preview: preview,
                isImage: isImage,
                label: label,
                sizeLabel: sizeLabel,
                thumb: isImage ? root.thumbDir + id + ".png" : ""
            };
            entryObj.pinned = root.isPinned(entryObj);
            out.push(entryObj);
        }
        root.entries = out;
        root.loaded = true;
    }

    Process {
        id: listProc
        command: ["clipvault", "list"]
        stdout: StdioCollector { id: collected }
        onExited: (code) => {
            if (code !== 0) {
                console.warn("clipvault list failed with exit code " + code + ", retrying once");
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
        onExited: (code) => {
            root.backendMissing = code !== 0
            if (!root.backendMissing)
                root.kickStore()
        }
    }

    Component.onCompleted: {
        Quickshell.execDetached(["mkdir", "-p", root.stateDir]);
        root.loadPinned();
        root.loadOrder();
        probeProc.running = true;
        refresh();
    }
}
