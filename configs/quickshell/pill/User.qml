pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io
import "Singletons"

/**
 * 人 USER & SESSION sub-surface: user profile information, active session
 * and desktop details, and quick management of user override configuration
 * files (~/.config/xiu/vars.lua and ~/.config/xiu/user.lua).
 */
SettingsSurface {
    id: root

    backSurface: "settings"
    implicitHeight: content.implicitHeight
    rows: []

    readonly property string username: Quickshell.env("USER") || "user"
    readonly property string homeDir: Quickshell.env("HOME") || ("/home/" + username)
    readonly property string userShell: Quickshell.env("SHELL") || "/bin/sh"
    readonly property string sessionType: Quickshell.env("XDG_SESSION_TYPE") || "wayland"
    readonly property string currentDesktop: Quickshell.env("XDG_CURRENT_DESKTOP") || "Hyprland"

    readonly property string hostname: {
        var h = hostnameFile.text().trim();
        return h.length > 0 ? h : (Quickshell.env("HOSTNAME") || "localhost");
    }

    readonly property string uptimeStr: {
        var t = uptimeFile.text().trim().split(" ")[0];
        var sec = parseFloat(t);
        if (isNaN(sec) || sec <= 0) return "";
        var hrs = Math.floor(sec / 3600);
        var mins = Math.floor((sec % 3600) / 60);
        if (hrs > 0) return hrs + "h " + mins + "m";
        return mins + "m";
    }

    readonly property string varsPath: homeDir + "/.config/xiu/vars.lua"
    readonly property string userLuaPath: homeDir + "/.config/xiu/user.lua"
    readonly property string xiuConfigDir: homeDir + "/.config/xiu"

    readonly property bool varsExists: varsView.text().length > 0
    readonly property bool userLuaExists: userView.text().length > 0

    FileView {
        id: hostnameFile
        path: "/proc/sys/kernel/hostname"
        blockLoading: true
        printErrors: false
    }

    FileView {
        id: uptimeFile
        path: "/proc/uptime"
        blockLoading: true
        printErrors: false
    }

    FileView {
        id: varsView
        path: root.varsPath
        blockLoading: true
        printErrors: false
    }

    FileView {
        id: userView
        path: root.userLuaPath
        blockLoading: true
        printErrors: false
    }

    onActiveChanged: {
        if (root.active) {
            varsView.reload();
            userView.reload();
            uptimeFile.reload();
        }
    }

    Process {
        id: launchEditorProc
        property string targetFile: ""
        property string template: ""
        command: ["sh", "-c",
            "mkdir -p \"$HOME/.config/xiu\"; " +
            "if [ ! -f \"$1\" ]; then printf '%s\\n' \"$2\" > \"$1\"; fi; " +
            "if [ -n \"$EDITOR\" ]; then " +
            "  ${TERMINAL:-foot} -e $EDITOR \"$1\" & " +
            "elif command -v helix >/dev/null 2>&1; then " +
            "  ${TERMINAL:-foot} -e helix \"$1\" & " +
            "else " +
            "  xdg-open \"$1\" 2>/dev/null || ${TERMINAL:-foot} -e nano \"$1\" & " +
            "fi",
            "sh", targetFile, template]
        onExited: {
            varsView.reload();
            userView.reload();
        }
    }

    Process {
        id: openDirProc
        command: ["sh", "-c", "mkdir -p \"$HOME/.config/xiu\" && (xdg-open \"$HOME/.config/xiu\" 2>/dev/null || true)"]
    }

    function openVars() {
        launchEditorProc.targetFile = root.varsPath;
        launchEditorProc.template = "-- xiu personalization overrides\\n-- Sourced by ~/.config/hypr/modules/vars.lua\\nreturn {\\n    -- terminal    = \\\"foot\\\",\\n    -- browser     = \\\"brave\\\",\\n    -- editor      = \\\"foot -e helix\\\",\\n    -- fileManager = \\\"dolphin\\\",\\n    -- musicPlayer = \\\"spotify-launcher\\\",\\n}\\n";
        launchEditorProc.running = true;
    }

    function openUserLua() {
        launchEditorProc.targetFile = root.userLuaPath;
        launchEditorProc.template = "-- xiu user Hyprland overrides\\n-- Sourced by ~/.config/hypr/hyprland.lua on startup and reload.\\n-- Custom keybinds, window rules, or exec rules belong here.\\n";
        launchEditorProc.running = true;
    }

    Column {
        id: content
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 0

        SettingsHeader {
            s: root.s
            glyph: "人"
            title: "USER & SESSION"
        }

        // Profile Header Card
        Item {
            width: parent.width
            height: 74 * root.s

            Rectangle {
                anchors.fill: parent
                anchors.margins: 10 * root.s
                radius: Metrics.rCard * root.s
                color: Theme.frameBg
                border.width: Metrics.hairW(root.s)
                border.color: Theme.hairSoft

                Row {
                    anchors.fill: parent
                    anchors.leftMargin: 12 * root.s
                    anchors.rightMargin: 12 * root.s
                    spacing: 12 * root.s

                    // Avatar Circle
                    Rectangle {
                        anchors.verticalCenter: parent.verticalCenter
                        width: 36 * root.s
                        height: 36 * root.s
                        radius: width / 2
                        color: Qt.alpha(Theme.accent, 0.15)
                        border.width: Metrics.hairW(root.s)
                        border.color: Qt.alpha(Theme.accent, 0.4)

                        GlyphIcon {
                            anchors.centerIn: parent
                            width: 18 * root.s
                            height: 18 * root.s
                            name: "user"
                            color: Theme.accent
                            stroke: 1.8
                        }
                    }

                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 2 * root.s

                        Text {
                            text: root.username
                            color: Theme.cream
                            font.family: Theme.font
                            font.pixelSize: Metrics.tTitle * root.s
                            font.weight: Font.DemiBold
                        }

                        Text {
                            text: root.username + "@" + root.hostname
                            color: Theme.subtle
                            font.family: Theme.font
                            font.pixelSize: Metrics.tCaption * root.s
                        }
                    }

                    Item {
                        // Spacer
                        width: 1
                        height: parent.height
                    }

                    Item {
                        anchors.verticalCenter: parent.verticalCenter
                        width: uptimeText.implicitWidth
                        height: parent.height
                        visible: root.uptimeStr.length > 0

                        Text {
                            id: uptimeText
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.right: parent.right
                            text: "up " + root.uptimeStr
                            color: Theme.faint
                            font.family: Theme.font
                            font.pixelSize: Metrics.tCaption * root.s
                        }
                    }
                }
            }
        }

        // Section: Session Details
        SettingsGroupLabel {
            s: root.s
            leftPadding: 14 * root.s
            text: "SESSION DETAILS"
        }

        SettingsRow {
            surface: root
            icon: "monitor"
            name: "Compositor"
            sub: root.currentDesktop + " (" + root.sessionType + ")"
            control: Text {
                text: "Wayland"
                color: Theme.faint
                font.family: Theme.font
                font.pixelSize: Metrics.tBody * root.s
            }
        }

        SettingsRow {
            surface: root
            icon: "keyboard"
            name: "Login shell"
            sub: root.userShell
            control: Text {
                text: root.userShell.split("/").pop()
                color: Theme.faint
                font.family: Theme.font
                font.pixelSize: Metrics.tBody * root.s
            }
        }

        SettingsRow {
            surface: root
            icon: "app-window"
            name: "Home directory"
            sub: root.homeDir
            control: Text {
                text: "~"
                color: Theme.faint
                font.family: Theme.font
                font.pixelSize: Metrics.tBody * root.s
            }
        }

        // Section: User Overrides
        SettingsGroupLabel {
            s: root.s
            leftPadding: 14 * root.s
            text: "USER CONFIGURATION OVERRIDES"
        }

        SettingsRow {
            surface: root
            icon: "cog"
            name: "Variables override"
            sub: "vars.lua · " + (root.varsExists ? "Active" : "Not created")
            control: SegPill {
                s: root.s
                option: ({ label: root.varsExists ? "Edit" : "Create", value: "vars" })
                onPicked: root.openVars()
            }
        }

        SettingsRow {
            surface: root
            icon: "layers"
            name: "Hyprland override"
            sub: "user.lua · " + (root.userLuaExists ? "Active" : "Not created")
            control: SegPill {
                s: root.s
                option: ({ label: root.userLuaExists ? "Edit" : "Create", value: "userlua" })
                onPicked: root.openUserLua()
            }
        }

        SettingsRow {
            surface: root
            icon: "inbox"
            name: "Configuration root"
            sub: "~/.config/xiu/"
            last: true
            control: SegPill {
                s: root.s
                option: ({ label: "Reveal", value: "reveal" })
                onPicked: openDirProc.running = true
            }
        }

        Item {
            width: parent.width
            height: 12 * root.s
        }
    }
}
