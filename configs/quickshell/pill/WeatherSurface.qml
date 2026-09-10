pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import "Singletons"

/**
 * 天気 WEATHER sub-surface: the full readout the glance only samples —
 * current conditions with wind, feels-like, humidity and pressure, an
 * 8-hour strip, and the five-day forecast. The city and the backend switch
 * live at the top: the inline city field geocodes on commit (Weather
 * re-locates through the Flags listener), and the Open-Meteo / AccuWeather
 * segmented control flips Flags.weatherBackend — keyless Open-Meteo by
 * default, AccuWeather when its key is set (an empty key falls back with a
 * note saying why). Reached from the pill's weather glance and the settings
 * index; morphs back the same ways.
 */
SettingsSurface {
    id: root

    backSurface: "settings"
    implicitHeight: content.implicitHeight

    readonly property var backendOptions: [
        { label: "Open-Meteo", value: "open-meteo" },
        { label: "AccuWeather", value: "accuweather" }
    ]

    /** Wind direction degrees to a compass point. */
    function dirFor(deg) {
        if (deg < 0 || deg > 360)
            return "";
        var points = ["N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW"];
        return points[Math.round(deg / 22.5) % 16];
    }

    Column {
        id: content
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 0

        SettingsHeader {
            s: root.s
            glyph: "天"
            title: "WEATHER"
            // The weather surface opens from the pill's weather glance, not
            // the settings index, but it still steps back to the index like
            // every other sub-surface — the chevron, not the settings cog.
            showBack: true
        }

        // ── city + backend ────────────────────────────────────────────────
        Item {
            width: parent.width
            height: 40 * root.s

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 14 * root.s
                anchors.verticalCenter: parent.verticalCenter
                text: "City"
                color: Theme.cream
                font.family: Theme.font
                font.pixelSize: Metrics.tTitle * root.s
                font.weight: Font.DemiBold
            }

            SettingsTextEdit {
                anchors.right: parent.right
                anchors.rightMargin: 14 * root.s
                anchors.verticalCenter: parent.verticalCenter
                s: root.s
                value: Flags.weatherCity
                placeholder: Weather.ready ? Weather.city : "auto"
                fieldWidth: 150 * root.s
                onCommitted: text => Flags.weatherCity = text
            }
        }

        Item {
            width: parent.width
            height: 40 * root.s

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 14 * root.s
                anchors.verticalCenter: parent.verticalCenter
                text: "Source"
                color: Theme.cream
                font.family: Theme.font
                font.pixelSize: Metrics.tTitle * root.s
                font.weight: Font.DemiBold
            }

            SettingsSeg {
                anchors.right: parent.right
                anchors.rightMargin: 14 * root.s
                anchors.verticalCenter: parent.verticalCenter
                s: root.s
                options: root.backendOptions
                value: Flags.weatherBackend
                onPicked: v => Flags.weatherBackend = v
            }
        }

        Text {
            visible: Weather.backendNote.length > 0
            leftPadding: 14 * root.s
            bottomPadding: 4 * root.s
            text: Weather.backendNote
            color: Theme.subtle
            font.family: Theme.font
            font.pixelSize: Metrics.tBody * root.s
        }

        Item {
            width: parent.width
            height: 40 * root.s
            visible: Flags.weatherBackend === "accuweather"

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 14 * root.s
                anchors.verticalCenter: parent.verticalCenter
                text: "API key"
                color: Theme.cream
                font.family: Theme.font
                font.pixelSize: Metrics.tTitle * root.s
                font.weight: Font.DemiBold
            }

            SettingsTextEdit {
                anchors.right: parent.right
                anchors.rightMargin: 14 * root.s
                anchors.verticalCenter: parent.verticalCenter
                s: root.s
                value: Flags.weatherKey
                placeholder: "paste key"
                fieldWidth: 220 * root.s
                onCommitted: text => Flags.weatherKey = text
            }
        }

        Rectangle {
            width: parent.width
            height: 1
            color: Theme.hair
            anchors.topMargin: 6 * root.s
        }

        // ── current ───────────────────────────────────────────────────────
        Item {
            width: parent.width
            height: 84 * root.s
            visible: Weather.ready

            GlyphIcon {
                id: wxGlyph
                anchors.left: parent.left
                anchors.leftMargin: 14 * root.s
                anchors.verticalCenter: parent.verticalCenter
                width: 40 * root.s
                height: 40 * root.s
                name: Weather.glyphFor(Weather.codeNow, Weather.isDay)
                color: Theme.cream
                stroke: 1.6
            }

            Column {
                anchors.left: wxGlyph.right
                anchors.leftMargin: 16 * root.s
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2 * root.s

                Text {
                    text: Weather.tempNow + "°"
                    color: Theme.bright
                    font.family: Theme.font
                    font.pixelSize: 30 * root.s
                    font.weight: Font.DemiBold
                    font.features: { "tnum": 1 }
                }
                Text {
                    text: Weather.labelFor(Weather.codeNow)
                        + (Weather.feelsNow !== 0 ? " · feels " + Weather.feelsNow + "°" : "")
                    color: Theme.subtle
                    font.family: Theme.font
                    font.pixelSize: 11 * root.s
                    font.weight: Font.Medium
                }
            }

            Column {
                anchors.right: parent.right
                anchors.rightMargin: 16 * root.s
                anchors.verticalCenter: parent.verticalCenter
                spacing: 3 * root.s

                Text {
                    text: (Weather.windNow > 0 ? Weather.windNow + " km/h " + root.dirFor(Weather.windDir) : "—")
                        + "  ·  " + Weather.humidity + "%"
                    color: Theme.dim
                    font.family: Theme.font
                    font.pixelSize: 10.5 * root.s
                }
                Text {
                    text: (Weather.pressureNow > 0 ? Weather.pressureNow + " hPa" : "")
                        + (Weather.sunrise.length > 0 ? "  ·  ☀ " + Weather.sunrise + " ☾ " + Weather.sunset : "")
                    color: Theme.dim
                    font.family: Theme.font
                    font.pixelSize: 10.5 * root.s
                }
            }
        }

        Rectangle {
            width: parent.width
            height: 1
            color: Theme.hair
        }

        // ── hourly strip ──────────────────────────────────────────────────
        Text {
            topPadding: 12 * root.s
            bottomPadding: 6 * root.s
            leftPadding: 14 * root.s
            text: "NEXT HOURS"
            color: Theme.faint
            font.family: Theme.font
            font.pixelSize: 9 * root.s
            font.weight: Font.Bold
            font.capitalization: Font.AllUppercase
            font.letterSpacing: 1.2 * root.s
            visible: Weather.hourly.length > 0
        }

        Row {
            leftPadding: 12 * root.s
            rightPadding: 12 * root.s
            spacing: 10 * root.s
            visible: Weather.hourly.length > 0

            Repeater {
                model: Math.min(8, Weather.hourly.length)

                delegate: Column {
                    id: hourCol
                    required property int index
                    readonly property var entry: Weather.hourly[Math.min(hourCol.index, Weather.hourly.length - 1)]
                    spacing: 4 * root.s

                    GlyphIcon {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 16 * root.s
                        height: 16 * root.s
                        name: Weather.glyphFor(hourCol.entry ? hourCol.entry.code : 3, true)
                        color: Theme.subtle
                        stroke: 1.6
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: hourCol.entry ? hourCol.entry.temp + "°" : "—"
                        color: Theme.cream
                        font.family: Theme.font
                        font.pixelSize: 11 * root.s
                        font.weight: Font.Medium
                        font.features: { "tnum": 1 }
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: hourCol.entry ? hourCol.entry.hour : ""
                        color: Theme.faint
                        font.family: Theme.font
                        font.pixelSize: 9 * root.s
                    }
                }
            }
        }

        // ── five-day forecast ─────────────────────────────────────────────
        Text {
            topPadding: 14 * root.s
            bottomPadding: 6 * root.s
            leftPadding: 14 * root.s
            text: "5 DAYS"
            color: Theme.faint
            font.family: Theme.font
            font.pixelSize: 9 * root.s
            font.weight: Font.Bold
            font.capitalization: Font.AllUppercase
            font.letterSpacing: 1.2 * root.s
            visible: Weather.daily.length > 0
        }

        Repeater {
            model: Weather.daily

            delegate: Item {
                id: dayRow
                required property var modelData
                required property int index
                width: parent ? parent.width : 0
                height: 38 * root.s

                Rectangle {
                    anchors.bottom: parent.bottom
                    anchors.left: parent.left
                    anchors.right: parent.right
                    height: 1
                    color: Theme.hairSoft
                    visible: dayRow.index < Weather.daily.length - 1
                }

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 14 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                    text: modelData.day
                    color: Theme.cream
                    font.family: Theme.font
                    font.pixelSize: 12 * root.s
                    font.weight: Font.Medium
                }

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 64 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                    text: Weather.labelFor(modelData.code)
                        + (modelData.min ? " · low " + modelData.min + "°" : "")
                    color: Theme.subtle
                    font.family: Theme.font
                    font.pixelSize: 10.5 * root.s
                }

                Text {
                    anchors.right: parent.right
                    anchors.rightMargin: 14 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                    text: modelData.temp + "°"
                    color: Theme.cream
                    font.family: Theme.font
                    font.pixelSize: 13 * root.s
                    font.weight: Font.DemiBold
                    font.features: { "tnum": 1 }
                }
            }
        }

        Text {
            width: parent.width
            topPadding: 14 * root.s
            visible: !Weather.ready
            horizontalAlignment: Text.AlignHCenter
            text: "resolving location…"
            color: Theme.faint
            font.family: Theme.font
            font.pixelSize: 10.5 * root.s
        }
    }
}
