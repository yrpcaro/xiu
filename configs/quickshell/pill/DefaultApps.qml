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
        { label: "Folders", key: "inode/directory", mimes: ["inode/directory"], icon: "app-window" },
        { label: "Images", key: "image/png", mimes: ["image/png", "image/jpeg", "image/gif", "image/webp", "image/avif"], icon: "palette" },
        { label: "Documents", key: "application/pdf", mimes: ["application/pdf", "application/epub+zip"], icon: "layers" }
    ]

    /**
     * The apps offered per category. The rice's own picks plus whatever the
     * file already names, so an existing default is always selectable. Only
     * entries whose .desktop exists are offered.
     */
    function appsFor(cat) {
        var out = [];
        var seen = {};
        var candidates = [];
        if (cat.key === "inode/directory")
            candidates = ["xiu-yazi.desktop", "org.kde.dolphin.desktop", "thunar.desktop"];
        else if (cat.key === "image/png")
            candidates = ["imv.desktop", "imv-dir.desktop", "org.gnome.Loupe.desktop", "feh.desktop"];
        else
            candidates = ["org.pwmt.zathura.desktop", "org.pwmt.zathura-pdf-mupdf.desktop", "org.gnome.Papers.desktop"];
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
     * The .desktop availability probe: the file names under
     * /usr/share/applications and the user's local overrides dir. Offered
     * apps gate on this so the page never offers a handler that cannot run.
     */
    Process {
        id: desktopProc
        property var exists: []
        command: ["sh", "-c",
            "ls /usr/share/applications \"$HOME/.local/share/applications\" 2>/dev/null || true"]
        stdout: StdioCollector { id: desktopCollected }
        onExited: {
            var names = desktopCollected.text.split("\n");
            var out = [];
            for (var i = 0; i < names.length; i++) {
                var n = names[i].trim();
                if (n.length > 0 && n.indexOf(".desktop") === n.length - 8)
                    out.push(n);
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
