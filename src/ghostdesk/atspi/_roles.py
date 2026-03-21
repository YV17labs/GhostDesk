# Copyright (c) 2026 YV17 — MIT License
"""Role mappings for AT-SPI accessible objects."""

from __future__ import annotations

from gi.repository import Atspi  # type: ignore[attr-defined]

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

# Roles that carry text content (for read_screen).
_TEXT_ROLES = frozenset(
    {
        Atspi.Role.HEADING,
        Atspi.Role.PARAGRAPH,
        Atspi.Role.LABEL,
        Atspi.Role.STATIC,
        Atspi.Role.TEXT,
        Atspi.Role.LINK,
        Atspi.Role.LIST_ITEM,
        Atspi.Role.ARTICLE,
        Atspi.Role.BLOCK_QUOTE,
        Atspi.Role.CAPTION,
        Atspi.Role.COMMENT,
        Atspi.Role.SECTION,
        Atspi.Role.ENTRY,
        Atspi.Role.TABLE_CELL,
        Atspi.Role.COLUMN_HEADER,
        Atspi.Role.ROW_HEADER,
        Atspi.Role.DESCRIPTION_TERM,
        Atspi.Role.DESCRIPTION_VALUE,
        Atspi.Role.FOOTER,
        Atspi.Role.MARK,
        Atspi.Role.CONTENT_DELETION,
        Atspi.Role.CONTENT_INSERTION,
        Atspi.Role.ALERT,
        Atspi.Role.NOTIFICATION,
        Atspi.Role.STATUS_BAR,
        Atspi.Role.TITLE_BAR,
        Atspi.Role.TOOL_TIP,
        Atspi.Role.DOCUMENT_WEB,
        Atspi.Role.DOCUMENT_TEXT,
        Atspi.Role.DOCUMENT_FRAME,
    }
)

# Human-readable role names for the JSON output.
_ROLE_NAMES = {
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
    Atspi.Role.HEADING: "heading",
    Atspi.Role.PARAGRAPH: "paragraph",
    Atspi.Role.LABEL: "label",
    Atspi.Role.STATIC: "static",
    Atspi.Role.ARTICLE: "article",
    Atspi.Role.BLOCK_QUOTE: "blockquote",
    Atspi.Role.CAPTION: "caption",
    Atspi.Role.SECTION: "section",
    Atspi.Role.COLUMN_HEADER: "column_header",
    Atspi.Role.ROW_HEADER: "row_header",
    Atspi.Role.TABLE: "table",
    Atspi.Role.TABLE_ROW: "table_row",
    Atspi.Role.DESCRIPTION_TERM: "term",
    Atspi.Role.DESCRIPTION_VALUE: "definition",
    Atspi.Role.FOOTER: "footer",
    Atspi.Role.ALERT: "alert",
    Atspi.Role.NOTIFICATION: "notification",
    Atspi.Role.STATUS_BAR: "statusbar",
    Atspi.Role.TITLE_BAR: "titlebar",
    Atspi.Role.TOOL_TIP: "tooltip",
    Atspi.Role.IMAGE: "image",
    Atspi.Role.DIALOG: "dialog",
    Atspi.Role.FORM: "form",
    Atspi.Role.LANDMARK: "landmark",
    Atspi.Role.SEPARATOR: "separator",
    Atspi.Role.PROGRESS_BAR: "progressbar",
    Atspi.Role.SCROLL_BAR: "scrollbar",
    Atspi.Role.DOCUMENT_WEB: "document",
    Atspi.Role.DOCUMENT_TEXT: "document",
    Atspi.Role.DOCUMENT_FRAME: "document",
}


def _role_name(role: Atspi.Role) -> str:
    if role in _ROLE_NAMES:
        return _ROLE_NAMES[role]
    try:
        return role.value_nick.replace("-", "_")
    except Exception:
        return str(role).split(".")[-1].lower()


# The human-readable names for interactive roles must stay in sync with
# ghostdesk.roles.INTERACTIVE_ROLE_NAMES (the single source of truth).
# Import it here and assert at load time so a mismatch is caught immediately.
from ghostdesk.roles import INTERACTIVE_ROLE_NAMES as _CANONICAL_ROLES  # noqa: E402

_interactive_names = frozenset(_ROLE_NAMES[r] for r in _INTERACTIVE_ROLES)
if _interactive_names != _CANONICAL_ROLES:
    _missing = _CANONICAL_ROLES - _interactive_names
    _extra = _interactive_names - _CANONICAL_ROLES
    raise RuntimeError(
        f"atspi/_roles.py out of sync with ghostdesk/roles.py. "
        f"Missing: {_missing or 'none'}, Extra: {_extra or 'none'}"
    )
