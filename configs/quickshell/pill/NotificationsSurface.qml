pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import "Singletons"

/**
 * 知 NOTIFICATIONS sub-surface: pill flashes, dwell timing and muted apps.
 * Allows toggling specific OSD pop-ups (volume, brightness, media, workspace,
 * battery, screen recording), configuring their display duration, setting how
 * long toast notifications persist, and muting specific apps from pill flashes.
 */
SettingsSurface {
    id: root

    backSurface: "settings"
    implicitHeight: content.implicitHeight

    function flashOn(k) {
        return (Flags.osdOff || []).indexOf(k) < 0;
    }

    function setFlash(k, on) {
        var off = (Flags.osdOff || []).filter(function(o) { return o !== k; });
        if (!on)
            off.push(k);
        Flags.osdOff = off;
    }

    Column {
        id: content
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 0

        SettingsHeader {
            s: root.s
            glyph: "知"
            title: "NOTIFICATIONS"
            showBack: true
        }

        Item { width: 1; height: 10 * root.s }

        // ── Media ─────────────────────────────────────────────────────────
        SettingsGroupLabel {
            s: root.s
            leftPadding: 14 * root.s
            text: "MEDIA"
        }

        SettingsRow {
            surface: root
            name: "Play and pause"
            sub: "When music starts or stops"
            icon: "play"

            LinkToggle {
                s: root.s
                on: root.flashOn("play")
                onToggled: root.setFlash("play", !root.flashOn("play"))
            }
        }

        SettingsRow {
            surface: root
            name: "New song"
            sub: "When the track changes"
            icon: "music"

            LinkToggle {
                s: root.s
                on: root.flashOn("song")
                onToggled: root.setFlash("song", !root.flashOn("song"))
            }
        }

        Item { width: 1; height: 8 * root.s }

        // ── Pill flashes ──────────────────────────────────────────────────
        SettingsGroupLabel {
            s: root.s
            leftPadding: 14 * root.s
            text: "PILL FLASHES"
        }

        SettingsRow {
            surface: root
            name: "Volume"
            sub: "When you change the volume or mute"
            icon: "speaker"

            LinkToggle {
                s: root.s
                on: root.flashOn("volume")
                onToggled: root.setFlash("volume", !root.flashOn("volume"))
            }
        }

        SettingsRow {
            surface: root
            name: "Brightness"
            sub: "When the screen gets brighter or darker"
            icon: "sun"

            LinkToggle {
                s: root.s
                on: root.flashOn("brightness")
                onToggled: root.setFlash("brightness", !root.flashOn("brightness"))
            }
        }

        SettingsRow {
            surface: root
            name: "Workspace"
            sub: "The dots when you switch workspaces"
            icon: "layers"

            LinkToggle {
                s: root.s
                on: root.flashOn("workspace")
                onToggled: root.setFlash("workspace", !root.flashOn("workspace"))
            }
        }

        SettingsRow {
            visible: Battery.present
            surface: root
            name: "Charging"
            sub: "When you plug in the charger"
            icon: "bolt"

            LinkToggle {
                s: root.s
                on: root.flashOn("battery")
                onToggled: root.setFlash("battery", !root.flashOn("battery"))
            }
        }

        SettingsRow {
            surface: root
            name: "Screen recording"
            sub: "When a recording starts or stops"
            icon: "record"

            LinkToggle {
                s: root.s
                on: root.flashOn("record")
                onToggled: root.setFlash("record", !root.flashOn("record"))
            }
        }

        SettingsRow {
            surface: root
            name: "How long they stay"
            sub: "Scales every flash, the song card included"
            icon: "clock"

            SettingsSeg {
                s: root.s
                options: [
                    { label: "0.5×", value: 0.5 },
                    { label: "0.75×", value: 0.75 },
                    { label: "1×", value: 1.0 },
                    { label: "1.5×", value: 1.5 },
                    { label: "2×", value: 2.0 }
                ]
                value: Flags.osdTime
                onPicked: (v) => Flags.osdTime = v
            }
        }

        SettingsRow {
            surface: root
            name: "Do not disturb"
            sub: "Also hides media, workspace, charging and recording"
            icon: "dnd"

            LinkToggle {
                s: root.s
                on: Flags.dnd
                onToggled: Flags.dnd = !Flags.dnd
            }
        }

        Item { width: 1; height: 8 * root.s }

        // ── App notifications ─────────────────────────────────────────────
        SettingsGroupLabel {
            s: root.s
            leftPadding: 14 * root.s
            text: "APP NOTIFICATIONS"
        }

        SettingsRow {
            surface: root
            name: "Show for"
            sub: "How long a new notification stays in the pill"
            icon: "clock"

            SettingsSeg {
                s: root.s
                options: [
                    { label: "3 s", value: 3 },
                    { label: "4 s", value: 4 },
                    { label: "6 s", value: 6 },
                    { label: "8 s", value: 8 },
                    { label: "12 s", value: 12 }
                ]
                value: Flags.notifSecs
                onPicked: (v) => Flags.notifSecs = v
            }
        }

        Item { width: 1; height: 8 * root.s }

        // ── Apps ──────────────────────────────────────────────────────────
        SettingsGroupLabel {
            s: root.s
            leftPadding: 14 * root.s
            text: "APPS"
        }

        SettingsRow {
            visible: Notifs.knownApps.length === 0
            surface: root
            name: "No apps yet"
            sub: "Apps show up here after their first notification"
            icon: "inbox"
            last: true
        }

        Repeater {
            model: Notifs.knownApps

            SettingsRow {
                id: appRow
                required property var modelData
                required property int index
                readonly property string appName: String(modelData)

                surface: root
                name: appRow.appName
                sub: Notifs.isMuted(appRow.appName) ? "Only in the inbox" : "In the pill and the inbox"
                icon: "inbox"
                last: appRow.index === Notifs.knownApps.length - 1

                LinkToggle {
                    s: root.s
                    on: !Notifs.isMuted(appRow.appName)
                    onToggled: Notifs.setMuted(appRow.appName, !Notifs.isMuted(appRow.appName))
                }
            }
        }

        Item { width: 1; height: 6 * root.s }
    }
}
