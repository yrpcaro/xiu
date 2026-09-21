import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { ALL_COMMANDS, matchCommands } = require("./commands.js");
const { evaluate } = require("./calc.js");

let failed = 0;
function eq(actual, expected, msg) {
    const a = JSON.stringify(actual);
    const e = JSON.stringify(expected);
    if (a === e) console.log("PASS " + msg);
    else { failed++; console.log("FAIL " + msg + "\n  expected " + e + "\n  got      " + a); }
}

function ok(cond, msg) {
    if (cond) console.log("PASS " + msg);
    else { failed++; console.log("FAIL " + msg); }
}

// 1. Non-command query
eq(matchCommands("firefox"), [], "non-command query returns empty array");
eq(matchCommands(""), [], "empty query returns empty array");

// 2. All commands on bare '>'
const all = matchCommands(">");
ok(all.length >= 18, "bare > returns all commands (count: " + all.length + ")");
ok(all.some(c => c.prefix === ">calc"), "all commands includes >calc");
ok(all.some(c => c.prefix === ">install"), "all commands includes >install");
ok(all.some(c => c.prefix === ">wallpaper"), "all commands includes >wallpaper");
ok(all.some(c => c.prefix === ">lock"), "all commands includes >lock");

// 3. Prefix matching
const wa = matchCommands(">wa");
ok(wa.some(c => c.prefix === ">wallpaper"), ">wa matches >wallpaper");
ok(wa.some(c => c.prefix === ">random"), ">wa matches Random Wallpaper");

// 4. Aliases
const wp = matchCommands(">wp");
ok(wp.length === 1 && wp[0].prefix === ">wallpaper", ">wp alias matches >wallpaper");

const appimg = matchCommands(">appimage /tmp/foo.AppImage");
ok(appimg.length === 1 && appimg[0].prefix === ">install", ">appimage alias matches >install");
eq(appimg[0].arg, "/tmp/foo.AppImage", "extracts arg path correctly");

// 5. Calculator command & expressions
const calcCmd = matchCommands(">calc 24 * 7");
ok(calcCmd.length === 1 && calcCmd[0].prefix === ">calc", ">calc matches calculator command");
eq(calcCmd[0].arg, "24 * 7", "extracts math arg correctly");

const calcRes1 = evaluate("24 * 7");
ok(calcRes1.ok && calcRes1.value === 168, "bare math 24 * 7 evaluates to 168");

const calcRes2 = evaluate(">calc 24 * 7");
ok(calcRes2.ok && calcRes2.value === 168, ">calc 24 * 7 evaluates to 168");

const calcRes3 = evaluate("=168");
ok(calcRes3.ok && calcRes3.value === 168, "=168 evaluates to 168");

if (failed > 0) {
    console.error(`commands.test.mjs: ${failed} failed`);
    process.exit(1);
} else {
    console.log("commands: all tests pass");
}
