# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""LLM instructions for the GhostDesk MCP server."""

INSTRUCTIONS = """
You control a virtual Linux desktop via screen capture, mouse, and keyboard.
Call `app_list()` to see which GUI apps are currently installed.
Do this before choosing which app to use, or after installing new software.

Each tool's own docstring covers its parameters, return shape, and per-tool
caveats. The notes below are only the cross-tool orchestration patterns the
docstrings can't express.

## Verifying clicks

For destructive actions (delete, send, close) don't trust
`screen_changed: true` alone — re-screenshot and inspect the pixels.

If a click misses (`screen_changed: false`), do not retry the same
coordinates — take a new `screen_shot()` and pick fresh coordinates.
"""
