pragma ComponentBehavior: Bound

import QtQuick
import "Singletons"

/**
 * One segmented-choice pill, shared by SettingsSeg's single-line control and
 * the Default apps page's wrapping flows — the same rounded tile everywhere:
 * the settled choice glows, an armed two-step confirm reads vermilion, hover
 * fills the frame, and a tap emits picked.
 */
Rectangle {
    id: pill

    property var option: ({ label: "", value: "" })
    property bool current: false
    property bool armed: false
    property real s: 1
    signal picked(var value)

    width: label.implicitWidth + 18 * s
    height: label.implicitHeight + 12 * s
    radius: Metrics.rCard * s
    color: armed ? Qt.alpha(Theme.verm, 0.42)
        : (current ? Qt.alpha(Theme.onGlow, 0.16) : (area.containsMouse ? Theme.frameBg : "transparent"))
    border.width: armed ? Metrics.hairW(s) : 0
    border.color: Qt.alpha(Theme.vermLit, 0.7)
    Behavior on color { ColorAnimation { duration: Motion.fast } }

    Text {
        id: label
        anchors.centerIn: parent
        text: pill.option.label ?? ""
        color: (current || armed) ? Theme.cream : Theme.subtle
        font.family: Theme.font
        font.pixelSize: Metrics.tBody * s
        font.weight: Font.Bold
        font.letterSpacing: 0.3 * s
    }

    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: pill.picked(pill.option.value)
    }
}
