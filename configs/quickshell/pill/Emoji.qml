pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import "lib/emoji.js" as Emoji
import "Singletons"

/**
 * 絵 EMOJI surface: search field over the shared emoji table, drawn as one
 * of the pill's surfaces. Typing filters by substring, Return or a click
 * copies the glyph to the Wayland clipboard and closes the surface, arrow
 * keys walk the grid. The table is the same set the `xiu emoji` CLI serves
 * (cli/src/commands/emoji.rs, mirrored into lib/emoji.js) — the CLI stays
 * for scripts and keybinds, the surface is the interactive face.
 */
PillSurface {
    id: root

    mTop: 15
    mLeft: 17
    mRight: 17
    mBottom: 14

    property string query: ""
    property int selectedIndex: 0
    readonly property int columns: 7
    /** The filtered grid rows: [ { name, text } ... ] from the shared table. */
    readonly property var results: {
        var q = query.trim().toLowerCase();
        if (q.length === 0)
            return Emoji.EMOJI;
        return Emoji.matches(q);
    }

    function copyGlyph(text) {
        Quickshell.execDetached(["sh", "-c", "printf '%s' \"$1\" | wl-copy", "_", text]);
        root.requestClose();
    }

    function move(delta) {
        if (results.length === 0)
            return;
        selectedIndex = Math.max(0, Math.min(results.length - 1, selectedIndex + delta));
    }

    SearchField {
        id: search
        z: 5
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        s: root.s
        kanji: "絵"
        placeholder: "Search emoji"
        counterText: root.results.length + " / " + Emoji.EMOJI.length
        onTextChanged: {
            root.query = text;
            root.selectedIndex = 0;
        }
        onAccepted: if (root.selectedIndex >= 0 && root.selectedIndex < root.results.length)
            root.copyGlyph(root.results[root.selectedIndex].text ?? root.results[root.selectedIndex][1])
        onMoved: delta => root.move(delta)
    }

    Grid {
        id: grid
        anchors.top: search.bottom
        anchors.topMargin: 10 * root.s
        anchors.left: parent.left
        anchors.right: parent.right
        columns: root.columns
        rowSpacing: 4 * root.s
        columnSpacing: 4 * root.s

        Repeater {
            model: root.results

            delegate: Item {
                id: cell
                required property var modelData
                required property int index

                width: Math.floor(grid.width / root.columns) - 4 * root.s
                height: 34 * root.s

                Rectangle {
                    anchors.fill: parent
                    radius: 8 * root.s
                    color: area.containsMouse || root.selectedIndex === cell.index ? Theme.frameBg : "transparent"
                    border.width: root.selectedIndex === cell.index ? 1 : 0
                    border.color: Theme.frameBorder
                    Behavior on color { ColorAnimation { duration: Motion.fast } }
                }

                Text {
                    anchors.centerIn: parent
                    text: cell.modelData[1]
                    font.pixelSize: 17 * root.s
                    // glyph width never moves: the cell is fixed, the text centers
                }

                MouseArea {
                    id: area
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.selectedIndex = cell.index;
                        root.copyGlyph(cell.modelData[1]);
                    }
                }
            }
        }
    }

    Text {
        anchors.top: search.bottom
        anchors.topMargin: 18 * root.s
        width: parent.width
        visible: root.results.length === 0
        horizontalAlignment: Text.AlignHCenter
        text: "no emoji matches '" + root.query + "'"
        color: Theme.faint
        font.family: Theme.font
        font.pixelSize: 11 * root.s
    }
}
