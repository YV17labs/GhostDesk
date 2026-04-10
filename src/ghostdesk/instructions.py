# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""LLM instructions for the GhostDesk MCP server."""

INSTRUCTIONS = """
You control a virtual Linux desktop via screen capture, mouse, and keyboard.
Pre-installed apps: **Firefox**, **GNOME Terminal**.

Each tool's own docstring covers its parameters, return shape, and per-tool
caveats. The notes below are only the cross-tool orchestration patterns the
docstrings can't express.

## Clicking on screen

1. **Locate.** `screenshot()` to see the desktop and find your target
   visually.
2. **Resolve coordinates.** For small or dense targets, crop to the
   area with `screenshot(region=Region(x, y, w, h))` and read the
   center pixel directly from the image. Note that this is a true
   crop at native screen resolution — pixels are not enlarged or
   interpolated, you simply receive a smaller sub-rectangle of the
   framebuffer. The benefit is fewer pixels to scan and no visual
   distractors, not extra detail. Size the region with a comfortable
   margin around your estimated target — don't crop tight on the
   pixel you think it's at. If your initial guess is off by 30
   pixels and you captured a 40×40 box, the target won't be in the
   crop at all and you'll have to start over. A region a few times
   larger than the target absorbs aiming error and still gives you
   plenty of room to read the exact center.

   On top of a crop, pass `grid=True` to draw a coordinate ruler in
   the margins of the returned image: the top strip labels the X
   axis every 50 px, the left strip labels the Y axis every 20 px,
   and thin alternating magenta/cyan gridlines overlay the content.
   Labels use *absolute screen coordinates* even on a crop, so you
   can read the click point straight off the rulers instead of
   estimating pixel offsets. Handy for smaller vision models that
   struggle to count pixels on their own. Only combine `grid=True`
   with `region=` — on a full-screen capture the ruler is visually
   overwhelming and wastes tokens.
3. **Click, then verify.** After the action, re-screenshot and confirm the
   UI actually reacted as intended. For destructive actions (delete, send,
   close) don't trust `screen_changed: true` alone — inspect the pixels.

If a click misses (`screen_changed: false`), do not retry the same
coordinates. Re-crop the target area and pick fresh coordinates from the
new capture.

## Keyboard

- Prefer keyboard shortcuts over clicks when both are available — they're
  more reliable than coordinate-based clicks.
- Accents and special characters: `set_clipboard()` then
  `press_key("ctrl+v")`. `type_text` may not handle them on every layout.
"""
