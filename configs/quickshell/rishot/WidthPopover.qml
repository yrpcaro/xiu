import QtQuick
import QtQuick.Layouts
import "Singletons"

Item {
    id: pop

    /** Externally driven current stroke width. */
    property int selected: 4

    /** Emitted when a preset dot is tapped. */
    signal picked(int w)

    readonly property int arrow: 7
    implicitWidth: card.implicitWidth
    implicitHeight: card.implicitHeight + arrow

    readonly property var widths: [
        { id: 2, dot: 6 },
        { id: 4, dot: 11 },
        { id: 7, dot: 16 }
    ]

    Rectangle {
        id: card
        width: parent.width
        height: parent.height - pop.arrow
        radius: 10
        color: Theme.panelBg
        border.color: Theme.panelBorder
        border.width: 1
        implicitWidth: row.implicitWidth + 20
        implicitHeight: 48

        RowLayout {
            id: row
            anchors.centerIn: parent
            spacing: 6

            Repeater {
                model: pop.widths
                Rectangle {
                    required property var modelData
                    Layout.preferredWidth: 34
                    Layout.preferredHeight: 34
                    radius: 7
                    readonly property bool sel: pop.selected === modelData.id
                    color: sel ? Theme.vermilion
                        : (hh.hovered ? Qt.rgba(1, 1, 1, 0.06) : "transparent")

                    Rectangle {
                        anchors.centerIn: parent
                        width: modelData.dot
                        height: modelData.dot
                        radius: width / 2
                        color: parent.sel ? Theme.white : Theme.idle
                    }

                    HoverHandler { id: hh }
                    TapHandler { onTapped: pop.picked(modelData.id) }
                }
            }
        }
    }

    Canvas {
        width: pop.arrow * 2
        height: pop.arrow
        anchors.bottom: card.top
        anchors.horizontalCenter: card.horizontalCenter
        onPaint: {
            var ctx = getContext("2d");
            ctx.reset();
            ctx.beginPath();
            ctx.moveTo(0, height);
            ctx.lineTo(width, height);
            ctx.lineTo(width / 2, 0);
            ctx.closePath();
            ctx.fillStyle = Theme.panelBg;
            ctx.fill();
        }
    }
}
