local programs = require("modules/programs")

hl.on("hyprland.start", function ()
    hl.exec_cmd("awww-daemon")
    hl.exec_cmd("dbus-update-activation-environment --systemd --all")
    hl.exec_cmd("starshell")
    hl.exec_cmd("xhost +SI:localuser:root")
    hl.exec_cmd("hyprpm reload")
end)
