# Copyright (c) 2026 YV17 — MIT License
"""Command: find — find element and return coordinates."""

from __future__ import annotations

import argparse
import json
import sys

from gi.repository import Atspi  # type: ignore[attr-defined]

from ._roles import _role_name
from ._helpers import _get_extents
from ._tree import _find_on_desktop


def cmd_find(args: argparse.Namespace) -> None:
    """Find an element and return its name, role, and coordinates."""
    obj = _find_on_desktop(args.text, args.role)
    if obj is None:
        json.dump({"error": f"Element not found: '{args.text}'"}, sys.stdout)
        return

    try:
        role = obj.get_role()
    except Exception:
        role = Atspi.Role.UNKNOWN

    result: dict = {
        "name": "",
        "role": _role_name(role),
    }
    try:
        result["name"] = obj.get_name() or ""
    except Exception:
        pass

    extents = _get_extents(obj)
    if extents:
        result.update(extents)

    json.dump(result, sys.stdout, ensure_ascii=False)
