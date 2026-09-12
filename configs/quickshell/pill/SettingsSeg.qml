pragma ComponentBehavior: Bound

import QtQuick
import "Singletons"

/**
 * Mini-segmented choice control. `options` is a list of `{ label, value }`; the
 * pill whose value equals `value` lights with a flame tint. Picking a pill emits
 * `picked(value)`; selection keys off the source value, never a child's effective
 * visibility. The host passes `s` for scale.
 */
Rectangle {
    id: seg

    property real s: 1
    property var options: []
    property var value
    signal picked(var value)

    /**
     * The option a two-step confirm is currently holding open, or `undefined`
     * when none is. It reads as vermilion rather than the flame tint of a
     * settled choice, so an armed pill is visibly a question and not an answer.
     * Only the danger segs (Session's Suspend) ever set it.
     */
    property var armedValue: undefined

    /**
     * When `flushLeft`, the control shifts left by the first option's text inset
     * so that text lines up with x=0 of where the control is placed, rather than
     * the pill edge sitting there.
     */
    property bool flushLeft: false

    readonly property real pad: 1
    readonly property real edgePad: seg.pad + 9 * seg.s

    x: seg.flushLeft ? -seg.edgePad : 0
    width: pills.implicitWidth + 2 * pad
    height: pills.implicitHeight + 2 * pad
    radius: Metrics.rCard * seg.s
    color: "transparent"

    Row {
        id: pills
        anchors.centerIn: parent
        spacing: 2 * seg.s

        Repeater {
            model: seg.options

            SegPill {
                id: opt
                required property var modelData
                option: modelData
                current: seg.value === modelData.value
                armed: seg.armedValue !== undefined && seg.armedValue === modelData.value
                s: seg.s

                onPicked: value => seg.picked(value)
            }
        }
    }
}
