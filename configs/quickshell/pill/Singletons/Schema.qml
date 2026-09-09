pragma Singleton

import QtQuick
import Quickshell

/**
 * SCHEMA: the single declarative description of every user-facing setting the
 * shell owns, ported from CapsuleOS's design. It is metadata only — no reads,
 * no writes, no UI. Store routes a write by `backend`/`key`, the pages read
 * `label`/`caption`/`options`/`def`, and the settings index search reads the
 * same table, so a setting is described exactly once.
 *
 * `settings` is keyed by a stable camelCase id. Every entry carries where it
 * lives (`page`, `group`, `order`), how it reads (`label`, `caption`), how it
 * is driven (`control`), and where it is stored (`type`, `backend`, `key`,
 * `def`). Scrubs add `from`/`to`/`step`/`unit`; segs add `options`.
 *
 * `def` is the value the shipped source produces — Flags.qml's JsonAdapter
 * for `flags`/`idle` keys, the repo's hypr Lua modules for `deco`/`input`. It
 * is a fallback for a missing field, never an overwrite of a stored value.
 *
 * `control: "custom"` marks a setting whose UI is bespoke (the monitor card,
 * the layout picker, the hue strip, the keybind editor). Those carry
 * page-level metadata so the search can find them and route to their page;
 * they are not Store-routed.
 *
 * Migration state: the framework (this table, Store, the row/nav components
 * and the searchable index) landed in one piece; the pages move onto it a
 * group at a time. Appearance's flag-backed rows are on it; everything else
 * still owns its widgets, which is exactly what `custom` describes.
 */
