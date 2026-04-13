#!/command/with-contenv sh
set -eu

# Sway config for the headless ghostdesk container.
#
# The desktop is driven over D-Bus / IPC by an LLM agent, not by a human,
# but we still render a branded wallpaper and a minimal status bar so
# screenshots returned to the agent feel like a real desktop.  Colours
# are picked from assets/wallpaper_1280x1024.png:
#
#   background  #1a1b2e  (deep indigo night sky)
#   violet      #7b68ee  (ghost body, primary accent)
#   violet soft #8879d9  (subtitle / inactive text)
#   teal        #66d9c6  (terminal lines inside the ghost)
#   white       #ffffff  (window titles)

CFG_DIR="/home/vscode/.config/sway"
install -d -m 0755 -o vscode -g vscode "${CFG_DIR}"

cat > "${CFG_DIR}/config" <<'EOF'
# Sway headless config — ghostdesk POC
# wlroots creates a virtual HEADLESS-1 output because WLR_HEADLESS_OUTPUTS=1
output HEADLESS-1 resolution 1280x1024 position 0,0
output HEADLESS-1 bg /usr/share/ghostdesk/wallpaper.png fill #1a1b2e

font pango:DejaVu Sans 10
default_border none
default_floating_border none
gaps inner 0
gaps outer 0

# XDG desktop portal expects this env propagated to systemd --user scope.
# No systemd here, so we rely on s6 passing env via with-contenv instead.

# ---------- Status bar ----------
# Minimal bar at the bottom — date/time only, no workspaces (we run a
# single fixed workspace).  Colours match the ghostdesk wallpaper.
bar {
    position bottom
    status_command while date +'%Y-%m-%d  %H:%M'; do sleep 30; done
    font pango:DejaVu Sans 10

    colors {
        background        #1a1b2eee
        statusline        #ffffff
        separator         #8879d9

        #                  border     background  text
        focused_workspace  #7b68ee    #7b68ee     #ffffff
        active_workspace   #8879d9    #1a1b2e     #ffffff
        inactive_workspace #1a1b2e    #1a1b2e     #8879d9
        urgent_workspace   #ff5c8a    #ff5c8a     #ffffff
        binding_mode       #7b68ee    #7b68ee     #ffffff
    }
}
EOF

chown vscode:vscode "${CFG_DIR}/config"
