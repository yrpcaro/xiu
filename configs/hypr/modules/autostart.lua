hl.on("hyprland.start", function()
    hl.exec_cmd("gnome-keyring-daemon --start --components=secrets")
    hl.exec_cmd("xiu clipboard watch")
    hl.exec_cmd("xiu wallpaper init")
    hl.exec_cmd("xiu resizer --daemon")
    hl.exec_cmd("hyprctl setcursor Bibata-Modern-Ice 24")
    hl.exec_cmd("xiu cursor sync")
    hl.exec_cmd("systemctl --user start hyprland-session.target")
    hl.exec_cmd("systemctl --user start hyprpolkitagent")
    hl.exec_cmd("xiu watchdog pill")
    hl.exec_cmd("xiu watchdog lock")
    hl.exec_cmd("systemctl --user restart hypridle")

    -- Monthly trash cleanup (trash-cli). Off by default: uncomment to have
    -- the trash emptied of anything older than 30 days at every login.
    -- hl.exec_cmd("trash-empty 30")
end)
