# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Apps app_running tool — enumerate currently open application windows.

Use this before launching: an agent that calls ``app_running()`` first
can focus an existing Firefox tab via keyboard/click instead of starting
a second instance and doubling the memory footprint.
"""

from ghostdesk._sway import collect_views, get_tree


def _project(node: dict) -> dict:
    app = (
        node.get("app_id")
        or (node.get("window_properties") or {}).get("class")
        or "?"
    )
    return {
        "app": str(app),
        "title": node.get("name") or "",
        "pid": int(node["pid"]),
        "focused": bool(node.get("focused", False)),
    }


async def app_running() -> list[dict]:
    """List the application windows currently open on the desktop.

    Walks Sway's tree and returns one entry per real client window
    (Firefox, foot, mousepad, …). Workspaces, outputs and the bar are
    not included — only views with a backing process.

    Call this before ``app_launch()``: if the app is already in the
    list, switch to it (``key_press("alt+tab")`` or click its tab in
    the bottom bar) instead of launching a second instance.

    Returns a list of dicts, each with:
    - app: the application identifier (``app_id`` for Wayland-native
      clients, or the window class as a fallback).
    - title: the current window title (may be empty for some apps).
    - pid: the process ID — match this against ``app_launch()`` results.
    - focused: ``True`` if this window currently has keyboard focus.
    """
    tree = await get_tree()
    if tree is None:
        return [{"error": "swaymsg get_tree failed (see server logs)"}]
    return collect_views(tree, _project)
