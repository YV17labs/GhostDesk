# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Window information utilities (best-effort via xdotool)."""

from __future__ import annotations

import asyncio

from ghostdesk.utils.cmd import run


async def get_active_window() -> str | None:
    """Return just the active window title (cheap — 2 xdotool calls)."""
    try:
        wid = (await run(["xdotool", "getactivewindow"])).strip()
        name = (await run(["xdotool", "getwindowname", wid])).strip()
        return name or None
    except Exception:
        return None


async def get_window_info() -> dict:
    """Return active window title and list of visible windows.

    All fields are best-effort — returns ``None`` / empty list on failure
    (e.g. no window focused, bare desktop).
    """
    info: dict = {"active_window": await get_active_window(), "windows": []}

    # Visible windows — fetch name+geometry in parallel per window
    try:
        wids_raw = await run(["xdotool", "search", "--onlyvisible", "--name", ""])
        wids = [w.strip() for w in wids_raw.strip().splitlines() if w.strip()]

        async def _get_window(wid: str) -> dict | None:
            try:
                name, geo_raw = await asyncio.gather(
                    run(["xdotool", "getwindowname", wid]),
                    run(["xdotool", "getwindowgeometry", wid]),
                )
                name = name.strip()
                if not name:
                    return None
                return {"title": name, **_parse_geometry(geo_raw)}
            except Exception:
                return None

        results = await asyncio.gather(*[_get_window(wid) for wid in wids])
        info["windows"] = [w for w in results if w is not None]
    except Exception:
        pass

    return info


def _parse_geometry(raw: str) -> dict:
    """Parse xdotool getwindowgeometry output into a dict."""
    result: dict = {"x": 0, "y": 0, "width": 0, "height": 0}
    for line in raw.splitlines():
        line = line.strip()
        if line.startswith("Position:"):
            # "Position: 10,20 (screen: 0)"
            pos = line.split(":")[1].split("(")[0].strip()
            parts = pos.split(",")
            result["x"] = int(parts[0])
            result["y"] = int(parts[1])
        elif line.startswith("Geometry:"):
            # "Geometry: 800x600"
            dims = line.split(":")[1].strip()
            w, h = dims.split("x")
            result["width"] = int(w)
            result["height"] = int(h)
    return result
