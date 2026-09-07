pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io
import "../lib/setDeco.js" as SetDeco
import "../lib/setInput.js" as SetInput
import "../lib/setAnim.js" as SetAnim

/**
 * STORE: the one validated read/write path for every setting Schema describes,
 * ported from CapsuleOS's design. A page calls `Store.get(id)` for the current
 * value and `Store.set(id, value)` to change it, and Store decides where that
 * value actually lives, coerces and range-checks it on the way in, and surfaces
 * a failure as `writeFailed(id, message)` instead of swallowing it.
 *
 * Routing is entirely Schema-driven, by `backend` then `key`:
 *
 *   flags  — a plain `Flags.<key>` assignment (the JSON state file).
 *   idle   — the three hypridle timeouts: writes the Flags key, then regenerates
 *            the whole hypridle.conf and restarts the service once that conf
 *            has actually landed.
 *   night  — the NightLight singleton's setters, so the conf debounce and the
 *            live hyprsunset push stay in one place.
 *   deco   — decoration.lua. A dotted key (`blur.size`) is scoped to that Lua
 *            block so a name shared by sibling blocks lands in the right one.
 *   input  — input.lua for lower-case keys; an UPPER_CASE key is an env.lua
 *            variable, which for the cursor also rewrites autostart.lua's
 *            `setcursor` line and pushes the pair live over hyprctl.
 *   anim   — animations.lua: `enabled` flips the master flag, `speed` retimes
 *            every leaf, and a curve key rewrites one named bezier's points.
 *
 * A Schema entry with an empty `key` and `control: "custom"` is a bespoke
 * editor that owns its own file (monitors.lua, binds.lua, the layout picker,
 * the hue strip). Those are deliberately NOT routable: `set` warns and returns
 * false.
 *
 * The file-backed texts are read at construction and refreshed on demand
 * rather than watched. A watcher would re-read while an atomic write of ours
 * was still in flight and hand the next write a stale base, dropping the value
 * just saved. Instead every file-backed write re-reads that one file
 * immediately before it applies the field edit, so an edit made by hand, by
 * another tool, or by a page that has not migrated onto Store yet (Look, Input
 * and Animation still carry their own writers) is the base the rewrite starts
 * from and survives it rather than being silently reverted. The re-read is
 * skipped only while one of Store's own writes to that file is still landing.
 * `reload()` refreshes everything under the same rule, for a page that wants
 * to resync on open.
 */
