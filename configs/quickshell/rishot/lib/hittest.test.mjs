import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { bboxOf, distToSeg, inBox, hitOne, hitTest } = require("./hittest.js");

let failed = 0;
function eq(actual, expected, msg) {
    const a = JSON.stringify(actual);
    const e = JSON.stringify(expected);
    if (a === e) {
        console.log("PASS " + msg);
    } else {
        failed++;
        console.log("FAIL " + msg + "\n  expected " + e + "\n  got      " + a);
    }
}

eq(bboxOf({ type: "rect", points: [{ x: 50, y: 40 }, { x: 10, y: 10 }] }),
    { x: 10, y: 10, w: 40, h: 30 }, "bbox normalizes reversed corners");
eq(bboxOf({ type: "step", points: [{ x: 100, y: 100 }], size: 32 }),
    { x: 84, y: 84, w: 32, h: 32 }, "bbox of step centres on point");

eq(Math.round(distToSeg(5, 0, { x: 0, y: 0 }, { x: 10, y: 0 })), 0, "point on segment");
eq(Math.round(distToSeg(5, 5, { x: 0, y: 0 }, { x: 10, y: 0 })), 5, "perpendicular distance");
eq(Math.round(distToSeg(0, 0, { x: 0, y: 0 }, { x: 0, y: 0 })), 0, "degenerate segment is point distance");

eq(inBox(15, 15, { x: 10, y: 10, w: 20, h: 20 }, 0), true, "inBox hit inside");
eq(inBox(5, 5, { x: 10, y: 10, w: 20, h: 20 }, 0), false, "inBox miss outside");
eq(inBox(8, 8, { x: 10, y: 10, w: 20, h: 20 }, 4), true, "inBox pad extends reach");

const rect = { type: "rect", points: [{ x: 10, y: 10 }, { x: 60, y: 50 }], width: 4 };
eq(hitOne(rect, 30, 30), true, "hitOne rect inside");
eq(hitOne(rect, 200, 200), false, "hitOne rect far miss");

const line = { type: "line", points: [{ x: 0, y: 0 }, { x: 100, y: 0 }], width: 4 };
eq(hitOne(line, 50, 3), true, "hitOne line within tolerance");
eq(hitOne(line, 50, 40), false, "hitOne line outside tolerance");

const ell = { type: "ellipse", points: [{ x: 0, y: 0 }, { x: 100, y: 100 }], width: 4 };
eq(hitOne(ell, 50, 50), true, "hitOne ellipse centre");
eq(hitOne(ell, 2, 2), false, "hitOne ellipse corner gap");

const pen = { type: "pen", points: [{ x: 0, y: 0 }, { x: 10, y: 10 }, { x: 20, y: 0 }], width: 4 };
eq(hitOne(pen, 10, 9), true, "hitOne pen on stroke");
eq(hitOne(pen, 100, 100), false, "hitOne pen far miss");

const step = { type: "step", points: [{ x: 100, y: 100 }], size: 32 };
eq(hitOne(step, 105, 105), true, "hitOne step within radius");
eq(hitOne(step, 200, 200), false, "hitOne step far miss");

const items = [
    { type: "rect", points: [{ x: 0, y: 0 }, { x: 100, y: 100 }], width: 4 },
    { type: "rect", points: [{ x: 20, y: 20 }, { x: 60, y: 60 }], width: 4 }
];
eq(hitTest(items, 40, 40), 1, "hitTest returns topmost overlapping index");
eq(hitTest(items, 90, 90), 0, "hitTest falls through to lower item");
eq(hitTest(items, 500, 500), null, "hitTest miss returns null");
eq(hitTest([], 0, 0), null, "hitTest empty returns null");

if (failed > 0) {
    console.log("\n" + failed + " test(s) FAILED");
    process.exit(1);
}
console.log("\nAll tests PASSED");
