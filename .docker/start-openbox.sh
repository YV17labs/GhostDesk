#!/bin/bash
# Wait for Xvfb to be fully ready (not just the socket file — actual X connection).
while ! xdpyinfo -display "${DISPLAY}" >/dev/null 2>&1; do
    sleep 0.1
done

# Set wallpaper BEFORE Openbox starts, so the WM never overrides it.
hsetroot -fill /usr/share/ghostdesk/wallpaper.png

exec openbox-session
