# Copyright (c) 2026 YV17 — MIT License
"""Desktop tree traversal helpers for AT-SPI."""

from __future__ import annotations

from gi.repository import Atspi  # type: ignore[attr-defined]

from ._roles import _role_name
from ._helpers import _is_showing


def _for_each_app(fn, *args):
    """Call fn(app, *args) for each application on the desktop."""
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        try:
            app = desktop.get_child_at_index(i)
            if app is not None:
                result = fn(app, *args)
                if result is not None:
                    return result
        except Exception:
            continue
    return None



def _find_element(
    obj: Atspi.Accessible,
    text: str | None = None,
    role: str | None = None,
    _text_lower: str | None = None,
) -> Atspi.Accessible | None:
    """Find an element by text match on name or description, and/or by role.

    If text is None, matches any element with the given role.
    If role is None, matches any role.
    """
    # Lowercase text once, reuse across recursion.
    if _text_lower is None and text is not None:
        _text_lower = text.lower()

    try:
        obj_role = obj.get_role()
    except Exception:
        obj_role = None

    if _is_showing(obj):
        if role is not None and _role_name(obj_role) != role:
            pass  # role doesn't match, skip this node
        elif text is None:
            return obj  # no text filter, role matched (or no role filter)
        else:
            name = ""
            desc = ""
            try:
                name = obj.get_name() or ""
            except Exception:
                pass
            try:
                desc = obj.get_description() or ""
            except Exception:
                pass

            if _text_lower in name.lower() or _text_lower in desc.lower():
                return obj

    try:
        count = obj.get_child_count()
    except Exception:
        return None

    for i in range(count):
        try:
            child = obj.get_child_at_index(i)
            if child is not None:
                found = _find_element(child, text, role, _text_lower)
                if found is not None:
                    return found
        except Exception:
            continue

    return None


def _find_on_desktop(text: str | None = None, role: str | None = None) -> Atspi.Accessible | None:
    """Search the entire desktop for an element."""
    return _for_each_app(_find_element, text, role)
