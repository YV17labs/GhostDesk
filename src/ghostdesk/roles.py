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
    "treeitem", "listitem", "cell", "table_row",
})

# All supported role names — every AT-SPI role that ghostdesk can map.
ALL_ROLE_NAMES: frozenset[str] = frozenset({
    "accelerator_label", "alert", "animation", "application", "arrow",
    "article", "audio", "autocomplete", "blockquote", "button", "calendar",
    "canvas", "caption", "cell", "chart", "check_menu_item", "checkbox",
    "color_chooser", "column_header", "combobox", "comment",
    "content_deletion", "content_insertion", "date_editor", "definition",
    "description_list", "desktop_frame", "desktop_icon", "dial", "dialog",
    "directory_pane", "document", "drawing_area", "editbar", "embedded",
    "extended", "file_chooser", "filler", "focus_traversable",
    "font_chooser", "footer", "footnote", "form", "frame", "glass_pane",
    "grouping", "header", "heading", "html_container", "icon", "image",
    "image_map", "info_bar", "input_method_window", "internal_frame",
    "invalid", "label", "landmark", "last_defined", "layered_pane",
    "level_bar", "link", "list", "list_box", "listitem", "log", "mark",
    "marquee", "math", "math_fraction", "math_root", "menu", "menu_bar",
    "menuitem", "notification", "option_pane", "page", "page_tab_list",
    "panel", "paragraph", "password", "popup_menu", "progressbar", "radio",
    "radio_menu_item", "rating", "redundant_object", "root_pane",
    "row_header", "ruler", "scroll_pane", "scrollbar", "section",
    "separator", "slider", "spinbutton", "split_pane", "static",
    "statusbar", "subscript", "suggestion", "superscript", "tab", "table",
    "table_column_header", "table_row", "table_row_header",
    "tearoff_menu_item", "term", "terminal", "text", "textfield", "timer",
    "titlebar", "toggle", "tool_bar", "tooltip", "tree", "tree_table",
    "treeitem", "unknown", "video", "viewport", "window",
})
