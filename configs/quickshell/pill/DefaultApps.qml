pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io
import "Singletons"

/**
 * 規 DEFAULT APPS sub-surface: which application owns the xdg defaults —
 * folders, images, documents. Reads the current handler per category from
 * ~/.config/mimeapps.list and writes a pick back through `xdg-mime default`,
 * the same mechanism the installer seeds, so a change lands system-wide the
 * moment it is picked and every delegating surface (xdg-open, the apps'
 * open-containing-folder, the portal's fallback) follows.
 *
 * The offered apps per category are the ones this rice installs and any
 * already-registered handler the file carries — a box running something
 * else keeps it as an option. Reached from the settings index and morphs
 * back to it on an empty click or the back chevron.
 */
SettingsSurface {
    id: root

    backSurface: "settings"
    implicitHeight: content.implicitHeight

    readonly property string mimeappsPath: Quickshell.env("HOME") + "/.config/mimeapps.list"
    /** One representative mime per category — the one xdg-mime asks about. */
    readonly property var categories: [
        { label: "Browser", key: "x-scheme-handler/http", mimes: ["x-scheme-handler/http", "x-scheme-handler/https"], icon: "ethernet", discover: "x-scheme-handler/http" },
        { label: "Terminal", key: "x-scheme-handler/terminal", mimes: ["x-scheme-handler/terminal"], icon: "keyboard", discover: "terminal" },
        { label: "Folders", key: "inode/directory", mimes: ["inode/directory"], icon: "app-window" },
        { label: "Images", key: "image/png", mimes: ["image/png", "image/jpeg", "image/gif", "image/webp", "image/avif"], icon: "palette" },
        { label: "Documents", key: "application/pdf", mimes: ["application/pdf", "application/epub+zip"], icon: "layers" },
        { label: "Media", key: "video/x-matroska", mimes: ["video/x-matroska", "audio/mpeg", "audio/ogg", "video/mp4"], icon: "music", discover: "audio/" }
    ]

    /** The rice's curated picks per category, offered first when installed. */
    readonly property var preferred: {
        var p = {
            "inode/directory": ["xiu-yazi.desktop", "org.kde.dolphin.desktop", "thunar.desktop"],
            "image/png": ["imv.desktop", "imv-dir.desktop", "org.gnome.Loupe.desktop", "feh.desktop"],
            "application/pdf": ["org.pwmt.zathura.desktop", "org.pwmt.zathura-pdf-mupdf.desktop", "org.gnome.Papers.desktop"],
            "x-scheme-handler/http": [],
            "x-scheme-handler/terminal": [],
            "video/x-matroska": []
        };
        return p;
    }

    /**
     * The apps offered per category. Curated picks first, then discovery:
     * categories with a `discover` token scan the installed .desktop files
     * for entries that declare the token (a mime prefix like audio/, or
     * terminal for the Terminal=true class), so every installed browser,
     * terminal and player is offered without hardcoding a single id. The
     * current handler joins even when nothing else found it.
     */
    function appsFor(cat) {
        var out = [];
        var seen = {};
        var candidates = (preferred[cat.key] || []).slice();
        if (cat.discover)
            candidates = candidates.concat(discoverApps(cat.discover));
        // the current handler joins the list even when not a shipped candidate
        var cur = handlerFor(cat.key);
        if (cur.length > 0 && candidates.indexOf(cur) < 0)
            candidates.unshift(cur);
        for (var i = 0; i < candidates.length; i++) {
            var id = candidates[i];
            if (seen[id] || !desktopExists(id))
                continue;
            seen[id] = true;
            out.push({ label: desktopLabel(id), value: id });
        }
        return out;
    }

    /**
     * Installed .desktop ids declaring a discovery token: browsers and media
     * players through their MimeType= line, terminals through Terminal=true.
     * The desktop probe's ls gives us the ids; a second pass over the files'
     * own text (same cat, cheap on the settings open) gives the declarations.
     */
    function discoverApps(token) {
        var out = [];
        if (token === "terminal") {
            var lines = desktopCollected.text.split("\n");
            var current = "";
            var isTerminal = false;
            for (var i = 0; i < lines.length; i++) {
                var l = lines[i];
                if (l.indexOf("=== ") === 0) {
                    if (current && isTerminal)
                        out.push(current);
                    current = l.slice(4).trim();
                    isTerminal = false;
                } else if (l.indexOf("Terminal=true") === 0) {
                    isTerminal = true;
                }
            }
            if (current && isTerminal)
                out.push(current);
            return out;
        }
        // mime-token discovery: the probe carries "id: mime-list" lines
        var decls = desktopCollected.text.split("\n");
        for (var j = 0; j < decls.length; j++) {
            var d = decls[j];
            var c = d.indexOf(": ");
            if (c < 1)
                continue;
            if (d.slice(c + 2).indexOf(token) >= 0)
                out.push(d.slice(0, c));
        }
        return out;
    }

    function desktopExists(id) {
        return desktopProc.exists.indexOf(id) >= 0;
    }

    /** Reads the current handler for one category's representative mime. */
    function handlerFor(key) {
        var lines = mimeFile.text().split("\n");
        var inDefaults = false;
        for (var i = 0; i < lines.length; i++) {
            var l = lines[i];
            if (l.indexOf("[") === 0) {
                inDefaults = l.indexOf("Default Applications") >= 0;
                continue;
            }
            if (!inDefaults)
                continue;
            var eq = l.indexOf("=");
            if (eq < 1)
                continue;
            if (l.slice(0, eq).trim() === key)
                return l.slice(eq + 1).trim().split(";")[0];
        }
        return "";
    }

    /** A display name from a .desktop id — the basename, cleaned. */
    function desktopLabel(id) {
        var base = id.replace(/\.desktop$/, "");
        var short = base.split(".").pop();
        return short.charAt(0).toUpperCase() + short.slice(1);
    }

    /** Applies one category's pick through xdg-mime, then re-reads. */
    function apply(cat, desktopId) {
        for (var i = 0; i < cat.mimes.length; i++)
            xdgProc.run(cat.mimes[i], desktopId);
    }

    FileView {
        id: mimeFile
        path: root.mimeappsPath
        blockLoading: true
        printErrors: false
        onFileChanged: reload()
    }

    /**
     * The .desktop probe, one pass over both applications dirs: it emits
     * "id: mimetypes" for every entry (Terminal=true entries get the
     * terminal token), which feeds both the existence check and the
     * category discovery — browsers and media players are found by their
     * declared mimes, terminals by their class. Offered apps gate on the
     * existence list so the page never offers a handler that cannot run.
     */
    Process {
        id: desktopProc
        property var exists: []
        command: ["sh", "-c",
            "for f in /usr/share/applications/*.desktop "
            + "\"$HOME\"/.local/share/applications/*.desktop; do "
            + "[ -f \"$f\" ] || continue; "
            + "id=$(basename \"$f\"); "
            + "mimes=$(grep -h '^MimeType=' \"$f\" | cut -d= -f2); "
            + "grep -q -e '^Terminal=true' -e 'TerminalEmulator' \"$f\" && term=\" terminal\" || term=\"\"; "
            + "echo \"$id: $mimes$term\"; done"]
        stdout: StdioCollector { id: desktopCollected }
        onExited: {
            var lines = desktopCollected.text.split("\n");
            var out = [];
            for (var i = 0; i < lines.length; i++) {
                var l = lines[i];
                var c = l.indexOf(": ");
                if (c > 0)
                    out.push(l.slice(0, c));
            }
            desktopProc.exists = out;
        }
        Component.onCompleted: running = true
    }

    /**
     * xdg-mime default runner: queued so a multi-mime category applies in
     * order, and a final mimeapps re-read lands after the last write.
     */
    Process {
        id: xdgProc
        property var queue: []
        function run(mime, desktopId) {
            queue.push([mime, desktopId]);
            pump();
        }
        function pump() {
            if (running || queue.length === 0)
                return;
            var next = queue.shift();
            command = ["xdg-mime", "default", next[1], next[0]];
            running = true;
        }
        onExited: {
            if (queue.length === 0)
                mimeFile.reload();
            else
                pump();
        }
    }

    Column {
        id: content
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 0

        SettingsHeader {
            s: root.s
            glyph: "規"
            title: "DEFAULT APPS"
        }

        Repeater {
            model: root.categories

            delegate: SettingsRow {
                id: catRow
                required property var modelData
                surface: root

                icon: catRow.modelData.icon
                name: catRow.modelData.label
                sub: root.handlerFor(catRow.modelData.key).length > 0
                    ? "Currently " + root.desktopLabel(root.handlerFor(catRow.modelData.key))
                    : "No default set"

                SettingsSeg {
                    s: root.s
                    options: root.appsFor(catRow.modelData)
                    value: root.handlerFor(catRow.modelData.key)
                    onPicked: v => root.apply(catRow.modelData, v)
                }
            }
        }
    }
}
