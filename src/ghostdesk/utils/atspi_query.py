# Copyright (c) 2026 YV17 — MIT License
"""AT-SPI2 accessibility tree query.

This script is executed via /usr/bin/python3 (system Python with python3-gi)
because PyGObject is a system package not available inside the uv venv.

Usage:
    /usr/bin/python3 -m ghostdesk.utils.atspi_query [--role ROLE ...] [--max N]

Outputs a JSON array of interactive UI elements to stdout.
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
    from gi.repository import Atspi  # type: ignore[attr-defined]
except (ImportError, ValueError) as exc:
    print(json.dumps({"error": f"AT-SPI not available: {exc}"}))
    sys.exit(1)

# AT-SPI roles we consider "interactive" — the ones an LLM agent would want to
# click on, type into, or otherwise manipulate.
_INTERACTIVE_ROLES = frozenset(
    {
        Atspi.Role.PUSH_BUTTON,
        Atspi.Role.TOGGLE_BUTTON,
        Atspi.Role.CHECK_BOX,
        Atspi.Role.RADIO_BUTTON,
        Atspi.Role.COMBO_BOX,
        Atspi.Role.MENU,
        Atspi.Role.MENU_ITEM,
        Atspi.Role.LINK,
        Atspi.Role.ENTRY,
        Atspi.Role.PASSWORD_TEXT,
        Atspi.Role.TEXT,
        Atspi.Role.SPIN_BUTTON,
        Atspi.Role.SLIDER,
        Atspi.Role.PAGE_TAB,
        Atspi.Role.TREE_ITEM,
        Atspi.Role.LIST_ITEM,
        Atspi.Role.TABLE_CELL,
    }
)

# Human-readable role names for the JSON output.
_ROLE_NAMES: dict[Atspi.Role, str] = {
    Atspi.Role.PUSH_BUTTON: "button",
    Atspi.Role.TOGGLE_BUTTON: "toggle",
    Atspi.Role.CHECK_BOX: "checkbox",
    Atspi.Role.RADIO_BUTTON: "radio",
    Atspi.Role.COMBO_BOX: "combobox",
    Atspi.Role.MENU: "menu",
    Atspi.Role.MENU_ITEM: "menuitem",
    Atspi.Role.LINK: "link",
    Atspi.Role.ENTRY: "textfield",
    Atspi.Role.PASSWORD_TEXT: "password",
    Atspi.Role.TEXT: "text",
    Atspi.Role.SPIN_BUTTON: "spinbutton",
    Atspi.Role.SLIDER: "slider",
    Atspi.Role.PAGE_TAB: "tab",
    Atspi.Role.TREE_ITEM: "treeitem",
    Atspi.Role.LIST_ITEM: "listitem",
    Atspi.Role.TABLE_CELL: "cell",
}


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

    # Early exit if this role doesn't match the filter.
    if role_filter:
        role_name = _ROLE_NAMES.get(role)
        if role_name not in role_filter:
            return None

    # Skip elements that are not showing / not visible.
    try:
        states = obj.get_state_set()
        if not states.contains(Atspi.StateType.SHOWING):
            return None
    except Exception:
        return None

    # Name / description — skip unnamed elements.
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

    # Bounding box on screen.
    try:
        component = obj.get_component_iface()
        if component is None:
            return None
        rect = component.get_extents(Atspi.CoordType.SCREEN)
        # Skip zero-size or off-screen elements.
        if rect.width <= 0 or rect.height <= 0:
            return None
        if rect.x + rect.width < 0 or rect.y + rect.height < 0:
            return None
    except Exception:
        return None

    role_name = _ROLE_NAMES.get(role, str(role).split(".")[-1].lower())

    element: dict = {
        "role": role_name,
        "name": name,
        "x": rect.x,
        "y": rect.y,
        "width": rect.width,
        "height": rect.height,
        "center_x": rect.x + rect.width // 2,
        "center_y": rect.y + rect.height // 2,
    }

    if description:
        element["description"] = description

    # Extra state flags useful for the LLM.
    try:
        if states.contains(Atspi.StateType.FOCUSED):
            element["focused"] = True
        if states.contains(Atspi.StateType.CHECKED):
            element["checked"] = True
        if not states.contains(Atspi.StateType.ENABLED):
            element["disabled"] = True
    except Exception:
        pass

    return element


def _walk(
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
                _walk(child, results, max_results, role_filter=role_filter)
        except Exception:
            continue


def query_elements(
    role_filter: set[str] | None = None,
    max_results: int = 200,
) -> list[dict]:
    """Query the AT-SPI tree and return interactive elements."""
    desktop = Atspi.get_desktop(0)
    results: list[dict] = []

    app_count = desktop.get_child_count()
    for i in range(app_count):
        try:
            app = desktop.get_child_at_index(i)
            if app is not None:
                _walk(app, results, max_results, role_filter=role_filter)
        except Exception:
            continue

    return results


def main() -> None:
    parser = argparse.ArgumentParser(description="Query AT-SPI accessibility tree")
    parser.add_argument(
        "--role",
        action="append",
        default=None,
        help="Filter by role (can be repeated, e.g. --role button --role link)",
    )
    parser.add_argument(
        "--max",
        type=int,
        default=200,
        help="Maximum number of elements to return (default: 200)",
    )
    args = parser.parse_args()

    role_filter = set(args.role) if args.role else None
    elements = query_elements(role_filter=role_filter, max_results=args.max)
    json.dump(elements, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
