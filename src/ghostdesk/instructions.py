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
  Use `region={"x": 0, "y": 0, "width": 400, "height": 300}` to capture
  a specific area.
  Use `annotate=True` to get a version where every interactive element
  (buttons, links, input fields, icons…) is highlighted with a colored
  bounding box and a small label showing its center coordinates, e.g.
  `(320, 161)`. Those two numbers are the exact `x` and `y` to pass to
  `mouse_click(x=320, y=161)`. Use this whenever you need to click
  something — it removes all guesswork. Not needed for reading content
  or verifying navigation results.
  `format`: "png" (default) or "webp".
- **Click**: mouse_click(x, y) — left-click. Use `button` for "middle"
  or "right".
- **Double-click**: mouse_double_click(x, y) — for opening files or
  selecting words.
- **Drag**: mouse_drag(from_x, from_y, to_x, to_y) — for selecting
  text, moving items, or resizing.
- **Scroll**: mouse_scroll(x, y, direction="down", amount=3) —
  direction: up/down/left/right. amount: 1–5.
- **Type text**: mouse_click(x, y) on the field, then type_text("hello").
- **Press keys**: press_key("Return"), press_key("ctrl+v").
- **Launch apps**: launch("firefox https://example.com"),
  launch("gnome-terminal").
- **Clipboard**: get_clipboard() to read, set_clipboard(text) to write.
  Use set_clipboard + press_key("ctrl+v") for accented or special characters.
- **Switch windows**: press_key("alt+Tab") to cycle between open apps.
- **Inspect elements**: inspect() — returns structured JSON with all
  detected UI elements and their center coordinates, without an image.
  Faster than a screenshot when you only need to locate an element.
  Use `x, y, width, height` to inspect a specific region.
  Fall back to `screenshot(annotate=True)` if the target is not found.

## 3 rules

1. **screenshot() first.** Look at the image to understand what is on screen.

2. **Click what you see.** Estimate coordinates from the screenshot
   and use mouse_click(x, y). If the target is small or hard to locate,
   use `screenshot(annotate=True)` first to get labeled coordinates.

3. **screenshot() after navigation.** After clicking a link or
   submitting a form, call wait() to let the page load, then
   take a new screenshot to see the result.

## Standard workflow

```
screenshot()
# → image of the desktop
mouse_click(740, 218)
# → clicked a link
wait(2)
# → waited 2s for the page to load
screenshot()
# → page loaded, continue
```
"""
