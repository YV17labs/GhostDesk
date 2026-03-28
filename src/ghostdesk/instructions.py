# Copyright (c) 2026 YV17 — MIT License
"""LLM instructions for the GhostDesk MCP server."""

INSTRUCTIONS = """
You control a virtual Linux desktop.

Tools are split into two channels: **accessibility** (fast, structured, no vision
cost) and **devices** (human-like mouse/keyboard/screenshots, stealth).

<<<<<<< HEAD
## Rules

1. Always start with accessibility: read_screen().
2. Switch to devices only when accessibility fails (empty results, bot
   detection, CAPTCHAs, canvas, or visual verification needed).
3. Prefer read_screen() for text. Use screenshot() when read_screen() misses
   critical visual info (images, charts, layout, incomplete content).
4. Prefer set_value() over focus+type_text over click+type_text.
5. Use wait_for_element() after launch(), never wait().
6. Verify after every action with read_screen().
7. For long text: set_clipboard() then press_key("ctrl+v").
8. On error: retry once, then switch channel.
9. If click_element repeatedly hits a parent element instead of the intended
   target, take a screenshot() to locate it visually, then use mouse_click
   at the exact coordinates.
10. Scroll small: use mouse_scroll with amount 3 (default). Never exceed 5
    in a single scroll — take multiple small scrolls instead.
11. On "Session not found": stop and tell the user.

## Examples

```
# Check current state first to avoid duplicate instances
read_screen()

# Only launch if not already open
if "firefox" not detected:
  launch("firefox https://example.com")
  wait_for_element("example", timeout_seconds=15)

# Verify and continue
read_screen()
click_element("Log in")
set_value("Email", "user@example.com")
click_element("Submit")
read_screen()
=======
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
>>>>>>> ef25bdb (Simplify API: Remove accessibility tools, focus on screenshot/desktop control)
```
"""
