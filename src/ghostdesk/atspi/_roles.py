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
        Atspi.Role.TABLE_ROW,
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

# Union of text + interactive roles — everything a screen reader would announce.
_READABLE_ROLES = _TEXT_ROLES | _INTERACTIVE_ROLES

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
    Atspi.Role.ACCELERATOR_LABEL: "accelerator_label",
    Atspi.Role.ANIMATION: "animation",
    Atspi.Role.APPLICATION: "application",
    Atspi.Role.ARROW: "arrow",
    Atspi.Role.AUDIO: "audio",
    Atspi.Role.AUTOCOMPLETE: "autocomplete",
    Atspi.Role.CALENDAR: "calendar",
    Atspi.Role.CANVAS: "canvas",
    Atspi.Role.CHART: "chart",
    Atspi.Role.CHECK_MENU_ITEM: "check_menu_item",
    Atspi.Role.COLOR_CHOOSER: "color_chooser",
    Atspi.Role.COMMENT: "comment",
    Atspi.Role.CONTENT_DELETION: "content_deletion",
    Atspi.Role.CONTENT_INSERTION: "content_insertion",
    Atspi.Role.DATE_EDITOR: "date_editor",
    Atspi.Role.DEFINITION: "definition",
    Atspi.Role.DESCRIPTION_LIST: "description_list",
    Atspi.Role.DESKTOP_FRAME: "desktop_frame",
    Atspi.Role.DESKTOP_ICON: "desktop_icon",
    Atspi.Role.DIAL: "dial",
    Atspi.Role.DIRECTORY_PANE: "directory_pane",
    Atspi.Role.DOCUMENT_EMAIL: "document",
    Atspi.Role.DOCUMENT_PRESENTATION: "document",
    Atspi.Role.DOCUMENT_SPREADSHEET: "document",
    Atspi.Role.DRAWING_AREA: "drawing_area",
    Atspi.Role.EDITBAR: "editbar",
    Atspi.Role.EMBEDDED: "embedded",
    Atspi.Role.EXTENDED: "extended",
    Atspi.Role.FILE_CHOOSER: "file_chooser",
    Atspi.Role.FILLER: "filler",
    Atspi.Role.FOCUS_TRAVERSABLE: "focus_traversable",
    Atspi.Role.FONT_CHOOSER: "font_chooser",
    Atspi.Role.FOOTNOTE: "footnote",
    Atspi.Role.FRAME: "frame",
    Atspi.Role.GLASS_PANE: "glass_pane",
    Atspi.Role.GROUPING: "grouping",
    Atspi.Role.HEADER: "header",
    Atspi.Role.HTML_CONTAINER: "html_container",
    Atspi.Role.ICON: "icon",
    Atspi.Role.IMAGE_MAP: "image_map",
    Atspi.Role.INFO_BAR: "info_bar",
    Atspi.Role.INPUT_METHOD_WINDOW: "input_method_window",
    Atspi.Role.INTERNAL_FRAME: "internal_frame",
    Atspi.Role.INVALID: "invalid",
    Atspi.Role.LAST_DEFINED: "last_defined",
    Atspi.Role.LAYERED_PANE: "layered_pane",
    Atspi.Role.LEVEL_BAR: "level_bar",
    Atspi.Role.LIST: "list",
    Atspi.Role.LIST_BOX: "list_box",
    Atspi.Role.LOG: "log",
    Atspi.Role.MARK: "mark",
    Atspi.Role.MARQUEE: "marquee",
    Atspi.Role.MATH: "math",
    Atspi.Role.MATH_FRACTION: "math_fraction",
    Atspi.Role.MATH_ROOT: "math_root",
    Atspi.Role.MENU_BAR: "menu_bar",
    Atspi.Role.OPTION_PANE: "option_pane",
    Atspi.Role.PAGE: "page",
    Atspi.Role.PAGE_TAB_LIST: "page_tab_list",
    Atspi.Role.PANEL: "panel",
    Atspi.Role.POPUP_MENU: "popup_menu",
    Atspi.Role.RADIO_MENU_ITEM: "radio_menu_item",
    Atspi.Role.RATING: "rating",
    Atspi.Role.REDUNDANT_OBJECT: "redundant_object",
    Atspi.Role.ROOT_PANE: "root_pane",
    Atspi.Role.RULER: "ruler",
    Atspi.Role.SCROLL_PANE: "scroll_pane",
    Atspi.Role.SPLIT_PANE: "split_pane",
    Atspi.Role.SUBSCRIPT: "subscript",
    Atspi.Role.SUGGESTION: "suggestion",
    Atspi.Role.SUPERSCRIPT: "superscript",
    Atspi.Role.TABLE_COLUMN_HEADER: "table_column_header",
    Atspi.Role.TABLE_ROW_HEADER: "table_row_header",
    Atspi.Role.TEAROFF_MENU_ITEM: "tearoff_menu_item",
    Atspi.Role.TERMINAL: "terminal",
    Atspi.Role.TIMER: "timer",
    Atspi.Role.TOOL_BAR: "tool_bar",
    Atspi.Role.TREE: "tree",
    Atspi.Role.TREE_TABLE: "tree_table",
    Atspi.Role.UNKNOWN: "unknown",
    Atspi.Role.VIDEO: "video",
    Atspi.Role.VIEWPORT: "viewport",
    Atspi.Role.WINDOW: "window",
}


def _role_name(role: Atspi.Role) -> str:
    name = _ROLE_NAMES.get(role)
    if name is not None:
        return name
    try:
        return role.value_nick.replace("-", "_")
    except Exception:
        return str(role).split(".")[-1].lower()


# Assert at load time that role name sets stay in sync with the
# canonical definitions in ghostdesk.roles (the single source of truth).
from ghostdesk.roles import INTERACTIVE_ROLE_NAMES as _CANONICAL_ROLES  # noqa: E402
from ghostdesk.roles import ALL_ROLE_NAMES as _CANONICAL_ALL_ROLES  # noqa: E402


def _assert_sync(actual: frozenset[str], canonical: frozenset[str], label: str) -> None:
    if actual != canonical:
        missing = canonical - actual
        extra = actual - canonical
        raise RuntimeError(
            f"atspi/_roles.py {label} out of sync with ghostdesk/roles.py. "
            f"Missing: {missing or 'none'}, Extra: {extra or 'none'}"
        )


_assert_sync(frozenset(_ROLE_NAMES[r] for r in _INTERACTIVE_ROLES), _CANONICAL_ROLES, "INTERACTIVE_ROLE_NAMES")
_assert_sync(frozenset(_ROLE_NAMES.values()), _CANONICAL_ALL_ROLES, "ALL_ROLE_NAMES")
