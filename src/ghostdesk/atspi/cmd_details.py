# Copyright (c) 2026 YV17 — MIT License
"""Command: details — inspect a specific element."""

from __future__ import annotations

import argparse
import json
import sys

from gi.repository import Atspi  # type: ignore[attr-defined]

from ._roles import _role_name
from ._helpers import _get_extents, _get_text_content, _get_states, _get_actions, _get_value, _get_relations
from ._tree import _find_on_desktop


def cmd_details(args: argparse.Namespace) -> None:
    """Inspect a specific element in detail."""
    obj = _find_on_desktop(args.text, args.role)
    if obj is None:
        json.dump({"error": f"Element not found: '{args.text}'"}, sys.stdout)
        return

    try:
        role = obj.get_role()
    except Exception:
        role = Atspi.Role.UNKNOWN

    result: dict = {
        "role": _role_name(role),
        "name": "",
        "description": "",
        "text": "",
        "states": [],
        "actions": [],
    }

    try:
        result["name"] = obj.get_name() or ""
    except Exception:
        pass

    try:
        result["description"] = obj.get_description() or ""
    except Exception:
        pass

    result["text"] = _get_text_content(obj)
    result["states"] = _get_states(obj)
    result["actions"] = _get_actions(obj)

    extents = _get_extents(obj)
    if extents:
        result["bounds"] = extents

    value = _get_value(obj)
    if value:
        result["value"] = value

    relations = _get_relations(obj)
    if relations:
        result["relations"] = relations

    # Attributes
    try:
        attrs = obj.get_attributes()
        if attrs:
            result["attributes"] = dict(attrs) if not isinstance(attrs, dict) else attrs
    except Exception:
        pass

    # Children summary
    try:
        child_count = obj.get_child_count()
        if child_count > 0:
            children = []
            for i in range(min(child_count, 50)):
                try:
                    child = obj.get_child_at_index(i)
                    if child:
                        cr = child.get_role()
                        cn = child.get_name() or ""
                        ct = _get_text_content(child)
                        children.append({
                            "role": _role_name(cr),
                            "name": cn,
                            "text": ct[:200] if ct else "",
                        })
                except Exception:
                    continue
            if children:
                result["children"] = children
    except Exception:
        pass

    json.dump(result, sys.stdout, ensure_ascii=False)
