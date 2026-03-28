# Copyright (c) 2026 YV17 — MIT License
"""AT-SPI2 accessibility tree query — CLI entry point.

This module is executed via /usr/bin/python3 (system Python with python3-gi)
because PyGObject is a system package not available inside the uv venv.

Usage:
    /usr/bin/python3 -m ghostdesk.atspi <command> [options]

Commands:
    read       — read the screen like a screen reader (name + role + states)
    details    — inspect a specific element (actions, states, text, relations)
    table      — extract a structured table
    find       — find element and return coordinates
    scroll     — scroll to an element
    set-value  — set text content on an element
    focus      — give focus to an element

Outputs JSON to stdout.
"""

from __future__ import annotations

import argparse
import json
import sys
import warnings

warnings.filterwarnings("ignore", category=DeprecationWarning, module="ghostdesk")

try:
    import gi

    gi.require_version("Atspi", "2.0")
    from gi.repository import Atspi  # type: ignore[attr-defined]  # noqa: F401
except (ImportError, ValueError) as exc:
    print(json.dumps({"error": f"AT-SPI not available: {exc}"}))
    sys.exit(1)

# Now safe to import from this package — gi.require_version has been called.
from .cmd_read import cmd_read
from .cmd_details import cmd_details
from .cmd_find import cmd_find
from .cmd_table import cmd_table
from .cmd_scroll import cmd_scroll
from .cmd_set_value import cmd_set_value
from .cmd_focus import cmd_focus
from .cmd_focused import cmd_focused


def main() -> None:
    parser = argparse.ArgumentParser(description="AT-SPI accessibility tree query")
    subparsers = parser.add_subparsers(dest="command")

    # read (primary command — replaces both "elements" and "text")
    p_read = subparsers.add_parser("read", help="Read screen like a screen reader")
    p_read.add_argument("--role", action="append", default=None)
    p_read.add_argument("--max", type=int, default=100,
                       help="Max visible elements to return (default: 100)")
    p_read.add_argument("--include-positions", action="store_true",
                       help="Include x/y coordinates in output (default: false)")
    p_read.add_argument("--app", default=None,
                       help="Filter by application name (e.g. 'Firefox')")
    p_read.set_defaults(func=cmd_read)

    # details
    p_detail = subparsers.add_parser("details", help="Inspect a specific element")
    p_detail.add_argument("text", help="Text to search for in element names")
    p_detail.add_argument("--role", default=None)
    p_detail.set_defaults(func=cmd_details)

    # find
    p_find = subparsers.add_parser("find", help="Find element and return coordinates")
    p_find.add_argument("text", help="Text to search for in element names")
    p_find.add_argument("--role", default=None)
    p_find.set_defaults(func=cmd_find)

    # table
    p_table = subparsers.add_parser("table", help="Extract a table")
    p_table.add_argument("--text", default=None, help="Table name/caption to search for")
    p_table.add_argument("--max-rows", type=int, default=100)
    p_table.set_defaults(func=cmd_table)

    # scroll
    p_scroll = subparsers.add_parser("scroll", help="Scroll to an element")
    p_scroll.add_argument("text", help="Element text to scroll to")
    p_scroll.add_argument("--role", default=None)
    p_scroll.set_defaults(func=cmd_scroll)

    # set-value
    p_set = subparsers.add_parser("set-value", help="Set text/value on an element")
    p_set.add_argument("text", help="Element to find")
    p_set.add_argument("value", help="Value to set")
    p_set.add_argument("--role", default=None)
    p_set.set_defaults(func=cmd_set_value)

    # focus
    p_focus = subparsers.add_parser("focus", help="Focus an element")
    p_focus.add_argument("text", help="Element text to focus")
    p_focus.add_argument("--role", default=None)
    p_focus.set_defaults(func=cmd_focus)

    # focused
    p_focused = subparsers.add_parser("focused", help="Return the focused element")
    p_focused.set_defaults(func=cmd_focused)

    args = parser.parse_args()

    if hasattr(args, "func"):
        args.func(args)
    else:
        # Default: read
        args.role = None
        args.app = None
        args.max = 100
        args.include_positions = True
        cmd_read(args)


if __name__ == "__main__":
    main()
