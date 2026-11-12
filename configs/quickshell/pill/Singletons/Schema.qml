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
        { id: "notifications", title: "Notifications", caption: "Pop-ups, dwell timing and muted apps", icon: "inbox" },
        { id: "defaultapps", title: "Default apps", caption: "Which app opens folders, images and documents", icon: "app-window" },
        { id: "user", title: "User & Session", caption: "Profile, session details, user overrides", icon: "user" },
        { id: "updates", title: "Updates", caption: "Version and check for updates", icon: "download" }
    ]

    /** Named groups per page, in the order they appear on screen. */
    readonly property var groupOrder: ({
        appearance: ["clock", "palette", "shell"],
        idlelock: ["idle"],
        notifications: ["media", "flashes", "appnotifs", "apps"],
        display: ["night"],
        input: ["cursor"],
        user: ["account"]
    })

    /** The visible title of each group in `groupOrder`. */
    readonly property var groupTitles: ({
        appearance: { clock: "Clock & widgets", palette: "Palette", shell: "Shell" },
        idlelock: { idle: "Idle" },
        notifications: { media: "Media", flashes: "Pill flashes", appnotifs: "App notifications", apps: "Apps" },
        display: { night: "Night light" },
        input: { cursor: "Cursor" },
        user: { account: "User settings" }
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
        weatherEnabled: {
            page: "appearance", group: "clock", order: 4,
            label: "Weather", caption: "Show weather in pill and calendar",
            control: "toggle", type: "bool", backend: "flags", key: "weatherEnabled", def: true
        },
        weatherCity: {
            page: "appearance", group: "clock", order: 5,
            label: "Weather town", caption: "The calendar's weather, by city name",
            control: "text", type: "string", backend: "flags", key: "weatherCity", def: ""
        },
        colorMode: {
            page: "appearance", group: "palette", order: 0,
            label: "Theme mode", caption: "Dark or light color scheme across the desktop",
            control: "seg", type: "string", backend: "flags", key: "colorMode", def: "dark",
            options: [
                { label: "Dark", value: "dark" },
                { label: "Light", value: "light" }
            ]
        },
        paletteMode: {
            page: "appearance", group: "palette", order: 1,
            label: "Palette", caption: "Wallpaper-driven, a fixed scheme, or a manual hue",
            control: "seg", type: "string", backend: "flags", key: "paletteMode", def: "static",
            options: [
                { label: "Dynamic", value: "dynamic" },
                { label: "Static", value: "static" },
                { label: "Manual", value: "manual" }
            ]
        },
        schemeVariant: {
            page: "appearance", group: "palette", order: 2,
            label: "Scheme variant", caption: "Color harmony and chroma curve (e.g. vibrant, tonal, expressive)",
            control: "seg", type: "string", backend: "flags", key: "schemeVariant", def: "auto"
        },
        smartScheme: {
            page: "appearance", group: "palette", order: 3,
            label: "Smart colors", caption: "Analyze wallpaper colourfulness to select matching scheme variant",
            control: "toggle", type: "bool", backend: "flags", key: "smartScheme", def: true
        },
        schemePreset: {
            page: "appearance", group: "palette", order: 4,
            label: "Scheme preset", caption: "Static curated palette preset (e.g. warm, nord, dracula)",
            control: "seg", type: "string", backend: "flags", key: "schemePreset", def: "dynamic"
        },
        manualHue: {
            page: "appearance", group: "palette", order: 5,
            label: "Manual hue", caption: "The strip picks the tone; this is its number",
            control: "custom", type: "int", backend: "flags", key: "manualHue", def: 30
        },
        manualSat: {
            page: "appearance", group: "palette", order: 6,
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
        iconWeight: {
            page: "appearance", group: "shell", order: 3,
            label: "Icon line weight", caption: "Multiplier on every icon's stroke width",
            control: "seg", type: "real", backend: "flags", key: "iconWeight", def: 1.0,
            options: [
                { label: "0.8×", value: 0.8 },
                { label: "1.0×", value: 1.0 },
                { label: "1.2×", value: 1.2 },
                { label: "1.4×", value: 1.4 }
            ]
        },

        // ── notifications ────────────────────────────────────────────────
        notifPlay: {
            page: "notifications", group: "media", order: 0,
            label: "Play and pause", caption: "When music starts or stops",
            control: "toggle", type: "bool", backend: "flags", key: "notifPlay", def: true
        },
        notifSong: {
            page: "notifications", group: "media", order: 1,
            label: "New song", caption: "When the track changes",
            control: "toggle", type: "bool", backend: "flags", key: "notifSong", def: true
        },
        notifVolume: {
            page: "notifications", group: "flashes", order: 0,
            label: "Volume", caption: "When you change the volume or mute",
            control: "toggle", type: "bool", backend: "flags", key: "notifVolume", def: true
        },
        notifBrightness: {
            page: "notifications", group: "flashes", order: 1,
            label: "Brightness", caption: "When the screen gets brighter or darker",
            control: "toggle", type: "bool", backend: "flags", key: "notifBrightness", def: true
        },
        notifWorkspace: {
            page: "notifications", group: "flashes", order: 2,
            label: "Workspace", caption: "The dots when you switch workspaces",
            control: "toggle", type: "bool", backend: "flags", key: "notifWorkspace", def: true
        },
        notifBattery: {
            page: "notifications", group: "flashes", order: 3,
            label: "Charging", caption: "When you plug in the charger",
            control: "toggle", type: "bool", backend: "flags", key: "notifBattery", def: true
        },
        notifRecord: {
            page: "notifications", group: "flashes", order: 4,
            label: "Screen recording", caption: "When a recording starts or stops",
            control: "toggle", type: "bool", backend: "flags", key: "notifRecord", def: true
        },
        notifOsdTime: {
            page: "notifications", group: "flashes", order: 5,
            label: "How long they stay", caption: "Scales every flash, the song card included",
            control: "seg", type: "real", backend: "flags", key: "osdTime", def: 1.0,
            options: [
                { label: "0.5×", value: 0.5 },
                { label: "0.75×", value: 0.75 },
                { label: "1×", value: 1.0 },
                { label: "1.5×", value: 1.5 },
                { label: "2×", value: 2.0 }
            ]
        },
        notifDnd: {
            page: "notifications", group: "flashes", order: 6,
            label: "Do not disturb", caption: "Also hides media, workspace, charging and recording",
            control: "toggle", type: "bool", backend: "flags", key: "dnd", def: false
        },
        notifSecs: {
            page: "notifications", group: "appnotifs", order: 0,
            label: "Show for", caption: "How long a new notification stays in the pill",
            control: "seg", type: "real", backend: "flags", key: "notifSecs", def: 4,
            options: [
                { label: "3 s", value: 3 },
                { label: "4 s", value: 4 },
                { label: "6 s", value: 6 },
                { label: "8 s", value: 8 },
                { label: "12 s", value: 12 }
            ]
        },
        notifApps: {
            page: "notifications", group: "apps", order: 0,
            label: "Apps", caption: "Choose which apps can show pill pop-ups",
            control: "custom", type: "string", backend: "flags", key: "notifMuted", def: ""
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
        },
        // ── user & session ────────────────────────────────────────────────
        userAvatar: {
            page: "user", group: "account", order: 0,
            label: "Profile picture", caption: "Set or remove user profile picture avatar",
            control: "custom", type: "string", backend: "none", key: "avatar", def: ""
        },
        userPassword: {
            page: "user", group: "account", order: 1,
            label: "Password", caption: "Change login and sudo password",
            control: "custom", type: "string", backend: "none", key: "passwd", def: ""
        },
        userShell: {
            page: "user", group: "account", order: 2,
            label: "Login shell", caption: "Change user login shell",
            control: "custom", type: "string", backend: "none", key: "shell", def: ""
        },
        userName: {
            page: "user", group: "account", order: 3,
            label: "Full name", caption: "Change user full name and account details",
            control: "custom", type: "string", backend: "none", key: "name", def: ""
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
