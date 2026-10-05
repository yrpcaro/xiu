pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import "Singletons"

/**
 * 天気 WEATHER surface: Modern, high-density weather dashboard.
 * Designed to seamlessly match xiu's washi aesthetic, card styling,
 * typography metrics, and color tokens. Driven by keyless Open-Meteo
 * with automatic IP geolocation, optional manual city search, live
 * atmospheric vitals, hourly trend strip, and 5-day forecast.
 */
SettingsSurface {
    id: root

    backSurface: "settings"
    implicitHeight: content.implicitHeight + 14 * root.s

    rows: [
        { item: locRow, kind: "nav", surface: "settings" }
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
            showBack: true
        }

        Item { width: 1; height: 10 * root.s }

        // ── Location & Configuration Row ────────────────────────────────────
        SettingsRow {
            id: locRow
            surface: root
            name: "Location"
            icon: "language"
            sub: Weather.ready
                 ? (Flags.weatherCity.length > 0 ? ("Override: " + Weather.city) : ("Auto (IP): " + (Weather.city.length > 0 ? Weather.city : "Detected")))
                 : "Locating & resolving..."

            Row {
                spacing: 6 * root.s
                anchors.verticalCenter: parent.verticalCenter

                SettingsTextEdit {
                    s: root.s
                    value: Flags.weatherCity
                    placeholder: Weather.ready && Weather.city.length > 0 ? Weather.city : "auto (IP)"
                    fieldWidth: 124 * root.s
                    onCommitted: text => {
                        Flags.weatherCity = text;
                        Weather.refresh();
                    }
                }

                // Reset to Auto button (visible if manual city is set)
                Rectangle {
                    width: 22 * root.s
                    height: 22 * root.s
                    radius: 5 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                    color: clearHover.hovered ? Theme.threadBg : "transparent"
                    visible: Flags.weatherCity && Flags.weatherCity.length > 0

                    GlyphIcon {
                        anchors.centerIn: parent
                        width: 11 * root.s
                        height: 11 * root.s
                        name: "close"
                        color: clearHover.hovered ? Theme.cream : Theme.subtle
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
                    width: 22 * root.s
                    height: 22 * root.s
                    radius: 5 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                    color: refHover.hovered ? Theme.threadBg : "transparent"

                    GlyphIcon {
                        anchors.centerIn: parent
                        width: 12 * root.s
                        height: 12 * root.s
                        name: "undo"
                        color: refHover.hovered ? Theme.vermLit : Theme.cream
                        stroke: 1.6
                    }

                    HoverHandler { id: refHover }
                    TapHandler {
                        onTapped: Weather.refresh()
                    }
                }
            }
        }

        Item { width: 1; height: 10 * root.s }

        // ── Hero Current Conditions Card ───────────────────────────────────
        Rectangle {
            width: parent.width
            height: 96 * root.s
            radius: 12 * root.s
            gradient: Gradient {
                GradientStop { position: 0.0; color: Theme.cardTop }
                GradientStop { position: 1.0; color: Theme.cardBot }
            }
            border.width: 1
            border.color: Theme.frameBorder
            visible: Weather.ready

            Item {
                anchors.fill: parent
                anchors.margins: 14 * root.s

                Rectangle {
                    id: heroIconBox
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    width: 48 * root.s
                    height: 48 * root.s
                    radius: 12 * root.s
                    color: Qt.alpha(Theme.vermLit, 0.12)
                    border.width: 1
                    border.color: Qt.alpha(Theme.vermLit, 0.25)

                    GlyphIcon {
                        anchors.centerIn: parent
                        width: 28 * root.s
                        height: 28 * root.s
                        name: Weather.glyphFor(Weather.codeNow, Weather.isDay)
                        color: Theme.todayWarm
                        stroke: 1.8
                    }
                }

                Column {
                    anchors.left: heroIconBox.right
                    anchors.leftMargin: 14 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 2 * root.s

                    Row {
                        spacing: 8 * root.s

                        Text {
                            text: Weather.tempNow + "°"
                            color: Theme.bright
                            font.family: Theme.font
                            font.pixelSize: 32 * root.s
                            font.weight: Font.Bold
                            font.features: { "tnum": 1 }
                        }

                        Text {
                            anchors.bottom: parent.bottom
                            anchors.bottomMargin: 4 * root.s
                            text: Weather.daily && Weather.daily.length > 0
                                  ? ("H: " + Weather.daily[0].temp + "°  L: " + Weather.daily[0].min + "°")
                                  : ""
                            color: Theme.faint
                            font.family: Theme.font
                            font.pixelSize: 11 * root.s
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
                    spacing: 6 * root.s

                    Rectangle {
                        anchors.right: parent.right
                        height: 20 * root.s
                        width: cityLabel.implicitWidth + 18 * root.s
                        radius: 10 * root.s
                        color: Theme.frameBg
                        border.color: Theme.hair
                        border.width: 1

                        Row {
                            anchors.centerIn: parent
                            spacing: 4 * root.s

                            Rectangle {
                                width: 4 * root.s
                                height: 4 * root.s
                                radius: 2 * root.s
                                color: Theme.vermLit
                            }

                            Text {
                                id: cityLabel
                                text: Weather.city.length > 0 ? Weather.city : "Detected"
                                color: Theme.cream
                                font.family: Theme.font
                                font.pixelSize: 10 * root.s
                                font.weight: Font.Medium
                            }
                        }
                    }

                    Row {
                        anchors.right: parent.right
                        spacing: 8 * root.s
                        visible: Weather.sunrise.length > 0

                        Row {
                            spacing: 3 * root.s
                            GlyphIcon {
                                anchors.verticalCenter: parent.verticalCenter
                                width: 11 * root.s
                                height: 11 * root.s
                                name: "sun"
                                color: Theme.todayWarm
                                stroke: 1.6
                            }
                            Text {
                                text: Weather.sunrise
                                color: Theme.dim
                                font.family: Theme.font
                                font.pixelSize: 10 * root.s
                            }
                        }

                        Row {
                            spacing: 3 * root.s
                            GlyphIcon {
                                anchors.verticalCenter: parent.verticalCenter
                                width: 11 * root.s
                                height: 11 * root.s
                                name: "moon"
                                color: Theme.subtle
                                stroke: 1.6
                            }
                            Text {
                                text: Weather.sunset
                                color: Theme.dim
                                font.family: Theme.font
                                font.pixelSize: 10 * root.s
                            }
                        }
                    }
                }
            }
        }

        // ── Atmospheric Metrics Badges ─────────────────────────────────────
        SettingsGroupLabel {
            s: root.s
            text: "Atmosphere"
            topPadding: 10 * root.s
            bottomPadding: 6 * root.s
            visible: Weather.ready
        }

        Grid {
            width: parent.width
            columns: 3
            spacing: 8 * root.s
            visible: Weather.ready

            // Wind
            Rectangle {
                width: (parent.width - 16 * root.s) / 3
                height: 52 * root.s
                radius: 10 * root.s
                color: Theme.frameBg
                border.color: Theme.hair
                border.width: 1

                Column {
                    anchors.centerIn: parent
                    spacing: 3 * root.s

                    Row {
                        anchors.horizontalCenter: parent.horizontalCenter
                        spacing: 4 * root.s
                        GlyphIcon {
                            anchors.verticalCenter: parent.verticalCenter
                            width: 10 * root.s
                            height: 10 * root.s
                            name: "waves"
                            color: Theme.faint
                            stroke: 1.5
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "WIND"
                            color: Theme.faint
                            font.family: Theme.font
                            font.pixelSize: 8.5 * root.s
                            font.weight: Font.Bold
                            font.letterSpacing: 0.8 * root.s
                        }
                    }

                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: (Weather.windNow > 0 ? Weather.windNow + " km/h " + root.dirFor(Weather.windDir) : "Calm")
                        color: Theme.cream
                        font.family: Theme.font
                        font.pixelSize: 11 * root.s
                        font.weight: Font.DemiBold
                    }
                }
            }

            // Humidity
            Rectangle {
                width: (parent.width - 16 * root.s) / 3
                height: 52 * root.s
                radius: 10 * root.s
                color: Theme.frameBg
                border.color: Theme.hair
                border.width: 1

                Column {
                    anchors.centerIn: parent
                    spacing: 3 * root.s

                    Row {
                        anchors.horizontalCenter: parent.horizontalCenter
                        spacing: 4 * root.s
                        GlyphIcon {
                            anchors.verticalCenter: parent.verticalCenter
                            width: 10 * root.s
                            height: 10 * root.s
                            name: "droplet"
                            color: Theme.faint
                            stroke: 1.5
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "HUMIDITY"
                            color: Theme.faint
                            font.family: Theme.font
                            font.pixelSize: 8.5 * root.s
                            font.weight: Font.Bold
                            font.letterSpacing: 0.8 * root.s
                        }
                    }

                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: Weather.humidity + "%"
                        color: Theme.cream
                        font.family: Theme.font
                        font.pixelSize: 11 * root.s
                        font.weight: Font.DemiBold
                    }
                }
            }

            // Pressure & UV
            Rectangle {
                width: (parent.width - 16 * root.s) / 3
                height: 52 * root.s
                radius: 10 * root.s
                color: Theme.frameBg
                border.color: Theme.hair
                border.width: 1

                Column {
                    anchors.centerIn: parent
                    spacing: 3 * root.s

                    Row {
                        anchors.horizontalCenter: parent.horizontalCenter
                        spacing: 4 * root.s
                        GlyphIcon {
                            anchors.verticalCenter: parent.verticalCenter
                            width: 10 * root.s
                            height: 10 * root.s
                            name: "sparkles"
                            color: Theme.faint
                            stroke: 1.5
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: Weather.uvNow > 0 ? "UV / PRESSURE" : "PRESSURE"
                            color: Theme.faint
                            font.family: Theme.font
                            font.pixelSize: 8.5 * root.s
                            font.weight: Font.Bold
                            font.letterSpacing: 0.8 * root.s
                        }
                    }

                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: Weather.uvNow > 0
                              ? ("UV " + Weather.uvNow + " · " + Weather.pressureNow + "hPa")
                              : (Weather.pressureNow > 0 ? Weather.pressureNow + " hPa" : "Normal")
                        color: Theme.cream
                        font.family: Theme.font
                        font.pixelSize: 11 * root.s
                        font.weight: Font.DemiBold
                    }
                }
            }
        }

        // ── Hourly Forecast Strip ──────────────────────────────────────────
        SettingsGroupLabel {
            s: root.s
            text: "Hourly Forecast"
            topPadding: 12 * root.s
            bottomPadding: 6 * root.s
            visible: Weather.hourly && Weather.hourly.length > 0
        }

        Rectangle {
            width: parent.width
            height: 74 * root.s
            radius: 12 * root.s
            gradient: Gradient {
                GradientStop { position: 0.0; color: Theme.cardTop }
                GradientStop { position: 1.0; color: Theme.cardBot }
            }
            border.color: Theme.frameBorder
            border.width: 1
            visible: Weather.hourly && Weather.hourly.length > 0

            Row {
                anchors.centerIn: parent
                spacing: Math.max(4 * root.s, (parent.width - (8 * 36 * root.s)) / 9)

                Repeater {
                    model: Math.min(8, Weather.hourly.length)

                    delegate: Item {
                        id: hourItem
                        required property int index
                        readonly property var entry: Weather.hourly[Math.min(hourItem.index, Weather.hourly.length - 1)]
                        width: 36 * root.s
                        height: 64 * root.s

                        Rectangle {
                            anchors.fill: parent
                            radius: 8 * root.s
                            color: hourHover.hovered ? Qt.alpha(Theme.cream, 0.05) : "transparent"
                        }
                        HoverHandler { id: hourHover }

                        Column {
                            anchors.centerIn: parent
                            spacing: 4 * root.s

                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: hourItem.entry ? hourItem.entry.hour + "h" : ""
                                color: Theme.faint
                                font.family: Theme.font
                                font.pixelSize: 9.5 * root.s
                                font.weight: Font.Medium
                            }

                            GlyphIcon {
                                anchors.horizontalCenter: parent.horizontalCenter
                                width: 16 * root.s
                                height: 16 * root.s
                                name: Weather.glyphFor(hourItem.entry ? hourItem.entry.code : 0, true)
                                color: Theme.todayWarm
                                stroke: 1.6
                            }

                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: hourItem.entry ? hourItem.entry.temp + "°" : "—"
                                color: Theme.cream
                                font.family: Theme.font
                                font.pixelSize: 11.5 * root.s
                                font.weight: Font.DemiBold
                                font.features: { "tnum": 1 }
                            }
                        }
                    }
                }
            }
        }

        // ── 5-Day Forecast ────────────────────────────────────────────────
        SettingsGroupLabel {
            s: root.s
            text: "5-Day Forecast"
            topPadding: 12 * root.s
            bottomPadding: 6 * root.s
            visible: Weather.daily && Weather.daily.length > 0
        }

        Rectangle {
            width: parent.width
            height: (Weather.daily.length * 36 * root.s) + 6 * root.s
            radius: 12 * root.s
            gradient: Gradient {
                GradientStop { position: 0.0; color: Theme.cardTop }
                GradientStop { position: 1.0; color: Theme.cardBot }
            }
            border.color: Theme.frameBorder
            border.width: 1
            visible: Weather.daily && Weather.daily.length > 0

            Column {
                anchors.fill: parent
                anchors.topMargin: 3 * root.s
                anchors.bottomMargin: 3 * root.s
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
                            anchors.fill: parent
                            anchors.leftMargin: 6 * root.s
                            anchors.rightMargin: 6 * root.s
                            radius: 8 * root.s
                            color: dayHover.hovered ? Qt.alpha(Theme.cream, 0.04) : "transparent"
                        }
                        HoverHandler { id: dayHover }

                        Rectangle {
                            anchors.bottom: parent.bottom
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.leftMargin: 12 * root.s
                            anchors.rightMargin: 12 * root.s
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
                                width: 36 * root.s
                                text: dayRow.index === 0 ? "Today" : dayRow.modelData.day
                                color: dayRow.index === 0 ? Theme.vermLit : Theme.cream
                                font.family: Theme.font
                                font.pixelSize: 11.5 * root.s
                                font.weight: dayRow.index === 0 ? Font.Bold : Font.Medium
                            }

                            GlyphIcon {
                                anchors.verticalCenter: parent.verticalCenter
                                width: 16 * root.s
                                height: 16 * root.s
                                name: Weather.glyphFor(dayRow.modelData.code, true)
                                color: Theme.todayWarm
                                stroke: 1.6
                            }

                            Text {
                                text: Weather.labelFor(dayRow.modelData.code)
                                color: Theme.subtle
                                font.family: Theme.font
                                font.pixelSize: 11 * root.s
                            }
                        }

                        Row {
                            anchors.right: parent.right
                            anchors.rightMargin: 12 * root.s
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 10 * root.s

                            Text {
                                width: 26 * root.s
                                horizontalAlignment: Text.AlignRight
                                text: dayRow.modelData.min ? dayRow.modelData.min + "°" : ""
                                color: Theme.faint
                                font.family: Theme.font
                                font.pixelSize: 11.5 * root.s
                                font.features: { "tnum": 1 }
                            }

                            // Temperature spread bar
                            Rectangle {
                                anchors.verticalCenter: parent.verticalCenter
                                width: 48 * root.s
                                height: 3.5 * root.s
                                radius: 2 * root.s
                                color: Theme.threadBg

                                Rectangle {
                                    anchors.verticalCenter: parent.verticalCenter
                                    anchors.left: parent.left
                                    anchors.leftMargin: 6 * root.s
                                    width: 32 * root.s
                                    height: parent.height
                                    radius: parent.radius
                                    gradient: Gradient {
                                        orientation: Gradient.Horizontal
                                        GradientStop { position: 0.0; color: Theme.vermDim }
                                        GradientStop { position: 1.0; color: Theme.vermLit }
                                    }
                                }
                            }

                            Text {
                                width: 26 * root.s
                                text: dayRow.modelData.temp + "°"
                                color: Theme.bright
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

        // ── Loading / Empty State ──────────────────────────────────────────
        Item {
            width: parent.width
            height: 120 * root.s
            visible: !Weather.ready

            Column {
                anchors.centerIn: parent
                spacing: 8 * root.s

                GlyphIcon {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 28 * root.s
                    height: 28 * root.s
                    name: "cloud"
                    color: Theme.dim
                    stroke: 1.6
                }

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "Locating & fetching forecast…"
                    color: Theme.faint
                    font.family: Theme.font
                    font.pixelSize: 11 * root.s
                    font.weight: Font.Medium
                }
            }
        }
    }
}
