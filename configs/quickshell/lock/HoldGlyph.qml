pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Widgets

/**
 * A power tile for the lock's corner: the pill's Power surface drawn at lock
 * scale. Same tile language — resting transparent with the capsule's quiet
 * stroke, hover lighting the fill and the glyph (vermilion on the destructive
 * pair, cream on the safe one), the hairline between the groups living in
 * the parent row — and the pill's hold contract: instant mode fires on a
 * plain click (sleep), hold mode arms with a press-and-hold measured by the
 * bottom-up heat fill under the glyph and fires only when it arrives. An
 * early release eases the fill back down and nothing happens, so a stray tap
 * on the lock screen can never restart or power off the session. Hover
 * reports the tile's label upward so the corner can name the hovered tile
 * without shifting anything.
 */
Item {
    id: btn

    property string glyph: ""
    property var argv: []
    property real s: 1
    /** 0 = fire on click; anything else is the hold duration in ms. */
    property real holdMs: 1150
    /** True for the destructive pair: hover, hold and the label speak vermilion. */
    property bool confirm: false
    /** The name the corner's floating label shows while this tile is hovered. */
    property string label: ""
    signal fired
    signal hoverEnter
    signal hoverExit

    width: 34 * s
    height: width

    /** The hold progress, 0..1; the heat fill and the glyph tint track it. */
    property real holdP: 0

    readonly property bool lit: area.containsMouse || btn.holdP > 0
    readonly property color accent: btn.confirm ? Theme.vermLit : Theme.cream

    function fire() {
        proc.running = true;
        btn.fired();
    }

    Process {
        id: proc
        command: btn.argv
    }

    Rectangle {
        anchors.fill: parent
        radius: 10 * s
        color: btn.lit ? Theme.frameBg : "transparent"
        border.width: 1
        /** Resting is the capsule's quiet stroke; a hover or an armed hold lights it. */
        border.color: btn.lit ? Theme.frameBorder : Theme.fieldBorder
        Behavior on color { ColorAnimation { duration: 200; easing.type: Easing.InOutQuad } }
        Behavior on border.color { ColorAnimation { duration: 200; easing.type: Easing.InOutQuad } }
    }

    /**
     * Heat fill lives in a ClippingRectangle carrying the tile's corner
     * radius: a plain Rectangle with its own radius gets it clamped to
     * height/2 while the fill is still flat, so corners poke outside the
     * tile outline on the first beat of every hold.
     */
    ClippingRectangle {
        anchors.fill: parent
        anchors.margins: 1
        radius: 9 * s
        color: "transparent"

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: parent.height * btn.holdP
            visible: btn.holdP > 0.001
            gradient: Gradient {
                GradientStop { position: 0.0; color: Qt.alpha(Theme.verm, 0.7) }
                GradientStop { position: 1.0; color: Qt.alpha(Theme.vermLit, 0.15) }
            }
        }
    }

    GlyphIcon {
        anchors.centerIn: parent
        width: 16 * s
        height: 16 * s
        name: btn.glyph
        color: btn.holdP > 0 ? Theme.flameCore
            : (btn.lit ? btn.accent : Theme.iconDim)
        stroke: 1.7
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
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onEntered: btn.hoverEnter()
        onExited: btn.hoverExit()
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
