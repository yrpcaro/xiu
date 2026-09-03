// Xiu browser theme: the native host streams the shell's palette
// (~/.cache/ricelin/colors.json) and every message recolours the whole
// browser through the theme API — toolbar, tabs, urlbar, popups and the
// new-tab page all follow the wallpaper.

const port = browser.runtime.connectNative("io.github.yrpcaro.xiu");

port.onMessage.addListener((c) => {
    browser.theme.update({
        colors: {
            frame: c.surface,
            toolbar: c.surface_container,
            toolbar_text: c.cream,
            toolbar_field: c.surface_container_high,
            toolbar_field_text: c.cream,
            toolbar_field_focus: c.surface_container,
            toolbar_field_text_focus: c.cream,
            toolbar_field_border: c.outline_variant,
            toolbar_field_highlight: c.primary_container,
            toolbar_bottom_separator: c.outline_variant,
            tab_background_text: c.dim,
            tab_text: c.cream,
            tab_selected: c.surface_container,
            tab_line: c.primary,
            popup: c.surface_container,
            popup_text: c.cream,
            popup_border: c.outline_variant,
            sidebar: c.surface_container,
            sidebar_text: c.cream,
            ntp_background: c.surface,
            ntp_text: c.cream,
            bookmark_text: c.cream,
            button_background_hover: c.surface_container_high,
            button_background_active: c.surface_container_highest,
            icons: c.subtle,
            icons_attention: c.primary,
        },
    });
});

port.onDisconnect.addListener((p) => {
    console.error("xiu theme: native host disconnected", p.error);
});
