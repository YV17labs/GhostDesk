# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.roles — canonical role names."""

import pytest

from ghostdesk.roles import ALL_ROLE_NAMES, INTERACTIVE_ROLE_NAMES


# --- INTERACTIVE_ROLE_NAMES ---


def test_interactive_role_names_is_frozenset():
    """INTERACTIVE_ROLE_NAMES is a frozenset (immutable)."""
    assert isinstance(INTERACTIVE_ROLE_NAMES, frozenset)


@pytest.mark.parametrize("role", [
    "button", "toggle", "checkbox", "radio", "combobox",
    "menu", "menuitem", "link", "textfield", "password",
    "text", "spinbutton", "slider", "tab", "treeitem",
    "listitem", "cell",
])
def test_contains_interactive_role(role):
    """INTERACTIVE_ROLE_NAMES contains all expected interactive roles."""
    assert role in INTERACTIVE_ROLE_NAMES


@pytest.mark.parametrize("role", [
    "heading", "paragraph", "separator", "image", "toolbar",
    "statusbar", "dialog", "window", "frame",
])
def test_does_not_contain_non_interactive_role(role):
    """INTERACTIVE_ROLE_NAMES does NOT contain non-interactive roles."""
    assert role not in INTERACTIVE_ROLE_NAMES


def test_interactive_role_names_count():
    """INTERACTIVE_ROLE_NAMES contains exactly 17 roles."""
    assert len(INTERACTIVE_ROLE_NAMES) == 17


def test_interactive_role_names_is_immutable():
    """INTERACTIVE_ROLE_NAMES cannot be modified."""
    with pytest.raises(AttributeError):
        INTERACTIVE_ROLE_NAMES.add("newrole")


# --- ALL_ROLE_NAMES ---


def test_all_role_names_is_frozenset():
    """ALL_ROLE_NAMES is a frozenset (immutable)."""
    assert isinstance(ALL_ROLE_NAMES, frozenset)


def test_all_role_names_superset_of_interactive():
    """ALL_ROLE_NAMES contains every interactive role."""
    assert INTERACTIVE_ROLE_NAMES.issubset(ALL_ROLE_NAMES)


@pytest.mark.parametrize("role", [
    "window", "frame", "terminal", "tree", "panel", "video",
    "section", "dialog", "calendar", "canvas", "application",
])
def test_all_role_names_contains_non_interactive(role):
    """ALL_ROLE_NAMES includes non-interactive AT-SPI roles."""
    assert role in ALL_ROLE_NAMES


def test_all_role_names_is_immutable():
    """ALL_ROLE_NAMES cannot be modified."""
    with pytest.raises(AttributeError):
        ALL_ROLE_NAMES.add("newrole")
