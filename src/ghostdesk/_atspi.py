# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""AT-SPI query — visible interactive elements.

Executed via system Python (/usr/bin/python3 -m ghostdesk._atspi)
which has python3-gi.  Outputs a JSON array to stdout.
"""

from __future__ import annotations

import json
import sys
import warnings

warnings.filterwarnings("ignore", category=DeprecationWarning)

try:
    import gi
    gi.require_version("Atspi", "2.0")
    from gi.repository import Atspi  # type: ignore[attr-defined]
except (ImportError, ValueError):
    json.dump([], sys.stdout)
    sys.exit(0)

_ROLE_NAMES = {
    Atspi.Role.PUSH_BUTTON: "button",
    Atspi.Role.TOGGLE_BUTTON: "toggle",
    Atspi.Role.CHECK_BOX: "checkbox",
    Atspi.Role.RADIO_BUTTON: "radio",
    Atspi.Role.COMBO_BOX: "combobox",
    Atspi.Role.MENU: "menu",
    Atspi.Role.MENU_ITEM: "menuitem",
    Atspi.Role.CHECK_MENU_ITEM: "checkmenuitem",
    Atspi.Role.RADIO_MENU_ITEM: "radiomenuitem",
    Atspi.Role.LINK: "link",
    Atspi.Role.ENTRY: "textfield",
    Atspi.Role.PASSWORD_TEXT: "password",
    Atspi.Role.SPIN_BUTTON: "spinbutton",
    Atspi.Role.SLIDER: "slider",
    Atspi.Role.PAGE_TAB: "tab",
    Atspi.Role.LIST_ITEM: "listitem",
    Atspi.Role.TREE_ITEM: "treeitem",
}

_INTERACTIVE_ROLES = frozenset(_ROLE_NAMES)

MAX_ITEMS = 200


def _name(obj: Atspi.Accessible) -> str:
    try:
        n = (obj.get_name() or "").strip()
        if n:
            return n
        if "Text" in obj.get_interfaces():
            count = obj.get_character_count()
            if 0 < count <= 200:
                return (obj.get_text(0, count) or "").strip()
    except Exception:
        pass
    return ""


def _center(obj: Atspi.Accessible) -> tuple[int, int] | None:
    try:
        if "Component" not in obj.get_interfaces():
            return None
        r = obj.get_extents(Atspi.CoordType.SCREEN)
        if r.width <= 0 or r.height <= 0:
            return None
        if r.x + r.width < 0 or r.y + r.height < 0:
            return None
        return (r.x + r.width // 2, r.y + r.height // 2)
    except Exception:
        return None


def _collect(obj: Atspi.Accessible, out: list[dict]) -> None:
    if len(out) >= MAX_ITEMS:
        return
    try:
        role = obj.get_role()
    except Exception:
        return

    if role in _INTERACTIVE_ROLES:
        try:
            states = obj.get_state_set()
            showing = states.contains(Atspi.StateType.SHOWING)
        except Exception:
            showing = False

        if showing:
            n = _name(obj)
            c = _center(obj)
            if n and c:
                out.append({
                    "role": _ROLE_NAMES.get(role, "unknown"),
                    "name": n,
                    "x": c[0],
                    "y": c[1],
                })

    try:
        for i in range(obj.get_child_count()):
            child = obj.get_child_at_index(i)
            if child is not None:
                _collect(child, out)
            if len(out) >= MAX_ITEMS:
                return
    except Exception:
        pass


def _get_windows(app: Atspi.Accessible) -> list[dict]:
    """Return visible FRAME windows for an application."""
    windows: list[dict] = []
    try:
        for i in range(app.get_child_count()):
            child = app.get_child_at_index(i)
            if child is None:
                continue
            if child.get_role() != Atspi.Role.FRAME:
                continue
            title = (child.get_name() or "").strip()
            if not title:
                continue
            if "Component" not in child.get_interfaces():
                continue
            r = child.get_extents(Atspi.CoordType.SCREEN)
            if r.width <= 1 or r.height <= 1:
                continue
            windows.append({
                "title": title,
                "x": r.x, "y": r.y,
                "width": r.width, "height": r.height,
            })
    except Exception:
        pass
    return windows


def main() -> None:
    apps: list[dict] = []
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        try:
            app = desktop.get_child_at_index(i)
            if app is None:
                continue
            name = (app.get_name() or "").strip() or "unknown"
            windows = _get_windows(app)
            elements: list[dict] = []
            _collect(app, elements)
            if elements or windows:
                entry: dict = {"app": name}
                if windows:
                    entry["windows"] = windows
                if elements:
                    elements.sort(key=lambda e: (e["y"], e["x"]))
                    entry["elements"] = elements
                apps.append(entry)
        except Exception:
            continue

    json.dump(apps, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
