# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Open window listing via Sway IPC (``swaymsg -t get_tree``).

Sway returns a full JSON tree of every container on the compositor.  A
"window" is a leaf container (``type`` ``con`` or ``floating_con``) that
has a ``pid`` — which is both Wayland-native clients (``app_id``) and
any XWayland apps (``window_properties.class``).
"""

from __future__ import annotations

import json
from typing import Iterable

from ghostdesk._cmd import run


def _walk(node: dict) -> Iterable[dict]:
    """Depth-first walk of a Sway tree node."""
    yield node
    for child in node.get("nodes", []) or []:
        yield from _walk(child)
    for child in node.get("floating_nodes", []) or []:
        yield from _walk(child)


def _extract_windows(tree: dict) -> list[dict]:
    """Return leaf containers that represent real windows.

    Sway's tree includes outputs, workspaces, and intermediate split
    containers — we only want the leaves that carry an actual surface.
    """
    out: list[dict] = []
    for n in _walk(tree):
        if n.get("type") not in ("con", "floating_con"):
            continue
        if n.get("pid") is None and not n.get("app_id"):
            continue
        # Skip non-leaf cons (they hold sub-containers, not a surface)
        if n.get("nodes") or n.get("floating_nodes"):
            continue
        title = (n.get("name") or "").strip()
        if not title:
            continue
        rect = n.get("rect") or {}
        app = n.get("app_id")
        if not app:
            # XWayland fallback
            wp = n.get("window_properties") or {}
            app = wp.get("class") or "unknown"
        out.append({
            "app": str(app).lower(),
            "title": title,
            "x": int(rect.get("x", 0)),
            "y": int(rect.get("y", 0)),
            "width": int(rect.get("width", 0)),
            "height": int(rect.get("height", 0)),
        })
    return out


async def get_open_windows() -> list[dict]:
    """Return visible windows with title and geometry."""
    try:
        output = await run(["swaymsg", "-t", "get_tree"])
        tree = json.loads(output)
        return _extract_windows(tree)
    except Exception:
        return []
