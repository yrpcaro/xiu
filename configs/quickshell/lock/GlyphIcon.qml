import QtQuick
import QtQuick.Shapes

/**
 * Baked vector glyphs for the lock, same recipe as the pill's GlyphIcon: 24x24
 * SVG path data stroked into a Shape, so nothing depends on icon themes. Only
 * the glyphs the lock actually needs live here.
 */
Item {
    id: root

    property string name: ""
    property color color: Theme.dim
    property real stroke: 1.8

    readonly property real u: Math.min(width, height) / 24

    readonly property var glyphs: ({
        "eye": { d: "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z", fill: false },
        "eye-off": { d: "M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94 M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19 M14.12 14.12a3 3 0 1 1-4.24-4.24 M1 1l22 22", fill: false },
        "user": { d: "M12 12.5a4.2 4.2 0 1 0 0-8.4 4.2 4.2 0 0 0 0 8.4z M4.5 21v-1.2a5.8 5.8 0 0 1 5.8-5.8h3.4a5.8 5.8 0 0 1 5.8 5.8V21", fill: false },
        "wifi": { d: "M4 9.5C9 4.8 15 4.8 20 9.5 M7 13c3-2.8 7-2.8 10 0 M11 16.8a1.4 1.4 0 1 0 2 0a1.4 1.4 0 1 0-2 0", fill: false },
        "bolt": { d: "M13 2 4 13.5h6.5L11 22l9-11.5h-6.5z", fill: false },
        "moon": { d: "M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z", fill: false },
        "suspend": { d: "M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8z", fill: false },
        "reboot": { d: "M21 12a9 9 0 1 1-2.6-6.4 M21 3v5h-5", fill: false },
        "shutdown": { d: "M12 3v9 M7.8 6.3a8 8 0 1 0 8.4 0", fill: false }
    })
    readonly property var g: glyphs[name] !== undefined ? glyphs[name] : ({ d: "", fill: false })

    Shape {
        id: glyph

        width: 24
        height: 24
        scale: root.u
        transformOrigin: Item.TopLeft
        x: glyph.boundingRect.width > 0
           ? root.width / 2 - (glyph.boundingRect.x + glyph.boundingRect.width / 2) * root.u
           : (root.width - 24 * root.u) / 2
        y: glyph.boundingRect.height > 0
           ? root.height / 2 - (glyph.boundingRect.y + glyph.boundingRect.height / 2) * root.u
           : (root.height - 24 * root.u) / 2
        antialiasing: true
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: root.g.fill ? "transparent" : root.color
            fillColor: root.g.fill ? root.color : "transparent"
            strokeWidth: root.stroke
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.g.d }
        }
    }
}
