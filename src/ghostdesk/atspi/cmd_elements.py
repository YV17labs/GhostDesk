# Copyright (c) 2026 YV17 — MIT License
"""Command: elements — list interactive UI elements."""

from __future__ import annotations

import argparse
import json
import sys

from gi.repository import Atspi  # type: ignore[attr-defined]

from ._roles import _INTERACTIVE_ROLES, _role_name
from ._helpers import _is_showing, _get_extents
from ._tree import _collect_from_desktop


def _get_element_info(
    obj: Atspi.Accessible,
    role_filter: set[str] | None = None,
) -> dict | None:
    """Extract element info if it is interactive and visible on screen."""
    try:
        role = obj.get_role()
    except Exception:
        return None

    if role not in _INTERACTIVE_ROLES:
        return None

    role_str = _role_name(role)
    if role_filter and role_str not in role_filter:
        return None

    if not _is_showing(obj):
        return None

    try:
        name = obj.get_name() or ""
    except Exception:
        name = ""

    try:
        description = obj.get_description() or ""
    except Exception:
        description = ""

    if not name and not description:
        return None

    extents = _get_extents(obj)
    if extents is None:
        return None

    element: dict = {
        "role": role_str,
        "name": name,
        **extents,
    }

    if description:
        element["description"] = description

    # Extra state flags useful for the LLM.
    try:
        states = obj.get_state_set()
        if states.contains(Atspi.StateType.FOCUSED):
            element["focused"] = True
        if states.contains(Atspi.StateType.CHECKED):
            element["checked"] = True
        if not states.contains(Atspi.StateType.ENABLED):
            element["disabled"] = True
    except Exception:
        pass

    return element


def _walk_elements(
    obj: Atspi.Accessible,
    results: list[dict],
    max_results: int,
    role_filter: set[str] | None = None,
) -> None:
    """Recursively walk the accessibility tree, collecting interactive elements."""
    if len(results) >= max_results:
        return

    info = _get_element_info(obj, role_filter=role_filter)
    if info is not None:
        results.append(info)

    try:
        count = obj.get_child_count()
    except Exception:
        return

    for i in range(count):
        if len(results) >= max_results:
            return
        try:
            child = obj.get_child_at_index(i)
            if child is not None:
                _walk_elements(child, results, max_results, role_filter=role_filter)
        except Exception:
            continue


def cmd_elements(args: argparse.Namespace) -> None:
    """List interactive elements."""
    role_filter = set(args.role) if args.role else None
    results = _collect_from_desktop(_walk_elements, args.max, role_filter)
    json.dump(results, sys.stdout, ensure_ascii=False)
