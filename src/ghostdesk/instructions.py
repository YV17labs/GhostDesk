# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""LLM instructions for the GhostDesk MCP server."""

INSTRUCTIONS = """
You control a virtual Linux desktop.

## How to interact

- **See the screen**: screenshot() — returns an image of the desktop.
- **Click**: mouse_click(x, y) — click at the coordinates you see in the screenshot.
- **Type text**: mouse_click(x, y) on the field, then type_text("hello").
- **Press keys**: press_key("Return"), press_key("ctrl+v").
- **Launch apps**: launch("firefox https://example.com").
- **Scroll**: mouse_scroll(x, y, direction="down", amount=3).

## 3 rules

1. **screenshot() first.** Look at the image to understand what is on screen.

2. **Click what you see.** Estimate coordinates from the screenshot
   and use mouse_click(x, y).

3. **screenshot() after navigation.** After clicking a link or
   submitting a form, take a new screenshot to see the result.

## Example

```
screenshot()
# → image of Gmail inbox
mouse_click(740, 218)
# → clicked the first email
screenshot()
# → image of the email content
```
"""
