# Copyright (c) 2026 YV17 — MIT License
"""Command: focused — return the currently focused UI element."""

from __future__ import annotations

import argparse
import json
import sys

from gi.repository import Atspi  # type: ignore[attr-defined]

from ._helpers import _get_extents
from ._roles import _role_name
from ._tree import _for_each_app


def _find_focused(app: Atspi.Accessible) -> Atspi.Accessible | None:
    """Walk an application tree to find the element with FOCUSED state."""
    try:
        states = app.get_state_set()
        if states.contains(Atspi.StateType.FOCUSED):
            return app
    except Exception:
        return None

    try:
        n = app.get_child_count()
    except Exception:
        return None

    for i in range(n):
        try:
            child = app.get_child_at_index(i)
            if child is not None:
                found = _find_focused(child)
                if found is not None:
                    return found
        except Exception:
            continue

    return None


def cmd_focused(args: argparse.Namespace) -> None:
    """Return the currently focused element's name, role, and position."""
    found = _for_each_app(_find_focused)

    if found is None:
        json.dump({"error": "No focused element found"}, sys.stdout)
        return

    try:
        name = found.get_name() or ""
    except Exception:
        name = ""

    result: dict = {
        "name": name,
        "role": _role_name(found.get_role()),
    }

    extents = _get_extents(found)
    if extents:
        result.update(extents)

    json.dump(result, sys.stdout, ensure_ascii=False)
