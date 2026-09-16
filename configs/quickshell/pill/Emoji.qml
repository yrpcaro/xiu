pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import "lib/emoji.js" as Emoji
import "Singletons"

/**
 * 絵 EMOJI surface: comprehensive Unicode emoji picker organized into
 * 9 standard categories with instant fuzzy search, category tabs, a responsive
 * grid of emoji tiles with smooth Ame glow and hover frames, and a live telemetry
 * preview footer displaying the glyph, canonical name, and codepoint.
 */
PillSurface {
    id: root

    mTop: 14
    mLeft: 16
    mRight: 16
    mBottom: 12

    implicitHeight: 440 * root.s

    property string query: ""
    property string activeCategory: "all"
    property int selectedIndex: 0
    property var hoveredItem: null

    readonly property var results: Emoji.matches(root.query, root.activeCategory)

    readonly property var currentEmoji: {
        if (hoveredItem !== null)
            return hoveredItem;
        if (results.length > 0 && selectedIndex >= 0 && selectedIndex < results.length)
            return results[selectedIndex];
        return null;
    }

    ameForm: (root.currentEmoji !== null && root.open) ? "soul" : "off"
    amePoint: {
        if (root.selectedIndex < 0 || results.length === 0 || grid.width <= 0)
            return Qt.point(grid.x + grid.width / 2, grid.y + grid.height / 2);
        var cols = Math.max(1, Math.floor(grid.width / grid.cellWidth));
        var col = root.selectedIndex % cols;
        var row = Math.floor(root.selectedIndex / cols);
        var x = grid.x + (col + 0.5) * grid.cellWidth;
        var y = grid.y + (row + 0.5) * grid.cellHeight - grid.contentY;
        return Qt.point(x, Math.max(grid.y, Math.min(grid.y + grid.height, y)));
    }

    function copyGlyph(text) {
        Quickshell.execDetached(["sh", "-c", "printf '%s' \"$1\" | wl-copy", "_", text]);
        root.requestClose();
    }

    function activate() {
        if (currentEmoji)
            copyGlyph(currentEmoji[1]);
        else if (results.length > 0)
            copyGlyph(results[0][1]);
    }

    function move(delta) {
        if (results.length === 0)
            return;
        selectedIndex = Math.max(0, Math.min(results.length - 1, selectedIndex + delta));
        grid.positionViewAtIndex(selectedIndex, GridView.Contain);
    }

    SearchField {
        id: search
        z: 10
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
            root.hoveredItem = null;
        }
        onAccepted: root.activate()
        onMoved: delta => root.move(delta)
    }

    Item {
        id: categoryBar
        anchors.top: search.bottom
        anchors.topMargin: 8 * root.s
        anchors.left: parent.left
        anchors.right: parent.right
        height: 28 * root.s

        Row {
            anchors.fill: parent
            spacing: 3 * root.s

            Repeater {
                model: Emoji.CATEGORIES

                delegate: Rectangle {
                    id: tab
                    required property var modelData
                    width: (categoryBar.width - (Emoji.CATEGORIES.length - 1) * 3 * root.s) / Emoji.CATEGORIES.length
                    height: 28 * root.s
                    radius: 7 * root.s

                    readonly property bool active: root.activeCategory === modelData.id
                    readonly property bool hovered: tabHover.hovered

                    color: active ? Theme.frameBg : (hovered ? Qt.rgba(0.94, 0.88, 0.84, 0.05) : "transparent")
                    border.width: active ? 1 : 0
                    border.color: Theme.frameBorder

                    HoverHandler {
                        id: tabHover
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (root.activeCategory === tab.modelData.id && tab.modelData.id !== "all")
                                root.activeCategory = "all";
                            else
                                root.activeCategory = tab.modelData.id;
                            root.selectedIndex = 0;
                            root.hoveredItem = null;
                        }
                    }

                    Text {
                        anchors.centerIn: parent
                        text: tab.modelData.icon
                        font.pixelSize: 13 * root.s
                        opacity: tab.active ? 1.0 : (tab.hovered ? 0.9 : 0.6)
                        Behavior on opacity { NumberAnimation { duration: Motion.fast } }
                    }
                }
            }
        }
    }

    Rectangle {
        id: divider
        anchors.top: categoryBar.bottom
        anchors.topMargin: 8 * root.s
        anchors.left: parent.left
        anchors.right: parent.right
        height: 1
        color: Theme.hair
    }

    GridView {
        id: grid
        anchors.top: divider.bottom
        anchors.topMargin: 6 * root.s
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: footer.top
        anchors.bottomMargin: 6 * root.s
        cellWidth: Math.floor(grid.width / 9)
        cellHeight: 38 * root.s
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        model: root.results.length

        delegate: Item {
            id: cell
            required property int index
            width: grid.cellWidth
            height: grid.cellHeight

            readonly property var entry: root.results[cell.index] || ["", "", "", ""]
            readonly property bool selected: cell.index === root.selectedIndex
            readonly property bool isHover: cellHover.hovered

            HoverHandler {
                id: cellHover
                onHoveredChanged: {
                    if (hovered) {
                        root.hoveredItem = cell.entry;
                        root.selectedIndex = cell.index;
                    } else if (root.hoveredItem === cell.entry) {
                        root.hoveredItem = null;
                    }
                }
            }

            Rectangle {
                anchors.centerIn: parent
                width: parent.width - 2 * root.s
                height: parent.height - 2 * root.s
                radius: 8 * root.s
                color: (cell.selected || cell.isHover) ? Theme.frameBg : "transparent"
                border.width: cell.selected ? 1 : 0
                border.color: Theme.frameBorder
                Behavior on color { ColorAnimation { duration: Motion.fast } }

                Text {
                    anchors.centerIn: parent
                    text: cell.entry[1]
                    font.pixelSize: 19 * root.s
                    renderType: Text.NativeRendering
                }
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    root.selectedIndex = cell.index;
                    root.copyGlyph(cell.entry[1]);
                }
            }
        }
    }

    Rectangle {
        id: footer
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        height: 42 * root.s
        radius: 10 * root.s
        color: Qt.rgba(0.94, 0.88, 0.84, 0.03)
        border.width: 1
        border.color: Theme.hair

        Item {
            anchors.fill: parent
            anchors.leftMargin: 10 * root.s
            anchors.rightMargin: 10 * root.s

            Row {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                spacing: 10 * root.s
                visible: root.currentEmoji !== null

                Text {
                    text: root.currentEmoji ? root.currentEmoji[1] : ""
                    font.pixelSize: 22 * root.s
                    anchors.verticalCenter: parent.verticalCenter
                }

                Column {
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 1 * root.s

                    Text {
                        text: root.currentEmoji ? root.currentEmoji[0] : ""
                        color: Theme.cream
                        font.family: Theme.font
                        font.pixelSize: 11.5 * root.s
                        font.weight: Font.Medium
                        elide: Text.ElideRight
                        width: 220 * root.s
                    }

                    Text {
                        text: root.currentEmoji ? Emoji.codepoint(root.currentEmoji[1]) : ""
                        color: Theme.faint
                        font.family: Theme.font
                        font.pixelSize: 9.5 * root.s
                    }
                }
            }

            Text {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                text: "↵ copy"
                color: Theme.dim
                font.family: Theme.font
                font.pixelSize: 10 * root.s
                visible: root.currentEmoji !== null
            }

            Text {
                anchors.centerIn: parent
                text: root.results.length > 0 ? "Hover or arrow keys to preview" : "No emoji found"
                color: Theme.faint
                font.family: Theme.font
                font.pixelSize: 11 * root.s
                visible: root.currentEmoji === null
            }
        }
    }
}
