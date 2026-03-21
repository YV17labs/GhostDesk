# Copyright (c) 2026 YV17 — MIT License
"""Command: text — read all visible text content from the screen."""

from __future__ import annotations

import argparse
import json
import sys

from gi.repository import Atspi  # type: ignore[attr-defined]

from ._roles import _TEXT_ROLES, _role_name
from ._helpers import _is_showing, _get_extents, _get_text_content
from ._tree import _collect_from_desktop


def _walk_text(
    obj: Atspi.Accessible,
    results: list[dict],
    max_results: int,
) -> None:
    """Recursively walk the tree, collecting visible text content."""
    if len(results) >= max_results:
        return

    try:
        role = obj.get_role()
    except Exception:
        role = None

    if role in _TEXT_ROLES and _is_showing(obj):
        text = _get_text_content(obj)
        name = ""
        try:
            name = obj.get_name() or ""
        except Exception:
            pass

        # Use text content if available, fall back to name.
        # Filter out object replacement characters (U+FFFC) used for embedded objects.
        stripped = text.strip()
        content = (stripped if stripped else name.strip()).replace("\ufffc", "").strip()

        if content:
            entry: dict = {
                "role": _role_name(role),
                "text": content,
            }

            # Add heading level if applicable
            if role == Atspi.Role.HEADING:
                try:
                    attrs = obj.get_attributes()
                    if isinstance(attrs, dict) and "level" in attrs:
                        entry["level"] = int(attrs["level"])
                except Exception:
                    pass

            extents = _get_extents(obj)
            if extents:
                entry["y"] = extents["y"]
                entry["x"] = extents["x"]

            results.append(entry)

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
                _walk_text(child, results, max_results)
        except Exception:
            continue


def cmd_text(args: argparse.Namespace) -> None:
    """Read all visible text from the screen."""
    results = _collect_from_desktop(_walk_text, args.max)
    # Sort by vertical position (top to bottom) for natural reading order
    results.sort(key=lambda e: (e.get("y", 9999), e.get("x", 9999)))
    json.dump(results, sys.stdout, ensure_ascii=False)
