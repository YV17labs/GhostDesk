# Copyright (c) 2026 YV17 — MIT License
"""Command: set-value — set text content or numeric value on an element."""

from __future__ import annotations

import argparse
import json
import sys

from ._helpers import _get_text_content
from ._tree import _find_on_desktop


def cmd_set_value(args: argparse.Namespace) -> None:
    """Set text content or numeric value on an element."""
    obj = _find_on_desktop(args.text, args.role)
    if obj is None:
        json.dump({"error": f"Element not found: '{args.text}'"}, sys.stdout)
        return

    # Try editable text first
    try:
        editable = obj.get_editable_text_iface()
        if editable is not None:
            # Clear and set
            char_count = obj.get_character_count()
            if char_count > 0:
                obj.delete_text(0, char_count)
            obj.insert_text(0, args.value, len(args.value))
            json.dump({
                "status": "ok",
                "element": obj.get_name() or args.text,
                "value_set": args.value,
            }, sys.stdout, ensure_ascii=False)
            return
    except Exception:
        pass

    # Try value interface (sliders, spinbuttons)
    try:
        val_iface = obj.get_value_iface()
        if val_iface is not None:
            numeric_val = float(args.value)
            obj.set_current_value(numeric_val)
            json.dump({
                "status": "ok",
                "element": obj.get_name() or args.text,
                "value_set": numeric_val,
            }, sys.stdout, ensure_ascii=False)
            return
    except Exception:
        pass

    json.dump({
        "error": f"Element '{args.text}' does not support text editing or value setting",
    }, sys.stdout)
