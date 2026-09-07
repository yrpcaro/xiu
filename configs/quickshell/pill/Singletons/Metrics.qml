pragma Singleton

import QtQuick
import Quickshell

/**
 * The settings family's design tokens, ported from CapsuleOS's Metrics
 * singleton — but scoped to just the settings family: the rest of the pill
 * keeps its inline literals, so this is a shared vocabulary for the new
 * surfaces, not a pill-wide refactor. Corner radii in px at s=1, the type
 * scale in pt at s=1, hairW as a function because a hairline stays one
 * physical pixel whatever the scale.
 */
Singleton {
    readonly property real rCard:      9
    readonly property real rTile:      13
    readonly property real tCaption:   9
    readonly property real tBody:      10.5
    readonly property real tLabel:     11.5
    readonly property real tTitle:     13
    readonly property real iconRow:    16
    readonly property real iconStroke: 1.8

    function hairW(s) { return Math.max(1, Math.round(1 * s)) }
}
