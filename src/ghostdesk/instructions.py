# Copyright (c) 2026 YV17 — MIT License
"""LLM instructions for the GhostDesk MCP server."""

INSTRUCTIONS = """
You control a virtual Linux desktop (X11, display :99, resolution 1280×800).

Tools are split into two channels: **accessibility** (fast, structured, no vision
cost) and **devices** (human-like mouse/keyboard/screenshots, stealth).

## Rules

1. Always start with accessibility: read_screen().
2. Switch to devices only when accessibility fails (empty results, bot
   detection, CAPTCHAs, canvas, or visual verification needed).
3. Never screenshot() to read text — read_screen() is free.
4. Prefer set_value() over focus+type_text over click+type_text.
5. Use wait_for_element() after launch(), never wait().
6. Verify after every action with read_screen().
7. For long text: set_clipboard() then press_key("ctrl+v").
8. On error: retry once, then switch channel.
9. On "Session not found": stop and tell the user.

## Examples

```
# Accessibility (default)
launch("firefox https://example.com")
wait_for_element("example", timeout_seconds=15)
read_screen()
click_element("Log in")
set_value("Email", "user@example.com")
click_element("Submit")
read_screen()

# Devices (fallback when accessibility fails)
launch("firefox https://protected-site.com")
wait(3000)
screenshot()
mouse_click(640, 400)
type_text("search query")
press_key("Return")
screenshot()
```
"""
