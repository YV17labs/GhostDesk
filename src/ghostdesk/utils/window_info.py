# Copyright (c) 2026 YV17 — MIT License
"""Window information utilities (best-effort via xdotool)."""

from __future__ import annotations

from ghostdesk.utils.xdotool import run


async def get_window_info() -> dict:
    """Return active window title and list of visible windows.

    All fields are best-effort — returns ``None`` / empty list on failure
    (e.g. no window focused, bare desktop).
    """
    info: dict = {"active_window": None, "windows": []}

    # Active window
    try:
        wid = (await run(["xdotool", "getactivewindow"])).strip()
        name = (await run(["xdotool", "getwindowname", wid])).strip()
        if name:
            info["active_window"] = name
    except Exception:
        pass

    # Visible windows
    try:
        wids_raw = await run(["xdotool", "search", "--onlyvisible", "--name", ""])
        for wid in wids_raw.strip().splitlines():
            wid = wid.strip()
            if not wid:
                continue
            try:
                name = (await run(["xdotool", "getwindowname", wid])).strip()
                if not name:
                    continue
                geo_raw = await run(["xdotool", "getwindowgeometry", wid])
                # "Window 123\n  Position: 10,20 (screen: 0)\n  Geometry: 800x600"
                geometry = _parse_geometry(geo_raw)
                info["windows"].append({"title": name, **geometry})
            except Exception:
                continue
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
