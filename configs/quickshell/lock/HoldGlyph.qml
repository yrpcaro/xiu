pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io

/**
 * A glyph button for the lock's power corner. Instant mode fires on a plain
 * click (sleep); hold mode arms with a press-and-hold measured by the fill
 * growing under the glyph and fires only when it arrives — an early release
 * eases the fill back and nothing happens, so a stray tap on the lock screen
 * can never restart or power off the session. The same contract as the pill's
 * power surface, drawn at lock scale.
 */
Item {
    id: btn

    property string glyph: ""
    property var argv: []
    property real s: 1
    /** 0 = fire on click; anything else is the hold duration in ms. */
    property real holdMs: 1150
    signal fired

    width: 24 * s
    height: 30 * s

    /** The hold progress, 0..1; the underline fill and the glyph tint track it. */
    property real holdP: 0

    function fire() {
        proc.running = true;
        btn.fired();
    }

    Process {
        id: proc
        command: btn.argv
    }

    Column {
        anchors.centerIn: parent
        spacing: 4 * s

        GlyphIcon {
            anchors.horizontalCenter: parent.horizontalCenter
            width: 20 * s
            height: 20 * s
            name: btn.glyph
            color: area.pressed || btn.holdP > 0 ? Theme.bright : (area.containsMouse ? Theme.cream : Theme.dim)
            stroke: 1.7
        }

        Item {
            anchors.horizontalCenter: parent.horizontalCenter
            width: 20 * s
            height: 2 * s

            Rectangle {
                anchors.fill: parent
                radius: height / 2
                color: Theme.trackBg
            }
            Rectangle {
                width: parent.width * btn.holdP
                height: parent.height
                radius: height / 2
                color: Theme.verm
            }
        }
    }

    NumberAnimation {
        id: holdAnim
        target: btn
        property: "holdP"
        to: 1
        duration: Math.max(1, btn.holdMs)
        easing.type: Easing.InOutQuad
        onFinished: btn.fire()
    }

    NumberAnimation {
        id: retreatAnim
        target: btn
        property: "holdP"
        to: 0
        duration: 220
        easing.type: Easing.OutCubic
    }

    MouseArea {
        id: area
        anchors.fill: parent
        anchors.margins: -8 * s
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onPressed: {
            if (btn.holdMs <= 0)
                return;
            retreatAnim.stop();
            holdAnim.restart();
        }
        onReleased: {
            if (btn.holdMs <= 0)
                return;
            if (holdAnim.running) {
                holdAnim.stop();
                retreatAnim.restart();
            }
        }
        onClicked: if (btn.holdMs <= 0)
            btn.fire()
    }
}
