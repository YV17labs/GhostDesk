# Copyright (c) 2026 YV17 — MIT License
"""Canonical role names — single source of truth.

This module has zero external dependencies so it can be imported by both
the main venv code (tools/accessibility/_client.py) and the system-Python
AT-SPI package (atspi/_roles.py).

Any change here must be reflected in atspi/_roles.py's Atspi.Role mappings.
"""

# Interactive roles — elements an LLM agent would click, type into, or manipulate.
INTERACTIVE_ROLE_NAMES: frozenset[str] = frozenset({
    "button", "toggle", "checkbox", "radio", "combobox", "menu", "menuitem",
    "link", "textfield", "password", "text", "spinbutton", "slider", "tab",
    "treeitem", "listitem", "cell",
})
