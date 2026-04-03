# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""LLM instructions for the GhostDesk MCP server."""

INSTRUCTIONS = """
You control a virtual Linux desktop.

## Pre-installed applications

- **Firefox** — web browser.
- **GNOME Terminal** — terminal emulator. Use it to run shell commands
  (launch it, click on the window, type_text("your command"), press_key("Return")).

## How to interact

- **See the screen**: screenshot() — returns an image of the desktop.
  Use x, y, width, height to capture a specific region.
  Use annotate=True to overlay detected UI elements with coordinate
  labels — useful when you need precise click coordinates.
- **Read the screen**: inspect() — returns structured JSON with
  cursor position, open windows, and every visible UI element with
  its click coordinates. No image.
- **Click**: mouse_click(x, y) — click at the given coordinates.
- **Type text**: mouse_click(x, y) on the field, then type_text("hello").
- **Press keys**: press_key("Return"), press_key("ctrl+v").
- **Launch apps**: launch("firefox https://example.com"), launch("gnome-terminal").
- **Scroll**: mouse_scroll(x, y, direction="down", amount=3).
- **Switch windows**: press_key("alt+Tab") to cycle between open applications.

## 3 rules

1. **screenshot() first.** Look at the image to understand what is on screen.

2. **Click what you see.** Estimate coordinates from the screenshot
   and use mouse_click(x, y).

3. **screenshot() after navigation.** After clicking a link or
   submitting a form, take a new screenshot to see the result.

## Text-only mode (when vision is limited)

Use inspect() instead of screenshot() to get element coordinates
without needing to interpret an image:

```
inspect()
# → {"cursor": {"x": 640, "y": 400},
#    "windows": [{"app": "firefox", "title": "Gmail", ...}],
#    "elements": [
#      {"role": "button", "x": 134, "y": 117, "label": "Gmail"},
#      {"role": "text", "x": 383, "y": 297, "label": "Stripe"},
#      ...
#    ]}
mouse_click(383, 297)
# → clicked on "Stripe"
inspect()
# → updated element list
```

## Standard workflow

```
screenshot()
# → image of the desktop
mouse_click(740, 218)
# → clicked a UI element
screenshot()
# → image of the updated screen
```
"""
