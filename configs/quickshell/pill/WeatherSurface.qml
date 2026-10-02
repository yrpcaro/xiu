pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import "Singletons"

/**
 * 天気 WEATHER surface: Modern, high-density weather dashboard.
 * Driven entirely by keyless Open-Meteo with automatic IP geolocation
 * and optional manual city search. Features hero current conditions card,
 * live atmospheric metrics grid, 24-hour hourly trend, and 5-day forecast.
 */
SettingsSurface {
    id: root

    backSurface: "settings"
    implicitHeight: content.implicitHeight + 20 * root.s

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
        spacing: 12 * root.s

        SettingsHeader {
            s: root.s
            glyph: "天"
            title: "WEATHER"
            showBack: true
        }

        // ── Location & Search Bar ───────────────────────────────────────────
        Rectangle {
            width: parent.width - 24 * root.s
            height: 38 * root.s
            anchors.horizontalCenter: parent.horizontalCenter
            radius: 8 * root.s
            color: Theme.tileBg
            border.color: Theme.hair
            border.width: 1

            Row {
                anchors.left: parent.left
                anchors.leftMargin: 10 * root.s
                anchors.verticalCenter: parent.verticalCenter
                spacing: 8 * root.s

                GlyphIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 15 * root.s
                    height: 15 * root.s
                    name: "language"
                    color: Theme.iconDim
                    stroke: 1.6
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Location"
                    color: Theme.cream
                    font.family: Theme.font
                    font.pixelSize: Metrics.tTitle * root.s
                    font.weight: Font.DemiBold
                }
            }

            Row {
                anchors.right: parent.right
                anchors.rightMargin: 8 * root.s
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6 * root.s

                SettingsTextEdit {
                    anchors.verticalCenter: parent.verticalCenter
                    s: root.s
                    value: Flags.weatherCity
                    placeholder: Weather.ready ? Weather.city : "auto (IP)"
                    fieldWidth: 140 * root.s
                    onCommitted: text => {
                        Flags.weatherCity = text;
                        Weather.refresh();
                    }
                }

                // Reset to Auto button (visible if manual city is set)
                Rectangle {
                    width: 24 * root.s
                    height: 24 * root.s
                    radius: 5 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                    color: clearHover.hovered ? Theme.threadBg : "transparent"
                    visible: Flags.weatherCity && Flags.weatherCity.length > 0

                    GlyphIcon {
                        anchors.centerIn: parent
                        width: 12 * root.s
                        height: 12 * root.s
                        name: "close"
                        color: Theme.subtle
                        stroke: 1.8
                    }

                    HoverHandler { id: clearHover }
                    TapHandler {
                        onTapped: {
                            Flags.weatherCity = "";
                            Weather.refresh();
                        }
                    }
                }

                // Refresh button
                Rectangle {
                    width: 24 * root.s
                    height: 24 * root.s
                    radius: 5 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                    color: refHover.hovered ? Theme.threadBg : "transparent"

                    GlyphIcon {
                        anchors.centerIn: parent
                        width: 13 * root.s
                        height: 13 * root.s
                        name: "undo"
                        color: Theme.cream
                        stroke: 1.6
                    }

                    HoverHandler { id: refHover }
                    TapHandler {
                        onTapped: Weather.refresh()
                    }
                }
            }
        }

        // ── Hero Current Weather Card ──────────────────────────────────────
        Rectangle {
            width: parent.width - 24 * root.s
            height: 104 * root.s
            anchors.horizontalCenter: parent.horizontalCenter
            radius: 12 * root.s
            color: Theme.tileBg
            border.color: Theme.hair
            border.width: 1
            visible: Weather.ready

            Item {
                anchors.fill: parent
                anchors.margins: 14 * root.s

                GlyphIcon {
                    id: heroGlyph
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    width: 48 * root.s
                    height: 48 * root.s
                    name: Weather.glyphFor(Weather.codeNow, Weather.isDay)
                    color: Theme.vermLit
                    stroke: 1.8
                }

                Column {
                    anchors.left: heroGlyph.right
                    anchors.leftMargin: 16 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 2 * root.s

                    Row {
                        spacing: 8 * root.s
                        Text {
                            text: Weather.tempNow + "°"
                            color: Theme.bright
                            font.family: Theme.font
                            font.pixelSize: 34 * root.s
                            font.weight: Font.Bold
                            font.features: { "tnum": 1 }
                        }
                        Text {
                            anchors.bottom: parent.bottom
                            anchors.bottomMargin: 5 * root.s
                            text: Weather.daily && Weather.daily.length > 0
                                  ? ("H: " + Weather.daily[0].temp + "°  L: " + Weather.daily[0].min + "°")
                                  : ""
                            color: Theme.faint
                            font.family: Theme.font
                            font.pixelSize: 10.5 * root.s
                            font.weight: Font.Medium
                        }
                    }

                    Text {
                        text: Weather.labelFor(Weather.codeNow) + (Weather.feelsNow !== 0 ? " · Feels " + Weather.feelsNow + "°" : "")
                        color: Theme.cream
                        font.family: Theme.font
                        font.pixelSize: 12 * root.s
                        font.weight: Font.Medium
                    }
                }

                Column {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 4 * root.s
                    horizontalAlignment: Text.AlignRight

                    Rectangle {
                        anchors.right: parent.right
                        height: 20 * root.s
                        width: cityLabel.implicitWidth + 14 * root.s
                        radius: 10 * root.s
                        color: Theme.threadBg
                        border.color: Theme.hair
                        border.width: 1

                        Text {
                            id: cityLabel
                            anchors.centerIn: parent
                            text: Weather.city.length > 0 ? Weather.city : "Detected"
                            color: Theme.subtle
                            font.family: Theme.font
                            font.pixelSize: 10 * root.s
                            font.weight: Font.Medium
                        }
                    }

                    Text {
                        anchors.right: parent.right
                        text: Weather.sunrise.length > 0 ? "☀ " + Weather.sunrise + "  ☾ " + Weather.sunset : ""
                        color: Theme.dim
                        font.family: Theme.font
                        font.pixelSize: 10 * root.s
                        font.weight: Font.Normal
                    }
                }
            }
        }

        // ── Atmospheric Metrics Badges ─────────────────────────────────────
        Grid {
            width: parent.width - 24 * root.s
            anchors.horizontalCenter: parent.horizontalCenter
            columns: 3
            spacing: 8 * root.s
            visible: Weather.ready

            // Wind
            Rectangle {
                width: (parent.width - 16 * root.s) / 3
                height: 52 * root.s
                radius: 8 * root.s
                color: Theme.tileBg
                border.color: Theme.hair
                border.width: 1

                Column {
                    anchors.centerIn: parent
                    spacing: 2 * root.s
                    horizontalAlignment: Text.AlignHCenter

                    Text {
                        text: "WIND"
                        color: Theme.faint
                        font.family: Theme.font
                        font.pixelSize: 8.5 * root.s
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8 * root.s
                    }
                    Text {
                        text: (Weather.windNow > 0 ? Weather.windNow + " km/h " + root.dirFor(Weather.windDir) : "Calm")
                        color: Theme.cream
                        font.family: Theme.font
                        font.pixelSize: 11 * root.s
                        font.weight: Font.Medium
                    }
                }
            }

            // Humidity
            Rectangle {
                width: (parent.width - 16 * root.s) / 3
                height: 52 * root.s
                radius: 8 * root.s
                color: Theme.tileBg
                border.color: Theme.hair
                border.width: 1

                Column {
                    anchors.centerIn: parent
                    spacing: 2 * root.s
                    horizontalAlignment: Text.AlignHCenter

                    Text {
                        text: "HUMIDITY"
                        color: Theme.faint
                        font.family: Theme.font
                        font.pixelSize: 8.5 * root.s
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8 * root.s
                    }
                    Text {
                        text: Weather.humidity + "%"
                        color: Theme.cream
                        font.family: Theme.font
                        font.pixelSize: 11 * root.s
                        font.weight: Font.Medium
                    }
                }
            }

            // Pressure & UV
            Rectangle {
                width: (parent.width - 16 * root.s) / 3
                height: 52 * root.s
                radius: 8 * root.s
                color: Theme.tileBg
                border.color: Theme.hair
                border.width: 1

                Column {
                    anchors.centerIn: parent
                    spacing: 2 * root.s
                    horizontalAlignment: Text.AlignHCenter

                    Text {
                        text: Weather.uvNow > 0 ? "UV / PRESSURE" : "PRESSURE"
                        color: Theme.faint
                        font.family: Theme.font
                        font.pixelSize: 8.5 * root.s
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8 * root.s
                    }
                    Text {
                        text: Weather.uvNow > 0
                              ? ("UV " + Weather.uvNow + " · " + Weather.pressureNow + "hPa")
                              : (Weather.pressureNow > 0 ? Weather.pressureNow + " hPa" : "Normal")
                        color: Theme.cream
                        font.family: Theme.font
                        font.pixelSize: 11 * root.s
                        font.weight: Font.Medium
                    }
                }
            }
        }

        // ── Hourly Forecast Strip ──────────────────────────────────────────
        Column {
            width: parent.width - 24 * root.s
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 6 * root.s
            visible: Weather.hourly && Weather.hourly.length > 0

            Text {
                text: "HOURLY FORECAST"
                color: Theme.faint
                font.family: Theme.font
                font.pixelSize: 9 * root.s
                font.weight: Font.Bold
                font.letterSpacing: 1.2 * root.s
            }

            Rectangle {
                width: parent.width
                height: 72 * root.s
                radius: 10 * root.s
                color: Theme.tileBg
                border.color: Theme.hair
                border.width: 1

                Row {
                    anchors.centerIn: parent
                    spacing: Math.max(4 * root.s, (parent.width - (8 * 36 * root.s)) / 9)

                    Repeater {
                        model: Math.min(8, Weather.hourly.length)

                        delegate: Column {
                            id: hourItem
                            required property int index
                            readonly property var entry: Weather.hourly[Math.min(hourItem.index, Weather.hourly.length - 1)]
                            spacing: 4 * root.s
                            width: 34 * root.s
                            horizontalAlignment: Text.AlignHCenter

                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: hourItem.entry ? hourItem.entry.hour + "h" : ""
                                color: Theme.faint
                                font.family: Theme.font
                                font.pixelSize: 9 * root.s
                                font.weight: Font.Medium
                            }

                            GlyphIcon {
                                anchors.horizontalCenter: parent.horizontalCenter
                                width: 16 * root.s
                                height: 16 * root.s
                                name: Weather.glyphFor(hourItem.entry ? hourItem.entry.code : 0, true)
                                color: Theme.subtle
                                stroke: 1.5
                            }

                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: hourItem.entry ? hourItem.entry.temp + "°" : "—"
                                color: Theme.cream
                                font.family: Theme.font
                                font.pixelSize: 11 * root.s
                                font.weight: Font.DemiBold
                                font.features: { "tnum": 1 }
                            }
                        }
                    }
                }
            }
        }

        // ── 5-Day Forecast ────────────────────────────────────────────────
        Column {
            width: parent.width - 24 * root.s
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 6 * root.s
            visible: Weather.daily && Weather.daily.length > 0

            Text {
                text: "5-DAY FORECAST"
                color: Theme.faint
                font.family: Theme.font
                font.pixelSize: 9 * root.s
                font.weight: Font.Bold
                font.letterSpacing: 1.2 * root.s
            }

            Rectangle {
                width: parent.width
                height: (Weather.daily.length * 36 * root.s) + 4 * root.s
                radius: 10 * root.s
                color: Theme.tileBg
                border.color: Theme.hair
                border.width: 1

                Column {
                    anchors.fill: parent
                    anchors.topMargin: 2 * root.s
                    anchors.bottomMargin: 2 * root.s
                    spacing: 0

                    Repeater {
                        model: Weather.daily

                        delegate: Item {
                            id: dayRow
                            required property var modelData
                            required property int index
                            width: parent ? parent.width : 0
                            height: 36 * root.s

                            Rectangle {
                                anchors.bottom: parent.bottom
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.margins: 10 * root.s
                                height: 1
                                color: Theme.hairSoft
                                visible: dayRow.index < Weather.daily.length - 1
                            }

                            Row {
                                anchors.left: parent.left
                                anchors.leftMargin: 12 * root.s
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 10 * root.s

                                Text {
                                    width: 32 * root.s
                                    text: dayRow.modelData.day
                                    color: dayRow.index === 0 ? Theme.vermLit : Theme.cream
                                    font.family: Theme.font
                                    font.pixelSize: 11.5 * root.s
                                    font.weight: dayRow.index === 0 ? Font.Bold : Font.Medium
                                }

                                GlyphIcon {
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: 15 * root.s
                                    height: 15 * root.s
                                    name: Weather.glyphFor(dayRow.modelData.code, true)
                                    color: Theme.subtle
                                    stroke: 1.5
                                }

                                Text {
                                    text: Weather.labelFor(dayRow.modelData.code)
                                    color: Theme.subtle
                                    font.family: Theme.font
                                    font.pixelSize: 10.5 * root.s
                                }
                            }

                            Row {
                                anchors.right: parent.right
                                anchors.rightMargin: 12 * root.s
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 10 * root.s

                                Text {
                                    text: dayRow.modelData.min ? dayRow.modelData.min + "°" : ""
                                    color: Theme.faint
                                    font.family: Theme.font
                                    font.pixelSize: 11 * root.s
                                    font.features: { "tnum": 1 }
                                }

                                Text {
                                    text: dayRow.modelData.temp + "°"
                                    color: Theme.cream
                                    font.family: Theme.font
                                    font.pixelSize: 12 * root.s
                                    font.weight: Font.DemiBold
                                    font.features: { "tnum": 1 }
                                }
                            }
                        }
                    }
                }
            }
        }

        Text {
            width: parent.width
            topPadding: 14 * root.s
            visible: !Weather.ready
            horizontalAlignment: Text.AlignHCenter
            text: "Locating & fetching forecast…"
            color: Theme.faint
            font.family: Theme.font
            font.pixelSize: 10.5 * root.s
        }
    }
}
