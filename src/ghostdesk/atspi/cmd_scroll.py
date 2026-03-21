# Copyright (c) 2026 YV17 — MIT License
"""Command: scroll — scroll to an element."""

from __future__ import annotations

import argparse
import json
import sys

from gi.repository import Atspi  # type: ignore[attr-defined]

from ._roles import _role_name
from ._tree import _find_on_desktop


def cmd_scroll(args: argparse.Namespace) -> None:
    """Scroll to bring an element into view."""
    obj = _find_on_desktop(args.text, args.role)
    if obj is None:
        json.dump({"error": f"Element not found: '{args.text}'"}, sys.stdout)
        return

    try:
        obj.scroll_to(Atspi.ScrollType.TOP_LEFT)
        name = obj.get_name() or args.text
        json.dump({
            "status": "ok",
            "scrolled_to": name,
            "role": _role_name(obj.get_role()),
        }, sys.stdout, ensure_ascii=False)
    except Exception as exc:
        json.dump({"error": f"scroll_to failed: {exc}"}, sys.stdout)
