# Copyright (c) 2026 YV17 — MIT License
"""Command: read — read the screen like a screen reader.

Walks the accessibility tree and returns a **semantic tree** — structural
containers without names (div, panel, filler, …) are collapsed, and only
meaningful elements (those with an accessible name, a useful role, and
visible on screen) are kept.

Each element is announced the way a screen reader would:
    name  +  role  +  states

Children of structural containers are promoted to the nearest meaningful
ancestor.  Children whose name is identical to their parent's are removed
(common pattern in web accessibility trees where a cell and its child
checkbox share the same accessible name).
"""

from __future__ import annotations

import argparse
import json
import sys

from gi.repository import Atspi  # type: ignore[attr-defined]

from ._roles import _READABLE_ROLES, _INTERACTIVE_ROLES, _role_name
from ._helpers import _is_showing, _get_extents, _get_text_content

# States that carry useful information for the LLM.
_USEFUL_STATES: list[tuple[Atspi.StateType, str]] = [
    (Atspi.StateType.CHECKED, "checked"),
    (Atspi.StateType.SELECTED, "selected"),
    (Atspi.StateType.EXPANDED, "expanded"),
    (Atspi.StateType.PRESSED, "pressed"),
    (Atspi.StateType.REQUIRED, "required"),
    (Atspi.StateType.INVALID_ENTRY, "invalid"),
    (Atspi.StateType.READ_ONLY, "readonly"),
    (Atspi.StateType.FOCUSED, "focused"),
]


def _get_useful_states(obj: Atspi.Accessible) -> list[str]:
    """Return only the states a screen reader would announce."""
    result: list[str] = []
    try:
        states = obj.get_state_set()
        if not states.contains(Atspi.StateType.ENABLED):
            result.append("disabled")
        for state_type, label in _USEFUL_STATES:
            if states.contains(state_type):
                result.append(label)
    except Exception:
        pass
    return result


def _collect_tree(
    obj: Atspi.Accessible,
    max_nodes: int,
    counters: dict,
    role_filter: set[str] | None = None,
) -> list[dict]:
    """Build a semantic tree from the AT-SPI accessibility tree.

    Returns a list of tree nodes (dicts).  Each node may have a ``children``
    key.  Structural containers (no name, non-readable role) are skipped and
    their children promoted to the parent level.

    Children with the exact same ``name`` as their parent are removed to
    avoid the common parent/child duplication pattern in web accessibility
    trees (e.g. a ``cell`` and its inner ``checkbox`` sharing one name).
    """
    if counters["visible"] >= max_nodes:
        return []

    try:
        role = obj.get_role()
    except Exception:
        return []

    showing = _is_showing(obj)
    readable = role in _READABLE_ROLES

    # Count every readable node for the has_more indicator.
    if readable:
        counters["total_in_tree"] += 1

    # --- Get the accessible name (screen-reader priority) ---
    name = ""
    if readable and showing:
        try:
            name = (obj.get_name() or "").strip()
        except Exception:
            pass
        if not name:
            name = _get_text_content(obj).replace("\ufffc", "").strip()

    # --- Recurse into children ---
    child_entries: list[dict] = []
    try:
        n_children = obj.get_child_count()
    except Exception:
        n_children = 0

    for i in range(n_children):
        if counters["visible"] >= max_nodes:
            break
        try:
            child = obj.get_child_at_index(i)
            if child is not None:
                child_entries.extend(
                    _collect_tree(child, max_nodes, counters, role_filter)
                )
        except Exception:
            continue

    # --- Decide whether this node is meaningful ---
    is_meaningful = bool(name) and readable and showing

    if is_meaningful and role_filter:
        if _role_name(role) not in role_filter:
            is_meaningful = False

    if not is_meaningful:
        # Structural container — skip it, promote its children.
        return child_entries

    # --- Build the node entry ---
    counters["visible"] += 1
    role_str = _role_name(role)
    entry: dict = {"role": role_str, "name": name}

    # Heading level
    if role == Atspi.Role.HEADING:
        try:
            attrs = obj.get_attributes()
            if isinstance(attrs, dict) and "level" in attrs:
                entry["level"] = int(attrs["level"])
        except Exception:
            pass

    # Position
    extents = _get_extents(obj)
    if extents:
        entry["y"] = extents["y"]
        entry["x"] = extents["x"]

    # States
    states = _get_useful_states(obj)
    if states:
        entry["states"] = states

    # --- Attach children, deduplicating same-name nodes ---
    # Skip children whose name is identical to this node (parent/child
    # duplication pattern).  When skipping, promote *their* children so
    # we don't lose deeper interactive elements.
    filtered: list[dict] = []
    for child in child_entries:
        if child.get("name") == name:
            # Promote grandchildren (if any) instead of this duplicate.
            filtered.extend(child.get("children", []))
        else:
            filtered.append(child)

    if filtered:
        entry["children"] = filtered

    return [entry]


def cmd_read(args: argparse.Namespace) -> None:
    """Read the screen like a screen reader — returns a semantic tree."""
    role_filter = set(args.role) if args.role else None
    counters = {"visible": 0, "total_in_tree": 0}

    all_entries: list[dict] = []
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        try:
            app = desktop.get_child_at_index(i)
            if app is not None:
                all_entries.extend(
                    _collect_tree(app, args.max, counters, role_filter)
                )
        except Exception:
            continue

    # Sort top-level items by position for natural reading order.
    # Children keep their tree order (document order).
    all_entries.sort(key=lambda e: (e.get("y", 9999), e.get("x", 9999)))

    output = {
        "items": all_entries,
        "visible": counters["visible"],
        "total_in_tree": counters["total_in_tree"],
    }
    json.dump(output, sys.stdout, ensure_ascii=False)
