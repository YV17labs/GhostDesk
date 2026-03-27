# Copyright (c) 2026 YV17 — MIT License
"""Command: read — read the screen like a screen reader.

Walks the accessibility tree and returns a flat list of visible elements
sorted by position.  Browser chrome is separated from web-app content
using document-role boundaries.
"""

from __future__ import annotations

import argparse
import json
import sys

from gi.repository import Atspi  # type: ignore[attr-defined]

from ._roles import _READABLE_ROLES, _INTERACTIVE_ROLES, _role_name
from ._helpers import _is_showing, _get_extents, _get_text_content

# Roles that mark the boundary between browser chrome and web app content.
_DOCUMENT_ROLES = frozenset({
    Atspi.Role.DOCUMENT_WEB,
    Atspi.Role.DOCUMENT_TEXT,
    Atspi.Role.DOCUMENT_FRAME,
    Atspi.Role.DOCUMENT_EMAIL,
    Atspi.Role.DOCUMENT_PRESENTATION,
    Atspi.Role.DOCUMENT_SPREADSHEET,
})

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
    include_positions: bool = True,
    inside_document: bool = False,
) -> list[dict]:
    """Build a flat list from the AT-SPI accessibility tree.

    Returns a list of element dicts.  Structural containers (no name,
    non-readable role) are skipped and their children promoted.

    Each entry is tagged with ``_doc: True`` when it lives inside a
    document boundary (DOCUMENT_WEB, etc.) so the caller can split
    browser chrome from web-app content.
    """
    try:
        role = obj.get_role()
    except Exception:
        return []

    # Detect document boundary — everything below is app content.
    if role in _DOCUMENT_ROLES:
        inside_document = True

    role_str = _role_name(role)
    if role_filter is not None:
        readable = role_str in role_filter
    else:
        readable = role in _READABLE_ROLES

    # Past the output limit: count roles up to a bounded cap, then stop.
    if counters["visible"] >= max_nodes:
        if counters["total_in_tree"] < max_nodes * 10:
            showing = _is_showing(obj)
            if readable and showing:
                counters["total_in_tree"] += 1
                counters["all_roles"].add(role_str)
            try:
                for i in range(obj.get_child_count()):
                    child = obj.get_child_at_index(i)
                    if child is not None:
                        _collect_tree(child, max_nodes, counters, role_filter,
                                      include_positions, inside_document)
            except Exception:
                pass
        return []

    showing = _is_showing(obj)
    if readable and showing:
        counters["total_in_tree"] += 1
        counters["all_roles"].add(role_str)

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
        try:
            child = obj.get_child_at_index(i)
            if child is not None:
                child_entries.extend(
                    _collect_tree(child, max_nodes, counters, role_filter,
                                include_positions, inside_document)
                )
        except Exception:
            continue

    # --- Decide whether this node is meaningful ---
    if not (bool(name) and readable and showing):
        # Structural container — skip it, promote its children.
        return child_entries

    # --- Build the node entry ---
    counters["visible"] += 1
    entry: dict = {"role": role_str, "name": name}

    # Heading level
    if role == Atspi.Role.HEADING:
        try:
            attrs = obj.get_attributes()
            if isinstance(attrs, dict) and "level" in attrs:
                entry["level"] = int(attrs["level"])
        except Exception:
            pass

    if include_positions:
        extents = _get_extents(obj)
        if extents:
            entry["y"] = extents["y"]
            entry["x"] = extents["x"]
            entry["center_x"] = extents["center_x"]
            entry["center_y"] = extents["center_y"]

    # States
    states = _get_useful_states(obj)
    if states:
        entry["states"] = states

    # Tag for browser/content split
    entry["_doc"] = inside_document

    if child_entries:
        entry["children"] = child_entries

    return [entry]


def _flatten(entries: list[dict]) -> list[dict]:
    """Flatten a nested tree into a single list (iterative, no mutation)."""
    result: list[dict] = []
    stack = list(reversed(entries))
    while stack:
        entry = stack.pop()
        children = entry.pop("children", None)
        result.append(entry)
        if children:
            stack.extend(reversed(children))
    return result


def _matches_app(obj: Atspi.Accessible, app_filter_lower: str) -> bool:
    """Check if an application's name contains the filter (already lowercase)."""
    try:
        name = (obj.get_name() or "").lower()
        return app_filter_lower in name
    except Exception:
        return False


def cmd_read(args: argparse.Namespace) -> None:
    """Read the screen like a screen reader — returns a flat list."""
    role_filter = set(args.role) if args.role else None
    app_filter = args.app.lower() if args.app else None
    counters = {"visible": 0, "total_in_tree": 0, "all_roles": set()}

    all_entries: list[dict] = []
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        try:
            app = desktop.get_child_at_index(i)
            if app is None:
                continue
            if app_filter and not _matches_app(app, app_filter):
                continue
            all_entries.extend(
                _collect_tree(app, args.max, counters, role_filter,
                            args.include_positions)
            )
        except Exception:
            continue

    flat = _flatten(all_entries)
    flat.sort(key=lambda e: (e.get("y", 9999), e.get("x", 9999)))

    # Single pass: split browser chrome from app content and strip _doc tag.
    app_content: list[dict] = []
    browser: list[dict] = []
    for e in flat:
        if e.pop("_doc", False):
            app_content.append(e)
        else:
            browser.append(e)

    # When no document boundary was found, everything is app content.
    if not app_content:
        app_content = browser
        browser = []

    output: dict = {
        "context": _build_context(app_content, counters),
        "items": app_content,
        "visible": counters["visible"],
        "total_in_tree": counters["total_in_tree"],
    }

    if browser:
        output["browser"] = browser

    # When truncated, show what roles exist — guides toward role filtering.
    if counters["total_in_tree"] > counters["visible"]:
        shown_roles = {e["role"] for e in app_content}
        hidden_roles = sorted(counters["all_roles"] - shown_roles)
        if hidden_roles:
            output["not_shown"] = hidden_roles

    json.dump(output, sys.stdout, ensure_ascii=False)


def _build_context(items: list[dict], counters: dict) -> str:
    """Build a one-line semantic summary from the elements.

    Uses role counts, landmark-like names, and element content to describe
    what the page looks like — similar to a screen reader announcing the
    page structure when you first land on it.
    """
    parts: list[str] = []

    # Count roles in the visible items.
    role_counts: dict[str, int] = {}
    for e in items:
        r = e.get("role", "")
        role_counts[r] = role_counts.get(r, 0) + 1

    # Detect page type from document name.
    for e in items:
        if e.get("role") == "document":
            parts.append(e.get("name", ""))
            break

    # Summarize content by dominant roles.
    summaries = [
        ("table_row", "rows"),
        ("cell", "cells"),
        ("link", "links"),
        ("button", "buttons"),
        ("textfield", "text fields"),
        ("heading", "headings"),
        ("paragraph", "paragraphs"),
        ("listitem", "list items"),
        ("checkbox", "checkboxes"),
        ("radio", "radio buttons"),
        ("tab", "tabs"),
    ]
    for role, label in summaries:
        count = role_counts.get(role, 0)
        if count > 0:
            parts.append(f"{count} {label}")

    # Mention truncation.
    total = counters.get("total_in_tree", 0)
    visible = counters.get("visible", 0)
    if total > visible:
        parts.append(f"{total - visible} more elements hidden (use role filter)")

    return ". ".join(parts) if parts else "Empty screen"
