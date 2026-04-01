# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Open window listing via xdotool."""

from __future__ import annotations

import asyncio

from ghostdesk.utils.cmd import run


async def _parse_wm_class(wid: str) -> str:
    """Extract the application name from WM_CLASS (second value)."""
    try:
        output = await run(["xprop", "-id", wid, "WM_CLASS"])
        # WM_CLASS(STRING) = "instance", "class"
        parts = output.split('"')
        if len(parts) >= 4:
            return parts[3].lower()
    except Exception:
        pass
    return ""


async def _window_info(wid: str) -> dict | None:
    """Return app name, title and geometry for a window ID, or None on failure."""
    try:
        name, geom, app = await asyncio.gather(
            run(["xdotool", "getwindowname", wid]),
            run(["xdotool", "getwindowgeometry", "--shell", wid]),
            _parse_wm_class(wid),
        )
        title = name.strip()
        if not title:
            return None
        props = dict(line.split("=", 1) for line in geom.splitlines() if "=" in line)
        return {
            "app": app or "unknown",
            "title": title,
            "x": int(props["X"]),
            "y": int(props["Y"]),
            "width": int(props["WIDTH"]),
            "height": int(props["HEIGHT"]),
        }
    except Exception:
        return None


async def get_open_windows() -> list[dict]:
    """Return visible windows with title and geometry."""
    try:
        output = await run(
            ["xdotool", "search", "--onlyvisible", "--name", ".+"],
        )
        wids = [w for line in output.splitlines() if (w := line.strip())]
        results = await asyncio.gather(*(_window_info(wid) for wid in wids))
        return [r for r in results if r is not None]
    except Exception:
        return []