Singleton {
    id: root

    /** The settings index: the settings surface is a Repeater over this. */
    readonly property var pages: [
        { id: "appearance", title: "Appearance", caption: "Clock, glyphs, accent palette", icon: "sparkles" },
        { id: "look", title: "Look", caption: "Gaps, rounding, blur, opacity", icon: "app-window" },
        { id: "display", title: "Display", caption: "Resolution, refresh, scale, night light", icon: "monitor" },
        { id: "input", title: "Input", caption: "Pointer, keyboard, layouts, cursor", icon: "mouse" },
        { id: "animation", title: "Animation", caption: "Speed, motion curve, enable", icon: "waves" },
        { id: "keybinds", title: "Keybinds", caption: "Rebind, add, set commands", icon: "keyboard" },
        { id: "workspaces", title: "Workspaces", caption: "Special spaces and their keys", icon: "layers" },
        { id: "idlelock", title: "Idle / Lock", caption: "Auto-lock, screen off, suspend", icon: "lock" },
        { id: "updates", title: "Updates", caption: "Version and check for updates", icon: "download" },
        { id: "weather", title: "Weather", caption: "Forecast, city, and the source behind it", icon: "cloud" },
        { id: "defaultapps", title: "Default apps", caption: "Which app opens folders, images and documents", icon: "app-window" }
    ]

    /** Named groups per page, in the order they appear on screen. */
    readonly property var groupOrder: ({
        appearance: ["clock", "palette", "shell"],
        idlelock: ["idle"],
        display: ["night"],
        input: ["cursor"]
    })

    /** The visible title of each group in `groupOrder`. */
    readonly property var groupTitles: ({
        appearance: { clock: "Clock & widgets", palette: "Palette", shell: "Shell" },
        idlelock: { idle: "Idle" },
        display: { night: "Night light" },
        input: { cursor: "Cursor" }
    })

    /** The visible title of one group, or "" for an ungrouped entry. */
    function groupTitle(page, group) {
        var t = root.groupTitles[page];
        return t && t[group] ? t[group] : "";
    }

    /**
     * Every setting. The order within a page comes from `order`; groups come
     * from `group` + `groupOrder`.
     */
    readonly property var settings: ({
        // ── appearance ────────────────────────────────────────────────────
        time12h: {
            page: "appearance", group: "clock", order: 0,
            label: "Time format", caption: "12-hour clock instead of 24-hour",
            control: "toggle", type: "bool", backend: "flags", key: "time12h", def: false
        },
        clockSeconds: {
            page: "appearance", group: "clock", order: 1,
            label: "Clock seconds", caption: "Show the seconds in the pill clock",
            control: "toggle", type: "bool", backend: "flags", key: "clockSeconds", def: false
        },
        showGlyphs: {
            page: "appearance", group: "clock", order: 2,
            label: "Japanese glyphs", caption: "The kanji labels across the shell",
            control: "toggle", type: "bool", backend: "flags", key: "showGlyphs", def: true
        },
        musicViz: {
            page: "appearance", group: "clock", order: 3,
            label: "Music visualizer", caption: "The media strip's cover art and thread",
            control: "toggle", type: "bool", backend: "flags", key: "musicViz", def: true
        },
        weatherCity: {
            page: "appearance", group: "clock", order: 4,
            label: "Weather town", caption: "The calendar's weather, by city name",
            control: "text", type: "string", backend: "flags", key: "weatherCity", def: ""
        },
        paletteMode: {
            page: "appearance", group: "palette", order: 0,
            label: "Palette", caption: "Wallpaper-driven, a fixed scheme, or a manual hue",
            control: "seg", type: "string", backend: "flags", key: "paletteMode", def: "static",
            options: [
                { label: "Dynamic", value: "dynamic" },
                { label: "Static", value: "static" },
                { label: "Manual", value: "manual" }
            ]
        },
        manualHue: {
            page: "appearance", group: "palette", order: 1,
            label: "Manual hue", caption: "The strip picks the tone; this is its number",
            control: "custom", type: "int", backend: "flags", key: "manualHue", def: 30
        },
        manualSat: {
            page: "appearance", group: "palette", order: 2,
            label: "Manual saturation", caption: "How much color the manual tone carries",
            control: "custom", type: "real", backend: "flags", key: "manualSat", def: 0.5
        },
        uiScale: {
            page: "appearance", group: "shell", order: 0,
            label: "UI scale", caption: "Everything the shell draws, as a multiple",
            control: "scrub", type: "real", backend: "flags", key: "uiScale", def: 1.0,
            from: 0.75, to: 1.5, step: 0.05
        },
        reduceMotion: {
            page: "appearance", group: "shell", order: 1,
            label: "Reduce motion", caption: "Settle surfaces instead of morphing them",
            control: "toggle", type: "bool", backend: "flags", key: "reduceMotion", def: false
        },
        uiFont: {
            page: "appearance", group: "shell", order: 2,
            label: "Font", caption: "The shell's UI typeface",
            control: "custom", type: "string", backend: "flags", key: "uiFont", def: ""
        },

        // ── look ──────────────────────────────────────────────────────────
        topGap: {
            page: "look", group: "", order: 0,
            label: "Top gap", caption: "The pill's margin off the screen edge",
            control: "scrub", type: "real", backend: "flags", key: "topGap", def: 1.0,
            from: 0, to: 2, step: 0.1
        },
        appGap: {
            page: "look", group: "", order: 1,
            label: "App gap", caption: "The band between the pill and your windows",
            control: "scrub", type: "real", backend: "flags", key: "appGap", def: 1.0,
            from: 0, to: 1, step: 0.1
        },
        pillOpacity: {
            page: "look", group: "", order: 2,
            label: "Pill opacity", caption: "How opaque the pill body renders",
            control: "scrub", type: "real", backend: "flags", key: "pillOpacity", def: 1.0,
            from: 0.5, to: 1, step: 0.05
        },
        pillBlur: {
            page: "look", group: "", order: 3,
            label: "Pill blur", caption: "Blur what shows through the pill body",
            control: "toggle", type: "bool", backend: "flags", key: "pillBlur", def: false
        },
        decoShape: {
            page: "look", group: "", order: 4,
            label: "Window shape", caption: "Rounding, borders and the blur pass",
            control: "custom", type: "string", backend: "deco", key: "", def: ""
        },

        // ── display ───────────────────────────────────────────────────────
        monitors: {
            page: "display", group: "", order: 0,
            label: "Monitors", caption: "Resolution, refresh, scale per output",
            control: "custom", type: "string", backend: "app", key: "", def: ""
        },
        nightLightMode: {
            page: "display", group: "night", order: 0,
            label: "Mode", caption: "Off, always on, or on after dark",
            control: "seg", type: "string", backend: "flags", key: "nightLightMode", def: "off",
            options: [
                { label: "Off", value: "off" },
                { label: "On", value: "on" },
                { label: "Auto", value: "auto" }
            ]
        },
        nightLightTemp: {
            page: "display", group: "night", order: 1,
            label: "Warmth", caption: "How warm the night light runs, in Kelvin",
            control: "scrub", type: "int", backend: "flags", key: "nightLightTemp", def: 4000,
            from: 2500, to: 6500, step: 100, unit: "K"
        },

        // ── input ─────────────────────────────────────────────────────────
        layouts: {
            page: "input", group: "", order: 0,
            label: "Keyboard layouts", caption: "The multi-select picker and Alt+Shift order",
            control: "custom", type: "string", backend: "input", key: "kb_layout", def: "us,ir"
        },
        cursorTheme: {
            page: "input", group: "cursor", order: 0,
            label: "Cursor theme", caption: "The theme name, applied live",
            control: "text", type: "string", backend: "input", key: "XCURSOR_THEME", def: "Bibata-Modern-Ice"
        },
        cursorSize: {
            page: "input", group: "cursor", order: 1,
            label: "Cursor size", caption: "Applied live and persisted to the session",
            control: "scrub", type: "int", backend: "input", key: "XCURSOR_SIZE", def: 24,
            from: 12, to: 48, step: 2
        },

        // ── animation ─────────────────────────────────────────────────────
        animationsEnabled: {
            page: "animation", group: "", order: 0,
            label: "Enabled", caption: "The master animation switch",
            control: "toggle", type: "bool", backend: "anim", key: "enabled", def: true
        },
        animationSpeed: {
            page: "animation", group: "", order: 1,
            label: "Speed", caption: "Retimes every animation leaf",
            control: "scrub", type: "real", backend: "anim", key: "speed", def: 1.0,
            from: 0.5, to: 2, step: 0.05
        },
        motionPreset: {
            page: "animation", group: "", order: 2,
            label: "Feel", caption: "The curve presets and their editor",
            control: "custom", type: "string", backend: "anim", key: "", def: ""
        },

        // ── keybinds / workspaces ─────────────────────────────────────────
        keybinds: {
            page: "keybinds", group: "", order: 0,
            label: "Keybinds", caption: "Rebind, add, set commands",
            control: "custom", type: "string", backend: "app", key: "", def: ""
        },
        workspaces: {
            page: "workspaces", group: "", order: 0,
            label: "Special spaces", caption: "The stash and workspace rules",
            control: "custom", type: "string", backend: "app", key: "", def: ""
        },

        // ── idlelock ──────────────────────────────────────────────────────
        idleLockMin: {
            page: "idlelock", group: "idle", order: 0,
            label: "Auto-lock", caption: "Minutes idle before the lock screen; 0 is off",
            control: "scrub", type: "int", backend: "idle", key: "idleLockMin", def: 5,
            from: 0, to: 60, step: 1, unit: "min"
        },
        idleScreenOffMin: {
            page: "idlelock", group: "idle", order: 1,
            label: "Screen off", caption: "Minutes idle before the display blanks; 0 is off",
            control: "scrub", type: "int", backend: "idle", key: "idleScreenOffMin", def: 6,
            from: 0, to: 60, step: 1, unit: "min"
        },
        idleSuspendMin: {
            page: "idlelock", group: "idle", order: 2,
            label: "Suspend", caption: "Minutes idle before suspend; 0 is off. Two-step confirmed",
            control: "scrub", type: "int", backend: "idle", key: "idleSuspendMin", def: 0,
            from: 0, to: 60, step: 1, unit: "min", danger: true
        },

        // ── updates ───────────────────────────────────────────────────────
        updates: {
            page: "updates", group: "", order: 0,
            label: "Updates", caption: "Version and check for updates",
            control: "custom", type: "string", backend: "app", key: "", def: ""
        },

        // ── recording (the recorder drawer owns the UI; the values are flags) ──
        recordFps: {
            page: "input", group: "", order: 3, hidden: true,
            label: "Record FPS", caption: "The recorder's frame rate",
            control: "seg", type: "int", backend: "flags", key: "recordFps", def: 60,
            options: [{ label: "30", value: 30 }, { label: "60", value: 60 }]
        },
        autoHide: {
            page: "look", group: "", order: 5,
            label: "Auto-hide pill", caption: "The pill retracts off the top edge; dwell the edge to reveal",
            control: "toggle", type: "bool", backend: "flags", key: "autoHide", def: false
        },
        autoHideDelay: {
            page: "look", group: "", order: 6,
            label: "Auto-hide timing", caption: "How long the reveal dwell and retract linger run",
            control: "seg", type: "string", backend: "flags", key: "autoHideDelay", def: "medium",
            options: [
                { label: "Off", value: "off" },
                { label: "Short", value: "short" },
                { label: "Medium", value: "medium" },
                { label: "Long", value: "long" }
            ]
        }
    })

    /**
     * The settings of one page in display order (group order first, then
     * `order`), or every setting when no page is given.
     */
    function entriesFor(page) {
        var ids = Object.keys(root.settings);
        var out = [];
        for (var i = 0; i < ids.length; i++) {
            var e = root.settings[ids[i]];
            if (e.hidden)
                continue;
            if (page === undefined || e.page === page)
                out.push(e);
        }
        var groups = root.groupOrder[page] || [];
        out.sort(function (a, b) {
            var ga = groups.indexOf(a.group), gb = groups.indexOf(b.group);
            if (ga < 0) ga = groups.length;
            if (gb < 0) gb = groups.length;
            if (ga !== gb)
                return ga - gb;
            return a.order - b.order;
        });
        return out;
    }

    /** Complain loudly at startup about structural mistakes in the table. */
    function check() {
        var ids = Object.keys(root.settings);
        for (var i = 0; i < ids.length; i++) {
            var e = root.settings[ids[i]];
            if (e.page.length === 0)
                console.warn("Schema: '" + ids[i] + "' has no page");
            if (e.group.length > 0 && !root.groupTitle(e.page, e.group))
                console.warn("Schema: group '" + e.group + "' of '" + ids[i] + "' has no title");
        }
        var pages = {};
        for (var j = 0; j < root.pages.length; j++)
            pages[root.pages[j].id] = true;
        for (var k = 0; k < ids.length; k++)
            if (!pages[root.settings[ids[k]].page])
                console.warn("Schema: '" + ids[k] + "' names page '" + root.settings[ids[k]].page
                    + "', which is not in the index");
    }

    Component.onCompleted: check()
}