Singleton {
    id: root

    /** A value landed on its backend. */
    signal wrote(string id)
    /** A write was refused or failed; `message` is user-facing. */
    signal writeFailed(string id, string message)

    readonly property string decoPath: Quickshell.env("HOME") + "/.config/hypr/modules/decoration.lua"
    readonly property string inputPath: Quickshell.env("HOME") + "/.config/hypr/modules/input.lua"
    readonly property string animPath: Quickshell.env("HOME") + "/.config/hypr/modules/animations.lua"
    readonly property string envPath: Quickshell.env("HOME") + "/.config/hypr/modules/env.lua"
    readonly property string autostartPath: Quickshell.env("HOME") + "/.config/hypr/modules/autostart.lua"
    readonly property string hypridlePath: Quickshell.env("HOME") + "/.config/hypr/hypridle.conf"
    readonly property string lockScript: Quickshell.env("HOME") + "/.config/hypr/scripts/lock.sh"

    property string _decoText: ""
    property string _inputText: ""
    property string _animText: ""
    property string _envText: ""
    property string _autostartText: ""

    /** The Lua texts, readable but not writable from outside. */
    readonly property string decoText: root._decoText
    readonly property string inputText: root._inputText
    readonly property string animText: root._animText

    /**
     * Writes of ours still landing, per file. A `setText` is asynchronous, so
     * between the call and the writer's `saved` the file on disk is still the
     * old one; re-reading in that window would resurrect the value we just
     * replaced. While a counter is above zero the in-memory text is authoritative
     * and `_sync` leaves it alone.
     */
    property int _decoPending: 0
    property int _inputPending: 0
    property int _animPending: 0
    property int _envPending: 0
    property int _autostartPending: 0

    /** The id whose write armed the pending Hyprland reload, so a failed reload can name it. */
    property string _reloadId: ""

    /**
     * Re-reads one backing file so the next rewrite starts from what is
     * actually on disk. Called at the top of every file-backed write. Skipped
     * while one of our own writes to that file is still in flight, since the
     * in-memory text is then the fresher of the two. The readers are
     * `blockAllReads`, without which this whole function is a no-op: `reload()`
     * alone is asynchronous and the following `text()` would hand back the very
     * snapshot being escaped.
     */
    function _sync(which) {
        if (which === "deco") {
            if (root._decoPending > 0)
                return;
            decoFile.reload();
            root._decoText = decoFile.text();
        } else if (which === "input") {
            if (root._inputPending > 0)
                return;
            inputFile.reload();
            root._inputText = inputFile.text();
        } else if (which === "anim") {
            if (root._animPending > 0)
                return;
            animFile.reload();
            root._animText = animFile.text();
        } else if (which === "env") {
            if (root._envPending > 0)
                return;
            envFile.reload();
            root._envText = envFile.text();
        } else if (which === "autostart") {
            if (root._autostartPending > 0)
                return;
            autostartFile.reload();
            root._autostartText = autostartFile.text();
        }
    }

    /** Re-reads every backing file. For a page that wants to resync on open. */
    function reload() {
        root._sync("deco");
        root._sync("input");
        root._sync("anim");
        root._sync("env");
        root._sync("autostart");
    }

    // ── validation ────────────────────────────────────────────────────────

    /**
     * Coerces `v` to the entry's declared type, clamps it into `from..to` when
     * the entry carries a range, and checks membership for a segmented choice.
     * Returns `{ ok, value, msg }`; a value that cannot be coerced or is not one
     * of the offered options fails rather than being written approximately.
     */
    function _coerce(e, v) {
        var out;
        if (e.type === "bool") {
            if (typeof v !== "boolean" && v !== 0 && v !== 1 && v !== "true" && v !== "false")
                return { ok: false, value: v, msg: e.label + " needs an on/off value." };
            out = v === true || v === 1 || v === "true";
        } else if (e.type === "int" || e.type === "real") {
            var n = typeof v === "number" ? v : parseFloat(v);
            if (typeof n !== "number" || isNaN(n) || !isFinite(n))
                return { ok: false, value: v, msg: e.label + " needs a number." };
            if (typeof e.from === "number" && typeof e.to === "number")
                n = Math.max(e.from, Math.min(e.to, n));
            out = e.type === "int" ? Math.round(n) : n;
        } else {
            if (v === undefined || v === null)
                return { ok: false, value: v, msg: e.label + " needs a value." };
            out = String(v);
        }

        if (e.control === "seg" && Array.isArray(e.options)) {
            var hit = false;
            for (var i = 0; i < e.options.length; i++)
                if (e.options[i].value === out)
                    hit = true;
            if (!hit)
                return { ok: false, value: out, msg: out + " is not an option for " + e.label + "." };
        }
        return { ok: true, value: out, msg: "" };
    }

    /** Formats a coerced value as the Lua literal its backend expects. */
    function _lit(e, v) {
        if (e.type === "string")
            return "\"" + v + "\"";
        if (e.type === "bool")
            return v ? "true" : "false";
        if (e.type === "int")
            return String(v);
        return e.backend === "deco" ? v.toFixed(2) : String(Math.round(v * 1e6) / 1e6);
    }

    function _fail(id, message) {
        root.writeFailed(id, message);
        return false;
    }

    /**
     * A writer came back failed. The disk write is asynchronous by design — `set`
     * validates, hands the new text to the writer and returns true, so the control
     * follows the value the user chose instead of waiting on the filesystem — which
     * makes this the ONLY place a permissions or disk error can still reach them.
     */
    function _saveFailed(file) {
        console.warn("Store: writing " + file + " failed");
        root.writeFailed(file, "Couldn't write " + file + " — the change wasn't saved.");
    }

    // ── reads ─────────────────────────────────────────────────────────────

    /** Parses a raw Lua/config string into the entry's declared type. */
    function _parse(e, raw) {
        if (raw === undefined || raw === null || raw === "")
            return e.def;
        if (e.type === "bool")
            return raw === "true" || raw === true;
        if (e.type === "int") {
            var i = parseInt(raw, 10);
            return isNaN(i) ? e.def : i;
        }
        if (e.type === "real") {
            var f = parseFloat(raw);
            return isNaN(f) ? e.def : f;
        }
        return String(raw);
    }

    /**
     * The current value of `id`, read from wherever it actually lives. Falls
     * back to the Schema default when the field is missing from its file, so a
     * hand-trimmed config never leaves a control blank.
     */
    function get(id) {
        var e = Schema.settings[id];
        if (e === undefined)
            return undefined;
        if (e.key.length === 0)
            return e.def;

        switch (e.backend) {
        case "flags":
        case "idle":
            return Flags[e.key];
        case "night":
            return Flags[e.key];
        case "deco":
            var dot = e.key.indexOf(".");
            return dot < 0
                ? root._parse(e, SetDeco.getField(root._decoText, e.key))
                : root._parse(e, SetDeco.getBlockField(root._decoText, e.key.slice(0, dot), e.key.slice(dot + 1)));
        case "input":
            // env.lua is `hl.env("KEY", "value")` calls, input.lua is a
            // `name = value` table: reading an env key with the field parser
            // matches nothing and hands back the Schema default whatever the
            // file holds.
            return root._isEnvKey(e.key)
                ? root._parse(e, SetInput.getEnv(root._envText, e.key))
                : root._parse(e, SetInput.getField(root._inputText, e.key));
        case "anim":
            if (e.key === "enabled")
                return SetAnim.getEnabled(root._animText) === "true";
            if (e.key === "speed")
                return root._parse(e, SetAnim.getLeafSpeed(root._animText, "global"));
            var pts = SetAnim.getCurvePoints(root._animText, e.key);
            return pts === null ? e.def : pts.map(function (p) { return p.toFixed(2); }).join(",");
        }
        return e.def;
    }

    /** An UPPER_CASE `input` key is an env.lua variable, not an input.lua field. */
    function _isEnvKey(key) {
        return /^[A-Z0-9_]+$/.test(key);
    }

    // ── writes ────────────────────────────────────────────────────────────

    /**
     * Validates `value` against the Schema entry for `id` and routes it to its
     * backend. Returns true once the value has landed; on any refusal or failed
     * rewrite it emits `writeFailed` and returns false without touching a file.
     */
    function set(id, value) {
        var e = Schema.settings[id];
        if (e === undefined) {
            console.warn("Store: set on unknown setting '" + id + "'");
            return false;
        }
        if (e.key.length === 0 && e.control === "custom") {
            console.warn("Store: '" + id + "' is a bespoke control that owns its own backend; it is not routed through Store.");
            return false;
        }

        var c = root._coerce(e, value);
        if (!c.ok)
            return root._fail(id, c.msg);
        var v = c.value;

        switch (e.backend) {
        case "flags":
            Flags[e.key] = v;
            break;
        case "idle":
            // The restart is armed by the writer's `saved`, not here: the write is
            // asynchronous, so restarting alongside it would reload the conf that
            // is still on disk — the OLD one.
            Flags[e.key] = v;
            idleWriter.setText(root.buildIdleConf());
            break;
        case "night":
            if (!root._setNight(e, v))
                return root._fail(id, "Couldn't apply " + e.label + ".");
            break;
        case "deco":
            if (!root._setDeco(id, e, v))
                return root._fail(id, "Couldn't save " + e.label + " — the field is missing from decoration.lua.");
            break;
        case "input":
            if (!root._setInput(id, e, v))
                return root._fail(id, "Couldn't save " + e.label + " — the field is missing from the input config.");
            break;
        case "anim":
            if (!root._setAnim(id, e, v))
                return root._fail(id, "Couldn't save " + e.label + " — the field is missing from animations.lua.");
            break;
        default:
            return root._fail(id, "No backend for " + e.label + ".");
        }

        root.wrote(id);
        return true;
    }

    /**
     * The full hypridle.conf for the current three Flags values, byte-for-byte
     * the generator the Idle/Lock page carries — regenerating with unchanged
     * values reproduces the file exactly.
     */
    function buildIdleConf() {
        var out = "general {\n"
            + "    lock_cmd = " + root.lockScript + "\n"
            + "    before_sleep_cmd = loginctl lock-session\n"
            + "    after_sleep_cmd = hyprctl dispatch dpms on\n"
            + "}\n";

        if (Flags.idleLockMin > 0)
            out += "\nlistener {\n"
                + "    timeout = " + (Flags.idleLockMin * 60) + "\n"
                + "    on-timeout = " + root.lockScript + "\n"
                + "}\n";

        if (Flags.idleScreenOffMin > 0)
            out += "\nlistener {\n"
                + "    timeout = " + (Flags.idleScreenOffMin * 60) + "\n"
                + "    on-timeout = hyprctl dispatch dpms off\n"
                + "    on-resume = hyprctl dispatch dpms on\n"
                + "}\n";

        if (Flags.idleSuspendMin > 0)
            out += "\nlistener {\n"
                + "    timeout = " + (Flags.idleSuspendMin * 60) + "\n"
                + "    on-timeout = systemctl suspend\n"
                + "}\n";

        return out;
    }

    /**
     * Night light goes through NightLight's own setters so the debounced conf
     * write and the live hyprsunset push stay in one place.
     */
    function _setNight(e, v) {
        if (e.key === "nightLightMode")
            NightLight.setMode(v);
        else if (e.key === "nightLightTemp")
            NightLight.setTemp(v);
        else if (e.key === "nightLightOnMin")
            NightLight.setOnMin(v);
        else if (e.key === "nightLightOffMin")
            NightLight.setOffMin(v);
        else
            return false;
        return true;
    }

    /**
     * Rewrites one decoration.lua field, top-level or scoped to its block, and
     * arms the debounced Hyprland reload. The two window-opacity fields also get
     * pushed through `hl.config`, which hits the REFRESH_WINDOW_STATES path so
     * windows that were not focused when the value changed do not keep a stale
     * alpha until their next focus change.
     */
    function _setDeco(id, e, v) {
        root._sync("deco");
        var lit = root._lit(e, v);
        var dot = e.key.indexOf(".");
        var res = dot < 0
            ? SetDeco.setField(root._decoText, e.key, lit)
            : SetDeco.setBlockField(root._decoText, e.key.slice(0, dot), e.key.slice(dot + 1), lit);
        if (!res.ok)
            return false;
        root._decoText = res.text;
        root._decoPending += 1;
        decoWriter.setText(res.text);
        root._reloadId = id;
        reloadTimer.restart();

        if (e.key === "active_opacity" || e.key === "inactive_opacity") {
            var ao = parseFloat(SetDeco.getField(root._decoText, "active_opacity"));
            var io = parseFloat(SetDeco.getField(root._decoText, "inactive_opacity"));
            opacityRefresh.command = ["hyprctl", "eval",
                "hl.config({ decoration = { active_opacity = " + (isNaN(ao) ? 1 : ao).toFixed(2)
                + ", inactive_opacity = " + (isNaN(io) ? 1 : io).toFixed(2) + " } })"];
            opacityRefresh.running = true;
        }
        return true;
    }

    /**
     * input.lua for a table field; env.lua (plus the autostart setcursor line
     * and a live hyprctl push) for the cursor pair.
     */
    function _setInput(id, e, v) {
        if (root._isEnvKey(e.key)) {
            if (e.key === "XCURSOR_THEME" || e.key === "XCURSOR_SIZE") {
                var theme = e.key === "XCURSOR_THEME" ? String(v)
                    : SetInput.getEnv(root._envText, "XCURSOR_THEME");
                var size = e.key === "XCURSOR_SIZE" ? v
                    : parseInt(SetInput.getEnv(root._envText, "XCURSOR_SIZE"), 10);
                if (!theme || theme.length === 0)
                    theme = Schema.settings.cursorTheme.def;
                if (isNaN(size) || size <= 0)
                    size = Schema.settings.cursorSize.def;
                return root._applyCursor(theme, size);
            }
            root._sync("env");
            var er = SetInput.setEnv(root._envText, e.key, String(v));
            if (!er.ok)
                return false;
            root._envText = er.text;
            root._envPending += 1;
            envWriter.setText(er.text);
            return true;
        }

        root._sync("input");
        var res = SetInput.setField(root._inputText, e.key, root._lit(e, v));
        if (!res.ok)
            return false;
        root._inputText = res.text;
        root._inputPending += 1;
        inputWriter.setText(res.text);
        root._reloadId = id;
        reloadTimer.restart();
        return true;
    }

    /**
     * Applies a cursor theme/size pair live, then persists it by rewriting the
     * XCURSOR/HYPRCURSOR env lines and the autostart setcursor call. Each rewrite
     * is independent, so a config missing one of the three lines still lands the
     * others.
     */
    function _applyCursor(theme, size) {
        root._sync("autostart");
        setcursorProc.command = ["hyprctl", "setcursor", theme, String(size)];
        setcursorProc.running = true;

        var env = root._envText;
        var e1 = SetInput.setEnv(env, "XCURSOR_THEME", theme);
        var e2 = SetInput.setEnv(e1.ok ? e1.text : env, "XCURSOR_SIZE", String(size));
        var e3 = SetInput.setEnv(e2.ok ? e2.text : (e1.ok ? e1.text : env), "HYPRCURSOR_SIZE", String(size));
        var any = e1.ok || e2.ok || e3.ok;
        if (any) {
            root._envText = e3.ok ? e3.text : (e2.ok ? e2.text : e1.text);
            root._envPending += 1;
            envWriter.setText(root._envText);
        }

        var auto = SetInput.setCursorLine(root._autostartText, theme, String(size));
        if (auto.ok) {
            root._autostartText = auto.text;
            root._autostartPending += 1;
            autostartWriter.setText(auto.text);
        }
        return any || auto.ok;
    }

    /**
     * animations.lua is a list of calls, not a config table, so each key has its
     * own rewrite: the master flag, a blanket retime of every leaf's speed, or
     * one named bezier's two control points carried as "x1,y1,x2,y2".
     */
    function _setAnim(id, e, v) {
        root._sync("anim");
        var res;
        if (e.key === "enabled") {
            res = SetAnim.setEnabled(root._animText, v ? "true" : "false");
        } else if (e.key === "speed") {
            res = SetAnim.setAllSpeeds(root._animText, String(Math.round(v * 1e6) / 1e6));
        } else {
            var p = String(v).split(",").map(function (x) { return parseFloat(x); });
            if (p.length !== 4 || p.some(function (x) { return isNaN(x); }))
                return false;
            res = SetAnim.setCurvePoints(root._animText, e.key,
                p[0].toFixed(2), p[1].toFixed(2), p[2].toFixed(2), p[3].toFixed(2));
        }
        if (!res.ok)
            return false;
        root._animText = res.text;
        root._animPending += 1;
        animWriter.setText(res.text);
        root._reloadId = id;
        reloadTimer.restart();
        return true;
    }

    // ── backing files ─────────────────────────────────────────────────────

    /**
     * A reader and a writer per file. The readers are `blockLoading` +
     * `blockAllReads` so a `reload()` immediately before a rewrite yields the
     * current disk contents synchronously; each writer counts its own in-flight
     * saves back down so `_sync` knows when the file on disk has caught up with
     * our text. A save that fails still has to decrement, or the guard would
     * latch and Store would never re-read that file again.
     */
    FileView { id: decoFile; path: root.decoPath; blockLoading: true; blockAllReads: true; printErrors: false }
    FileView {
        id: decoWriter
        path: root.decoPath
        atomicWrites: true
        printErrors: false
        onSaved: root._decoPending = Math.max(0, root._decoPending - 1)
        onSaveFailed: {
            root._decoPending = Math.max(0, root._decoPending - 1);
            root._saveFailed("decoration.lua");
        }
    }

    FileView { id: inputFile; path: root.inputPath; blockLoading: true; blockAllReads: true; printErrors: false }
    FileView {
        id: inputWriter
        path: root.inputPath
        atomicWrites: true
        printErrors: false
        onSaved: root._inputPending = Math.max(0, root._inputPending - 1)
        onSaveFailed: {
            root._inputPending = Math.max(0, root._inputPending - 1);
            root._saveFailed("input.lua");
        }
    }

    FileView { id: animFile; path: root.animPath; blockLoading: true; blockAllReads: true; printErrors: false }
    FileView {
        id: animWriter
        path: root.animPath
        atomicWrites: true
        printErrors: false
        onSaved: root._animPending = Math.max(0, root._animPending - 1)
        onSaveFailed: {
            root._animPending = Math.max(0, root._animPending - 1);
            root._saveFailed("animations.lua");
        }
    }

    FileView { id: envFile; path: root.envPath; blockLoading: true; blockAllReads: true; printErrors: false }
    FileView {
        id: envWriter
        path: root.envPath
        atomicWrites: true
        printErrors: false
        onSaved: root._envPending = Math.max(0, root._envPending - 1)
        onSaveFailed: {
            root._envPending = Math.max(0, root._envPending - 1);
            root._saveFailed("env.lua");
        }
    }

    FileView { id: autostartFile; path: root.autostartPath; blockLoading: true; blockAllReads: true; printErrors: false }
    FileView {
        id: autostartWriter
        path: root.autostartPath
        atomicWrites: true
        printErrors: false
        onSaved: root._autostartPending = Math.max(0, root._autostartPending - 1)
        onSaveFailed: {
            root._autostartPending = Math.max(0, root._autostartPending - 1);
            root._saveFailed("autostart.lua");
        }
    }

    /**
     * hypridle.conf is regenerated whole from Flags, so it needs no read-back and
     * no pending counter. The service restart hangs off `saved` so hypridle only
     * ever reloads a conf that actually landed.
     */
    FileView {
        id: idleWriter
        path: root.hypridlePath
        atomicWrites: true
        printErrors: false
        onSaved: hypridleRestart.running = true
        onSaveFailed: root._saveFailed("hypridle.conf")
    }

    /**
     * The Hyprland reload is debounced so a scrub drag writes the file on every
     * step but reloads once, and captured rather than detached so a failed reload
     * reaches the user through the same error strip as a refused write.
     */
    Timer {
        id: reloadTimer
        interval: 250
        repeat: false
        onTriggered: reloadProc.running = true
    }

    Process {
        id: reloadProc
        command: ["sh", "-c", "sleep 0.3; hyprctl reload"]
        onExited: function (exitCode) {
            if (exitCode !== 0)
                root.writeFailed(root._reloadId, "Hyprland reload failed. The change is saved but not applied.");
        }
    }

    Process { id: opacityRefresh; command: [] }
    Process { id: setcursorProc; command: [] }
    Process { id: hypridleRestart; command: ["systemctl", "--user", "restart", "hypridle"] }

    Component.onCompleted: root.reload()
}
