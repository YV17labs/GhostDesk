# Copyright (c) 2026 YV17 — MIT License
"""Helper functions for inspecting AT-SPI accessible objects."""

from __future__ import annotations

from gi.repository import Atspi  # type: ignore[attr-defined]

from ._roles import _role_name


def _is_showing(obj: Atspi.Accessible) -> bool:
    """Check if an element is visible on screen."""
    try:
        states = obj.get_state_set()
        return states.contains(Atspi.StateType.SHOWING)
    except Exception:
        return False


def _get_extents(obj: Atspi.Accessible) -> dict | None:
    """Get bounding box on screen. Returns None if not visible."""
    try:
        if "Component" not in obj.get_interfaces():
            return None
        rect = obj.get_extents(Atspi.CoordType.SCREEN)
        if rect.width <= 0 or rect.height <= 0:
            return None
        if rect.x + rect.width < 0 or rect.y + rect.height < 0:
            return None
        return {
            "x": rect.x,
            "y": rect.y,
            "width": rect.width,
            "height": rect.height,
            "center_x": rect.x + rect.width // 2,
            "center_y": rect.y + rect.height // 2,
        }
    except Exception:
        return None


def _get_text_content(obj: Atspi.Accessible) -> str:
    """Extract text content from an element using the Text interface."""
    try:
        ifaces = obj.get_interfaces()
        if "Text" not in ifaces:
            return ""
        char_count = obj.get_character_count()
        if char_count <= 0:
            return ""
        return obj.get_text(0, char_count) or ""
    except Exception:
        return ""


_STATE_MAP = {
    Atspi.StateType.FOCUSED: "focused",
    Atspi.StateType.SELECTED: "selected",
    Atspi.StateType.CHECKED: "checked",
    Atspi.StateType.EXPANDED: "expanded",
    Atspi.StateType.EDITABLE: "editable",
    Atspi.StateType.ENABLED: "enabled",
    Atspi.StateType.FOCUSABLE: "focusable",
    Atspi.StateType.VISIBLE: "visible",
    Atspi.StateType.SHOWING: "showing",
    Atspi.StateType.PRESSED: "pressed",
    Atspi.StateType.REQUIRED: "required",
    Atspi.StateType.INVALID_ENTRY: "invalid",
    Atspi.StateType.READ_ONLY: "readonly",
    Atspi.StateType.MULTI_LINE: "multiline",
    Atspi.StateType.SINGLE_LINE: "singleline",
}


def _get_states(obj: Atspi.Accessible) -> list[str]:
    """Get human-readable state list."""
    state_names = []
    try:
        states = obj.get_state_set()
        for state_type, name in _STATE_MAP.items():
            if states.contains(state_type):
                state_names.append(name)
    except Exception:
        pass
    return state_names


def _get_actions(obj: Atspi.Accessible) -> list[str]:
    """Get available action names."""
    actions = []
    try:
        if "Action" not in obj.get_interfaces():
            return actions
        action_iface = obj.get_action_iface()
        if action_iface is None:
            return actions
        n = action_iface.get_n_actions()
        for i in range(n):
            name = action_iface.get_name(i)
            if name:
                actions.append(name)
    except Exception:
        pass
    return actions


def _get_value(obj: Atspi.Accessible) -> dict | None:
    """Get current/min/max value for value-supporting elements."""
    try:
        if "Value" not in obj.get_interfaces():
            return None
        return {
            "current": obj.get_current_value(),
            "min": obj.get_minimum_value(),
            "max": obj.get_maximum_value(),
            "step": obj.get_minimum_increment(),
        }
    except Exception:
        return None


def _get_relations(obj: Atspi.Accessible) -> list[dict]:
    """Get accessibility relations (labelled-by, described-by, etc.)."""
    relations = []
    try:
        rel_set = obj.get_relation_set()
        for i in range(rel_set.get_length()):
            rel = rel_set.get_relation(i)
            rel_type = rel.get_relation_type()
            targets = []
            for j in range(rel.get_n_targets()):
                target = rel.get_target(j)
                targets.append({
                    "name": target.get_name() or "",
                    "role": _role_name(target.get_role()),
                })
            relations.append({
                "type": str(rel_type).split(".")[-1].lower(),
                "targets": targets,
            })
    except Exception:
        pass
    return relations
