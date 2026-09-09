var NAMED_KEYS = {
    0x01000009: "Print",
    0x01001007: "Print",
    0x01000000: "Escape",
    0x01000001: "Tab",
    0x01000004: "Return",
    0x01000005: "Return",
    0x20:       "Space",
    0x01000006: "Insert",
    0x01000007: "Delete",
    0x01000010: "Home",
    0x01000011: "End",
    0x01000016: "PageUp",
    0x01000017: "PageDown",
    0x01000012: "Left",
    0x01000013: "Up",
    0x01000014: "Right",
    0x01000015: "Down"
};

var MOD_BITS = [
    { mask: 0x10000000, name: "SUPER" },
    { mask: 0x04000000, name: "CTRL" },
    { mask: 0x08000000, name: "ALT" },
    { mask: 0x02000000, name: "SHIFT" }
];

var MODIFIER_KEYS = {
    0x01000020: true, 0x01000021: true,
    0x01000022: true, 0x01000023: true,
    0x01000024: true,
    0x01000025: true,
    0x01000026: true,
    0x01001103: true
};

var F1 = 0x01000030, F35 = 0x01000052;

var PUNCT = {
    0x2e: "period", 0x2c: "comma", 0x2f: "slash", 0x5c: "backslash",
    0x3b: "semicolon", 0x27: "apostrophe", 0x60: "grave",
    0x5b: "bracketleft", 0x5d: "bracketright", 0x2d: "minus", 0x3d: "equal"
};

function keyName(key) {
    if (MODIFIER_KEYS[key]) return null;
    if (NAMED_KEYS[key]) return NAMED_KEYS[key];
    if (PUNCT[key]) return PUNCT[key];
    if (key >= F1 && key <= F35) return "F" + (key - F1 + 1);
    if (key >= 0x41 && key <= 0x5a) return String.fromCharCode(key);
    if (key >= 0x30 && key <= 0x39) return String.fromCharCode(key);
    return null;
}

function modNames(modifiers) {
    var out = [];
    for (var i = 0; i < MOD_BITS.length; i++)
        if (modifiers & MOD_BITS[i].mask) out.push(MOD_BITS[i].name);
    return out;
}

/**
 * xkb keycode -> the us-layout character on that physical key, read straight
 * from /usr/share/X11/xkb/keycodes/evdev (that file's numbers are already xkb
 * keycodes = raw evdev + 8: KEY_Q=16 -> <AD01>=24). binds.lua writes its
 * letter binds on these codes so every bind hits the same physical key on the
 * us and ir(winkeys) layouts; the editor needs the reverse map to show a
 * letter next to a captured code. Qt's nativeScanCode is the raw evdev code —
 * add 8 for the xkb keycode.
 */
var US_LETTERS = {
    24: "q", 25: "w", 26: "e", 27: "r", 28: "t", 29: "y", 30: "u", 31: "i", 32: "o", 33: "p",
    38: "a", 39: "s", 40: "d", 41: "f", 42: "g", 43: "h", 44: "j", 45: "k", 46: "l",
    52: "z", 53: "x", 54: "c", 55: "v", 56: "b", 57: "n", 58: "m",
    10: "1", 11: "2", 12: "3", 13: "4", 14: "5", 15: "6", 16: "7", 17: "8", 18: "9", 19: "0"
};

/** The us-layout character for a raw evdev scan code, or null. */
function letterForScanCode(scan) {
    return US_LETTERS[scan + 8] || null;
}

/**
 * Turn a captured Qt keypress into the combo string Binds.rebind/inUse
 * expect. Letters, digits and punctuation capture as code:NNN — the raw
 * keycode binds.lua uses, so a bind recorded on the us layout hits the same
 * physical key on ir(winkeys). Named keys (Print, F-rows, arrows, space)
 * stay symbolic: their keycodes are identical across xkb layouts, so the
 * readable name loses nothing. Returns null for a bare modifier press so
 * the caller keeps listening for the final key.
 */
function chord(key, modifiers, scanCode) {
    var k = keyName(key);
    if (k === null) return null;
    // The keyName result for letters/digits is the KEYSYM's character; when
    // a scan code is available and that key sits in the letter/digit zone,
    // prefer the raw keycode. Punctuation keys stay symbolic (their
    // keycodes differ across xkb layouts but the names stay parseable).
    if (scanCode !== undefined && scanCode > 0 && US_LETTERS[scanCode + 8])
        k = "code:" + (scanCode + 8);
    var parts = modNames(modifiers);
    parts.push(k);
    return parts.join(" + ");
}

if (typeof module !== "undefined" && module.exports) {
    module.exports = { chord, keyName, letterForScanCode, US_LETTERS };
}
