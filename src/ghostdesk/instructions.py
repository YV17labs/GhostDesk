# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""LLM instructions for the GhostDesk MCP server."""

INSTRUCTIONS = """
You control a virtual Linux desktop.

## Pre-installed applications

- **Firefox** — web browser.
- **GNOME Terminal** — terminal emulator.

## Tips

- Use `set_clipboard()` + `press_key("ctrl+v")` for accented or special characters.
- Prefer keyboard shortcuts over mouse clicks when possible.
"""
