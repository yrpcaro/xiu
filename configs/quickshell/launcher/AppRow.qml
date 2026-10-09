import QtQuick
import Quickshell

Item {
    id: row

    required property var entry
    property bool selected: false

    signal activated()
    signal entered()

    implicitHeight: 50

    readonly property color cream: "#e6d6cb"
    readonly property color white: "#fff6f0"
    readonly property color dim2: "#565e6a"

    readonly property string secondary: {
        if (!entry) return "";
        if (entry.isCommand) return (entry.arg && entry.arg.length > 0) ? ("Arg: " + entry.arg) : (entry.desc || "");
        if (entry.genericName && entry.genericName.length > 0) return entry.genericName;
        if (entry.categories && entry.categories.length > 0) {
            var first = String(entry.categories).split(";")[0].trim();
            if (first.length > 0) return first;
        }
        return "";
    }

    Rectangle {
        anchors.fill: parent
        radius: 14
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#c0442b" }
            GradientStop { position: 1.0; color: "#a3371f" }
        }
        visible: row.selected
    }

    Rectangle {
        anchors.fill: parent
        radius: 14
        color: Qt.rgba(1, 1, 1, 0.035)
        visible: !row.selected && rowArea.containsMouse
    }

    MouseArea {
        id: rowArea
        anchors.fill: parent
        hoverEnabled: true
        onEntered: row.entered()
        onClicked: row.activated()
    }

    Item {
        anchors.fill: parent
        anchors.leftMargin: 15
        anchors.rightMargin: 15

        Rectangle {
            id: iconBox
            anchors.verticalCenter: parent.verticalCenter
            width: 26
            height: 26
            radius: 6
            color: Qt.rgba(1, 1, 1, 0.05)
            visible: (row.entry && row.entry.isCommand) || !(icon.status === Image.Ready && icon.source !== "")

            Text {
                anchors.centerIn: parent
                visible: row.entry && row.entry.isCommand
                text: ">"
                color: row.selected ? row.white : row.cream
                font.family: "Inter"
                font.pixelSize: 13
                font.weight: Font.Bold
            }
        }

        Image {
            id: icon
            anchors.fill: iconBox
            sourceSize.width: 52
            sourceSize.height: 52
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            visible: !(row.entry && row.entry.isCommand) && status === Image.Ready && source !== ""
            source: {
                if (!row.entry || row.entry.isCommand || !row.entry.icon) return "";
                var ic = row.entry.icon;
                if (ic.indexOf("/") === 0) return "file://" + ic;
                if (ic.indexOf("file://") === 0) return ic;
                var p = Quickshell.iconPath(ic, true);
                return p.length ? p : Quickshell.iconPath(ic, "application-x-executable");
            }
        }

        Row {
            id: labelRow
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: iconBox.right
            anchors.leftMargin: 12
            spacing: 6

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: row.entry ? row.entry.name : ""
                color: row.selected ? row.white : row.cream
                font.family: "Inter"
                font.pixelSize: 15
                font.weight: row.selected ? Font.DemiBold : Font.Normal
                elide: Text.ElideRight
                width: Math.min(implicitWidth, parent.parent.width - iconBox.width - 24 - (cmdBadge.visible ? cmdBadge.width + 6 : 0) - (secLabel.text.length > 0 ? secLabel.implicitWidth + 10 : 0) - (enter.visible ? enter.width + 10 : 0))
            }

            Rectangle {
                id: cmdBadge
                anchors.verticalCenter: parent.verticalCenter
                visible: row.entry && row.entry.isCommand
                height: 18
                width: cmdBadgeText.implicitWidth + 8
                radius: 4
                color: row.selected ? Qt.rgba(1, 1, 1, 0.2) : Qt.rgba(192 / 255, 68 / 255, 43 / 255, 0.22)
                border.width: 1
                border.color: row.selected ? Qt.rgba(1, 1, 1, 0.4) : Qt.rgba(192 / 255, 68 / 255, 43 / 255, 0.45)

                Text {
                    id: cmdBadgeText
                    anchors.centerIn: parent
                    text: (row.entry && row.entry.prefix) ? row.entry.prefix : ""
                    color: row.selected ? row.white : "#e0563b"
                    font.family: "Inter"
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                }
            }
        }

        Text {
            id: enter
            anchors.verticalCenter: parent.verticalCenter
            anchors.right: parent.right
            text: "↵"
            color: row.white
            font.family: "Inter"
            font.pixelSize: 13
            visible: row.selected
            width: visible ? implicitWidth + 7 : 0
            horizontalAlignment: Text.AlignRight
        }

        Text {
            id: secLabel
            anchors.verticalCenter: parent.verticalCenter
            anchors.right: enter.left
            text: row.secondary
            color: row.selected ? Qt.rgba(1, 0.965, 0.941, 0.72) : row.dim2
            font.family: "Inter"
            font.pixelSize: 12
            horizontalAlignment: Text.AlignRight
        }
    }
}
