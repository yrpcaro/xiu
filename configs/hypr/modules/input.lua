hl.config({
    input = {
        kb_layout          = "us,ir",
        kb_variant         = ",winkeys",
        kb_options         = "grp:alt_shift_toggle",
        follow_mouse       = 1,
        sensitivity        = 0,
        accel_profile      = "flat",
        repeat_rate        = 40,
        repeat_delay       = 400,
        numlock_by_default = false,
        touchpad = {
            natural_scroll = true,
            tap_to_click   = true,
        },
    },
    cursor = {
        no_hardware_cursors = true,
    },
    binds = {
        scroll_event_delay = 40,
    },
})
