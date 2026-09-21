pragma ComponentBehavior: Bound

import QtQuick
import "Singletons"

/**
 * Clipboard surface: search field over the clipboard history, drawn as one of
 * the pill's surfaces. Entries come from the Cliphist singleton snapshot so the
 * list is populated as soon as the pill finishes morphing. Typing filters by
 * substring, Return copies the selected entry and closes, hovering a row
 * cross-fades a dismiss glyph that deletes it (Ctrl+X does the same for the
 * keyboard selection). Image entries render their cached thumbnail beside the
 * size label. Holding the 掃 glyph for the heat duration wipes the whole
 * history; progress sweeps along the header divider and drains on early
 * release.
 */
PillSurface {
    id: root

    mTop: 15
    mLeft: 17
    mRight: 17
    mBottom: 14

    property string query: ""
    property int selectedIndex: 0
    property string sortMode: "recent"
    property int armedDeleteIndex: -1
    Keys.forwardTo: [search.input]

    Timer {
        id: deleteArmTimer
        interval: 2000
        onTriggered: root.armedDeleteIndex = -1
    }

    /**
     * Window-coordinate position of the last hover event that was allowed to
     * move the selection. Rows sliding under a stationary cursor during
     * keyboard scrolling produce hover events at an unchanged window position,
     * which must not steal the keyboard selection.
     */
    property point lastPointer: Qt.point(-1, -1)

    readonly property point caretPoint: {
        void root.width;
        void root.height;
        void search.input.width;
        return search.input.mapToItem(root,
            search.input.cursorRectangle.x + search.input.cursorRectangle.width / 2,
            search.input.cursorRectangle.y + search.input.cursorRectangle.height / 2);
    }
    readonly property real caretX: caretPoint.x
    readonly property real caretY: caretPoint.y

    ameForm: "caret"
    amePoint: Qt.point(caretX, caretY)

    readonly property var results: {
        var all = Cliphist.entries;
        var q = query.trim().toLowerCase();
        var filtered = [];
        for (var i = 0; i < all.length; i++) {
            var hay = (all[i].isImage ? all[i].label + " " + all[i].sizeLabel : all[i].preview).toLowerCase();
            if (!q.length || hay.indexOf(q) !== -1)
                filtered.push(all[i]);
        }
        filtered.sort(function(a, b) {
            var aPin = a.pinned ? 1 : 0;
            var bPin = b.pinned ? 1 : 0;
            if (aPin !== bPin) {
                return bPin - aPin;
            }
            if (root.sortMode === "alpha") {
                var aText = (a.isImage ? a.label : a.preview).toLowerCase();
                var bText = (b.isImage ? b.label : b.preview).toLowerCase();
                return aText.localeCompare(bText);
            } else if (root.sortMode === "oldest") {
                return Number(a.id) - Number(b.id);
            } else {
                return Number(b.id) - Number(a.id);
            }
        });
        return filtered;
    }

    function focusField() { search.input.forceActiveFocus(); }

    function move(delta) {
        if (results.length === 0)
            return;
        armedDeleteIndex = -1;
        deleteArmTimer.stop();
        selectedIndex = Math.max(0, Math.min(results.length - 1, selectedIndex + delta));
        list.positionViewAtIndex(selectedIndex, ListView.Contain);
    }

    function activate() {
        if (results.length === 0 || selectedIndex < 0 || selectedIndex >= results.length)
            return;
        Cliphist.copy(results[selectedIndex]);
        root.requestClose();
    }

    function removeAt(index) {
        if (index < 0 || index >= results.length)
            return;
        armedDeleteIndex = -1;
        deleteArmTimer.stop();
        Cliphist.remove(results[index]);
    }

    onActiveChanged: {
        if (active) {
            query = "";
            search.text = "";
            selectedIndex = 0;
            armedDeleteIndex = -1;
            deleteArmTimer.stop();
            Cliphist.refresh();
            Qt.callLater(root.focusField);
        }
    }
    onResultsChanged: if (selectedIndex >= results.length) selectedIndex = Math.max(0, results.length - 1)

    SearchField {
        id: search
        z: 5
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        s: root.s
        kanji: "控"
        placeholder: "Search clipboard"
        counterText: root.results.length + " / " + Cliphist.count
        onTextChanged: {
            root.query = text;
            root.selectedIndex = 0;
        }
        onMoved: (d) => root.move(d)
        onAccepted: root.activate()
        onDismissed: root.requestClose()
        onKeyPressed: (e) => {
            if (e.key === Qt.Key_Delete || (e.key === Qt.Key_X && (e.modifiers & Qt.ControlModifier) && search.input.selectedText.length === 0)) {
                if (root.selectedIndex >= 0 && root.selectedIndex < root.results.length) {
                    var entry = root.results[root.selectedIndex];
                    if (entry && entry.pinned) {
                        if (root.armedDeleteIndex === root.selectedIndex && deleteArmTimer.running) {
                            deleteArmTimer.stop();
                            root.armedDeleteIndex = -1;
                            root.removeAt(root.selectedIndex);
                        } else {
                            root.armedDeleteIndex = root.selectedIndex;
                            deleteArmTimer.restart();
                        }
                    } else {
                        root.removeAt(root.selectedIndex);
                    }
                    e.accepted = true;
                }
            }
        }

        Row {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8 * root.s

            Item {
                id: sortBtn
                anchors.verticalCenter: parent.verticalCenter
                width: 16 * root.s
                height: 16 * root.s

                readonly property color tone: sortArea.containsMouse ? Theme.cream : Theme.faint

                Tooltip {
                    s: root.s
                    placement: "below"
                    title: root.sortMode === "recent" ? "sort: recent" : (root.sortMode === "alpha" ? "sort: alphabetical" : "sort: oldest")
                    show: sortArea.containsMouse
                }

                Text {
                    visible: Flags.showGlyphs
                    anchors.centerIn: parent
                    text: "序"
                    color: sortBtn.tone
                    font.family: Theme.fontJp
                    font.pixelSize: 12 * root.s
                    Behavior on color { ColorAnimation { duration: Motion.fast } }
                }

                GlyphIcon {
                    visible: !Flags.showGlyphs
                    anchors.centerIn: parent
                    width: 12 * root.s
                    height: 12 * root.s
                    name: "sort"
                    color: sortBtn.tone
                    Behavior on color { ColorAnimation { duration: Motion.fast } }
                }

                MouseArea {
                    id: sortArea
                    anchors.fill: parent
                    anchors.margins: -5 * root.s
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        if (root.sortMode === "recent") root.sortMode = "alpha";
                        else if (root.sortMode === "alpha") root.sortMode = "oldest";
                        else root.sortMode = "recent";
                    }
                }
            }

            Item {
                id: wipeBtn
                anchors.verticalCenter: parent.verticalCenter
                width: 16 * root.s
                height: 16 * root.s

                readonly property real hold: wipeHeat.hold
                readonly property bool holding: wipeHeat.holding
                readonly property color tone: holding ? Theme.vermLit : (wipeArea.containsMouse ? Theme.cream : Theme.faint)

                Tooltip {
                    s: root.s
                    placement: "below"
                    title: "hold to wipe"
                    show: wipeArea.containsMouse || wipeBtn.holding
                }

                Text {
                    visible: Flags.showGlyphs
                    anchors.centerIn: parent
                    text: "掃"
                    color: wipeBtn.tone
                    font.family: Theme.fontJp
                    font.pixelSize: 12 * root.s
                    Behavior on color { ColorAnimation { duration: Motion.fast } }
                }

                GlyphIcon {
                    visible: !Flags.showGlyphs
                    anchors.centerIn: parent
                    width: 12 * root.s
                    height: 12 * root.s
                    name: "trash"
                    color: wipeBtn.tone
                    Behavior on color { ColorAnimation { duration: Motion.fast } }
                }

                HeatHold {
                    id: wipeHeat
                    onConfirmed: Cliphist.wipe()
                }

                MouseArea {
                    id: wipeArea
                    anchors.fill: parent
                    anchors.margins: -5 * root.s
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onPressed: wipeHeat.press()
                    onReleased: wipeHeat.release()
                    onExited: wipeHeat.cancel()
                }
            }
        }
    }

    Rectangle {
        id: divider
        anchors.top: search.bottom
        anchors.topMargin: 8 * root.s
        anchors.left: parent.left
        anchors.right: parent.right
        height: 1
        color: Theme.hair

        Rectangle {
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            width: parent.width * wipeBtn.hold
            visible: wipeBtn.holding
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: Qt.alpha(Theme.vermLit, 0.15) }
                GradientStop { position: 1.0; color: Theme.vermLit }
            }
        }
    }

    Text {
        anchors.centerIn: list
        visible: root.results.length === 0
        text: root.query.length ? "No matches"
            : (Cliphist.backendMissing ? "clipvault is not installed — xiu check says what's missing"
            : (Cliphist.loaded ? "History empty" : ""))
        color: Theme.faint
        font.family: Theme.font
        font.pixelSize: 10.5 * root.s
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
            height: (entry && entry.isImage ? 44 : 28) * root.s

            readonly property var entry: root.results[index]
            readonly property bool selected: index === root.selectedIndex

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
                id: rowArea
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

                Rectangle {
                    id: thumbTile
                    anchors.verticalCenter: parent.verticalCenter
                    visible: row.entry !== undefined && row.entry.isImage
                    width: visible ? 52 * root.s : 0
                    height: 32 * root.s
                    radius: 6 * root.s
                    color: Theme.tileBg
                    border.width: 1
                    border.color: Theme.border
                    clip: true

                    Image {
                        anchors.fill: parent
                        anchors.margins: 1
                        source: thumbTile.visible ? "file://" + row.entry.thumb : ""
                        sourceSize.width: 128
                        sourceSize.height: 128
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                        cache: false
                        smooth: true
                    }
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: thumbTile.visible ? thumbTile.right : parent.left
                    anchors.leftMargin: thumbTile.visible ? 9 * root.s : 0
                    anchors.right: sizeTag.left
                    anchors.rightMargin: 8 * root.s
                    text: row.entry === undefined ? "" : (row.entry.isImage ? row.entry.label : row.entry.preview)
                    color: row.entry !== undefined && row.entry.isImage
                        ? (row.selected ? Theme.dim : Theme.faint)
                        : (row.selected ? Theme.cream : Theme.subtle)
                    font.family: Theme.font
                    font.pixelSize: 11.5 * root.s
                    font.weight: row.selected ? Font.DemiBold : Font.Medium
                    elide: Text.ElideRight
                    maximumLineCount: 1
                    textFormat: Text.PlainText
                }

                Text {
                    id: sizeTag
                    anchors.right: tail.left
                    anchors.rightMargin: width > 0 ? 8 * root.s : 0
                    anchors.verticalCenter: parent.verticalCenter
                    text: row.entry !== undefined && row.entry.isImage ? row.entry.sizeLabel : ""
                    width: text.length ? implicitWidth : 0
                    color: Theme.faint
                    font.family: Theme.font
                    font.pixelSize: 10.5 * root.s
                    font.features: { "tnum": 1 }
                }

                Row {
                    id: tail
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 6 * root.s

                    Text {
                        id: ret
                        anchors.verticalCenter: parent.verticalCenter
                        opacity: row.selected && !rowHover.hovered && !delBtn.isArmed ? 1 : 0
                        text: "↵"
                        color: Theme.vermLit
                        font.family: Theme.font
                        font.pixelSize: 12 * root.s
                        Behavior on opacity { NumberAnimation { duration: Motion.fast } }
                    }

                    Item {
                        id: pinBtn
                        width: 16 * root.s
                        height: 16 * root.s
                        anchors.verticalCenter: parent.verticalCenter
                        visible: row.entry !== undefined && (row.entry.pinned || row.selected || rowHover.hovered)
                        opacity: (row.entry !== undefined && row.entry.pinned) ? 1.0 : (pinArea.containsMouse ? 1.0 : 0.6)

                        Tooltip {
                            s: root.s
                            placement: "left"
                            title: (row.entry !== undefined && row.entry.pinned) ? "unpin item" : "pin item"
                            show: pinArea.containsMouse
                        }

                        Text {
                            visible: Flags.showGlyphs
                            anchors.centerIn: parent
                            text: "留"
                            color: (row.entry !== undefined && row.entry.pinned) ? Theme.vermLit : (pinArea.containsMouse ? Theme.cream : Theme.dim)
                            font.family: Theme.fontJp
                            font.pixelSize: 11 * root.s
                            Behavior on color { ColorAnimation { duration: Motion.fast } }
                        }

                        GlyphIcon {
                            visible: !Flags.showGlyphs
                            anchors.centerIn: parent
                            width: 12 * root.s
                            height: 12 * root.s
                            name: (row.entry !== undefined && row.entry.pinned) ? "pin-filled" : "pin"
                            color: (row.entry !== undefined && row.entry.pinned) ? Theme.vermLit : (pinArea.containsMouse ? Theme.cream : Theme.dim)
                            Behavior on color { ColorAnimation { duration: Motion.fast } }
                        }

                        MouseArea {
                            id: pinArea
                            anchors.fill: parent
                            anchors.margins: -4 * root.s
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: Cliphist.togglePin(row.entry)
                        }
                    }

                    Item {
                        id: delBtn
                        width: 16 * root.s
                        height: 16 * root.s
                        anchors.verticalCenter: parent.verticalCenter
                        visible: row.selected || rowHover.hovered || holding || isArmed
                        readonly property bool isPinned: row.entry !== undefined && row.entry.pinned
                        readonly property real hold: delHeat.hold
                        readonly property bool holding: delHeat.holding
                        readonly property bool isArmed: root.armedDeleteIndex === row.index && deleteArmTimer.running

                        HeatHold {
                            id: delHeat
                            onConfirmed: root.removeAt(row.index)
                        }

                        Tooltip {
                            s: root.s
                            placement: "left"
                            title: delBtn.isPinned ? (delBtn.holding ? "holding to delete..." : (delBtn.isArmed ? "press delete again" : "hold to delete pinned")) : "delete item"
                            show: delArea.containsMouse || delBtn.holding || delBtn.isArmed
                        }

                        Rectangle {
                            anchors.fill: parent
                            radius: width / 2
                            visible: delBtn.holding || delBtn.isArmed
                            color: Qt.alpha(Theme.vermLit, delBtn.holding ? (0.2 + 0.8 * delBtn.hold) : 0.3)
                        }

                        Text {
                            anchors.centerIn: parent
                            text: delBtn.isArmed ? "!" : "✕"
                            color: (delBtn.holding || delBtn.isArmed) ? Theme.vermLit : (delArea.containsMouse ? Theme.cream : Theme.dim)
                            font.pixelSize: 10 * root.s
                            font.weight: delBtn.isArmed ? Font.Bold : Font.Normal
                            Behavior on color { ColorAnimation { duration: Motion.fast } }
                        }

                        MouseArea {
                            id: delArea
                            anchors.fill: parent
                            anchors.margins: -4 * root.s
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onPressed: {
                                if (delBtn.isPinned) {
                                    delHeat.press();
                                }
                            }
                            onReleased: {
                                if (delBtn.isPinned) {
                                    delHeat.release();
                                } else {
                                    root.removeAt(row.index);
                                }
                            }
                            onExited: {
                                if (delBtn.isPinned) {
                                    delHeat.cancel();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    WheelScroller {
        anchors.fill: list
        s: root.s
        flick: list
    }
}
