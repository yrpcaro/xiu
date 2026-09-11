pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import "lib/emoji.js" as Emoji
import "Singletons"

/**
 * 絵 EMOJI surface: search field over the shared emoji table, drawn as one
 * of the pill's surfaces in the same list language as the clipboard —
 * scrolling rows of glyph + name, frameBg selection with the frameBorder
 * ring, hover moving the selection, Return or a click copying through
 * wl-copy and closing. The table is the same set the rice has always
 * shipped (mirrored into lib/emoji.js from the CLI that used to carry it).
 */
PillSurface {
    id: root

    mTop: 15
    mLeft: 17
    mRight: 17
    mBottom: 14

    property string query: ""
    property int selectedIndex: 0
    property point lastPointer: Qt.point(-1, -1)

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

    function activate() {
        if (selectedIndex >= 0 && selectedIndex < results.length)
            copyGlyph(results[selectedIndex][1]);
    }

    function move(delta) {
        if (results.length === 0)
            return;
        selectedIndex = Math.max(0, Math.min(results.length - 1, selectedIndex + delta));
        list.positionViewAtIndex(selectedIndex, ListView.Contain);
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
        onAccepted: root.activate()
        onMoved: delta => root.move(delta)
    }

    Rectangle {
        id: divider
        anchors.top: search.bottom
        anchors.topMargin: 8 * root.s
        anchors.left: parent.left
        anchors.right: parent.right
        height: 1
        color: Theme.hair
    }

    ListView {
        id: list
        anchors.top: divider.bottom
        anchors.topMargin: 6 * root.s
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        spacing: 2 * root.s
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        model: root.results.length

        delegate: Item {
            id: row
            required property int index
            width: list.width
            height: 30 * root.s

            readonly property var entry: root.results[row.index] || ["", ""]
            readonly property bool selected: row.index === root.selectedIndex

            HoverHandler {
                id: rowHover
                onPointChanged: {
                    if (!hovered)
                        return;
                    var sp = point.scenePosition;
                    if (sp.x !== root.lastPointer.x || sp.y !== root.lastPointer.y) {
                        root.lastPointer = Qt.point(sp.x, sp.y);
                        root.selectedIndex = row.index;
                    }
                }
            }

            Rectangle {
                anchors.fill: parent
                radius: 9 * root.s
                visible: row.selected || rowHover.hovered
                color: row.selected ? Theme.frameBg : Qt.rgba(0.94, 0.88, 0.84, 0.03)
                border.width: row.selected ? 1 : 0
                border.color: Theme.frameBorder
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    root.selectedIndex = row.index;
                    root.activate();
                }
            }

            Item {
                anchors.fill: parent
                anchors.leftMargin: 11 * root.s
                anchors.rightMargin: 11 * root.s

                Text {
                    id: glyph
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    width: 24 * root.s
                    text: row.entry[1]
                    font.pixelSize: 16 * root.s
                }

                Text {
                    anchors.left: glyph.right
                    anchors.leftMargin: 10 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                    text: row.entry[0]
                    color: row.selected ? Theme.cream : Theme.subtle
                    font.family: Theme.font
                    font.pixelSize: 11.5 * root.s
                    font.weight: Font.Medium
                }
            }
        }
    }

    Text {
        anchors.centerIn: list
        visible: root.results.length === 0
        text: "no emoji matches '" + root.query + "'"
        color: Theme.faint
        font.family: Theme.font
        font.pixelSize: 11 * root.s
    }
}
