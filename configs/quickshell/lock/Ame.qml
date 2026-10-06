pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Effects
import "Singletons"

/**
 * 飴 Ame for the lock screen.
 * The breathing molten-glass bead consistent with the pill/shell.
 * Renders the warm-ember glowing sphere with internal swirl and breathing scale.
 */
Item {
    id: root

    property real s: 1
    property point point: Qt.point(0, 0)
    property string form: "rest"
    property real heat: 0

    readonly property real restR: 5.5 * s
    property real bx: point.x
    property real by: point.y
    property real swirl: 0

    Behavior on bx { NumberAnimation { duration: 340; easing.type: Easing.OutCubic } }
    Behavior on by { NumberAnimation { duration: 340; easing.type: Easing.OutCubic } }

    onPointChanged: {
        bx = point.x;
        by = point.y;
    }

    Timer {
        running: root.visible
        interval: 160
        repeat: true
        onTriggered: {
            root.swirl += interval * 0.0005;
            canvas.requestPaint();
        }
    }

    Canvas {
        id: canvas
        anchors.fill: parent
        renderStrategy: Canvas.Cooperative
        antialiasing: true

        readonly property real breathe: 1 + 0.025 * Math.sin(root.swirl * 0.32)

        function bead(ctx, x, y, R, stretch, ang, alpha) {
            ctx.save();
            if (alpha !== undefined)
                ctx.globalAlpha = alpha;
            ctx.translate(x, y);
            ctx.rotate(ang);
            ctx.scale(1 + stretch, 1 / (1 + stretch * 0.55));
            ctx.rotate(-ang);
            const hg = ctx.createRadialGradient(-R * 0.32, -R * 0.38, 0, 0, 0, R);
            hg.addColorStop(0, Theme.flameInk);
            hg.addColorStop(0.55, Theme.vermLit);
            hg.addColorStop(0.92, Theme.verm);
            hg.addColorStop(1, Theme.flameEmber);
            ctx.beginPath();
            ctx.arc(0, 0, R, 0, 7);
            ctx.fillStyle = hg;
            ctx.fill();
            ctx.save();
            ctx.beginPath();
            ctx.arc(0, 0, R, 0, 7);
            ctx.clip();
            ctx.globalAlpha = (alpha === undefined ? 1 : alpha) * 0.35;
            for (let k = 0; k < 2; k++) {
                ctx.beginPath();
                ctx.arc(0, 0, R * (0.45 + k * 0.22),
                        root.swirl * (0.5 + k * 0.25) + k * 2.6,
                        root.swirl * (0.5 + k * 0.25) + k * 2.6 + 2.4);
                ctx.strokeStyle = k ? Theme.flameBurn : Theme.flameTip;
                ctx.lineWidth = 1.6 * root.s;
                ctx.stroke();
            }
            ctx.restore();
            ctx.beginPath();
            ctx.ellipse(-R * 0.34 - R * 0.30, -R * 0.42 - R * 0.18, R * 0.60, R * 0.36);
            ctx.fillStyle = "rgba(255,246,240,0.6)";
            ctx.fill();
            ctx.beginPath();
            ctx.arc(0, 0, Math.max(0.5, R - 0.8 * root.s), Math.PI * 0.25, Math.PI * 0.75);
            ctx.strokeStyle = "rgba(255,217,194,0.45)";
            ctx.lineWidth = 1.2 * root.s;
            ctx.stroke();
            ctx.restore();
        }

        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();
            if (!root.visible)
                return;

            const S = root.s;
            const curBx = root.bx;
            const curBy = root.by;
            const baseR = root.restR;
            const r = baseR * canvas.breathe;
            bead(ctx, curBx, curBy, r, 0, 0);
        }
    }

    layer.enabled: true
    layer.effect: MultiEffect {
        blurEnabled: true
        blur: 0.34
        blurMax: 8
    }
}
