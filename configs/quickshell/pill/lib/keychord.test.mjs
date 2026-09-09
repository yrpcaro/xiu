import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { chord, letterForScanCode } = require("./keychord.js");

let failed = 0;
function eq(actual, expected, msg) {
    const a = JSON.stringify(actual);
    const e = JSON.stringify(expected);
    if (a === e) console.log("PASS " + msg);
    else { failed++; console.log("FAIL " + msg + "\n  expected " + e + "\n  got      " + a); }
}

// Qt delivers: key = keysym, modifiers = bitmask, nativeScanCode = raw evdev
// (xkb = evdev + 8). Letter and digit keys must capture as raw keycodes so a
// bind recorded on the us layout hits the same physical key on ir(winkeys).
// The xkb table read from /usr/share/X11/xkb/keycodes/evdev (its numbers ARE
// xkb codes = raw evdev + 8): L = <AC09> = 46, O = <AD09> = 32, G = <AC05> = 42,
// Q = <AD01> = 24, digit 1 = <AE01> = 10. Raw evdev: linux KEY_L = 38, KEY_O = 32,
// KEY_G = 34, KEY_1 = 2.
const SUPER = 0x10000000, SHIFT = 0x02000000;

eq(chord(0x47, SUPER, 34), "SUPER + code:42", "G with Super captures as code:42 (KEY_G=34)");
eq(chord(0x4c, 0, 38), "code:46", "bare L captures as code:46 (the lock bind)");
eq(chord(0x4f, SUPER, 24), "SUPER + code:32", "O captures as code:32 (KEY_O=24, the spotify bind)");
eq(chord(0x32, 0, 2), "code:10", "digit 1 captures as code:10");
eq(chord(0x4c, SHIFT, 38), "SHIFT + code:46", "Shift+L keeps its modifier");

// Named keys stay symbolic — their keycodes are identical across layouts
eq(chord(0x01000034, SUPER, 63), "SUPER + F5", "F5 stays symbolic");
eq(chord(0x01000009, 0, 99), "Print", "Print stays symbolic");
eq(chord(0x20, 0, 65), "Space", "Space stays symbolic");

// Bare modifiers keep waiting for the final key
eq(chord(0x01000020, SHIFT, 50), null, "bare Shift returns null");
eq(chord(0x01000022, 0, 64), null, "bare Alt returns null");

// No scan code available (defensive) falls back to the keysym name
eq(chord(0x47, SUPER), "SUPER + G", "missing scanCode falls back to keysym");

// letterForScanCode takes the RAW evdev code and maps to the us character
// (xkb = evdev + 8: L -> 38+8=46, Q -> 16+8=24, 2 -> 3+8=11, 0 -> 11+8=19)
eq(letterForScanCode(38), "l", "evdev 38 (KEY_L) -> l (xkb 46)");
eq(letterForScanCode(16), "q", "evdev 16 (KEY_Q) -> q (xkb 24)");
eq(letterForScanCode(3), "2", "evdev 3 (KEY_2) -> 2 (xkb 11)");
eq(letterForScanCode(11), "0", "evdev 11 (KEY_0) -> 0 (xkb 19)");
eq(letterForScanCode(91), null, "unmapped evdev code -> null");

if (failed > 0) {
    console.error(failed + " failures");
    process.exit(1);
}
console.log("keychord: all capture tests pass");
