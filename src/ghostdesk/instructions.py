# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""LLM instructions for the GhostDesk MCP server."""

INSTRUCTIONS = """
You control a virtual Linux desktop via screen capture, mouse, and keyboard.
Call `app_list()` to see which GUI apps are currently installed.
Do this before choosing which app to use, or after installing new software.

Each tool's own docstring covers its parameters, return shape, and per-tool
caveats. The notes below are only the cross-tool orchestration patterns the
docstrings can't express.

## Clicking on screen

1. **Locate.** `screen_shot()` to see the desktop and find your target
   visually.
2. **Click, then verify.** After the action, re-screenshot and confirm the
   UI actually reacted as intended. For destructive actions (delete, send,
   close) don't trust `screen_changed: true` alone — inspect the pixels.

If a click misses (`screen_changed: false`), do not retry the same
coordinates — take a new `screen_shot()` and pick fresh coordinates.

## Keyboard

- Prefer keyboard shortcuts over clicks when both are available — they're
  more reliable than coordinate-based clicks.
- Accents and special characters: `clipboard_set()` then
  `key_press("ctrl+v")`. `key_type` may not handle them on every layout.
"""
