function bboxOf(a) {
    var xs = a.points.map(function (p) { return p.x; });
    var ys = a.points.map(function (p) { return p.y; });
    var x0 = Math.min.apply(null, xs), x1 = Math.max.apply(null, xs);
    var y0 = Math.min.apply(null, ys), y1 = Math.max.apply(null, ys);
    if (a.type === "text") {
        var size = a.size || 16;
        var w = Math.max((a.text ? a.text.length : 1) * size * 0.6, size);
        return { x: x0, y: y0, w: w, h: size * 1.4 };
    }
    if (a.type === "step") {
        var d = a.size || 32;
        return { x: x0 - d / 2, y: y0 - d / 2, w: d, h: d };
    }
    return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}

function distToSeg(px, py, a, b) {
    var dx = b.x - a.x, dy = b.y - a.y;
    var len2 = dx * dx + dy * dy;
    if (len2 === 0) return Math.hypot(px - a.x, py - a.y);
    var t = ((px - a.x) * dx + (py - a.y) * dy) / len2;
    t = Math.max(0, Math.min(1, t));
    return Math.hypot(px - (a.x + t * dx), py - (a.y + t * dy));
}

function inBox(gx, gy, b, pad) {
    return gx >= b.x - pad && gx <= b.x + b.w + pad
        && gy >= b.y - pad && gy <= b.y + b.h + pad;
}

function hitOne(a, gx, gy) {
    var tol = Math.max(a.width || 4, 8);
    if (a.type === "rect" || a.type === "marker" || a.type === "blur"
        || a.type === "pixelate" || a.type === "zoom" || a.type === "text")
        return inBox(gx, gy, bboxOf(a), a.type === "text" ? 0 : tol);
    if (a.type === "step") {
        var r = (a.size || 32) / 2 + tol;
        return Math.hypot(gx - a.points[0].x, gy - a.points[0].y) <= r;
    }
    if (a.type === "line" || a.type === "arrow")
        return distToSeg(gx, gy, a.points[0], a.points[1]) <= tol;
    if (a.type === "pen") {
        for (var i = 1; i < a.points.length; i++)
            if (distToSeg(gx, gy, a.points[i - 1], a.points[i]) <= tol) return true;
        return false;
    }
    if (a.type === "ellipse") {
        var b = bboxOf(a);
        var rx = b.w / 2 + tol, ry = b.h / 2 + tol;
        if (rx <= 0 || ry <= 0) return false;
        var nx = (gx - (b.x + b.w / 2)) / rx, ny = (gy - (b.y + b.h / 2)) / ry;
        return nx * nx + ny * ny <= 1;
    }
    return false;
}

function hitTest(items, gx, gy) {
    for (var i = items.length - 1; i >= 0; i--)
        if (hitOne(items[i], gx, gy)) return i;
    return null;
}

if (typeof module !== "undefined" && module.exports) {
    module.exports = { bboxOf: bboxOf, distToSeg: distToSeg, inBox: inBox, hitOne: hitOne, hitTest: hitTest };
}
